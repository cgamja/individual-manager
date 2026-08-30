//! 펫 브릿지 — 바탕화면 펭귄 창의 생성과 수명 관리.
//!
//! 창 플래그는 전부 여기 한 곳에서 정한다. 창 레벨을 "항상 위"에서 "데스크톱 뒤"로
//! 뒤집고 싶어지면 고칠 곳도 여기 하나다 (KTD3).

use std::sync::Mutex;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::pet::{Bounds, Pet, Snapshot};
use crate::timer_bridge::now_ms;

/// 웹뷰가 구독하는 상태 이벤트.
pub const EVENT_PET_STATE: &str = "pet://state";

/// 위치·동작 갱신 주기. 스프라이트 프레임은 CSS가 담당하므로 이 주기는
/// "얼마나 부드럽게 이동하느냐"만 정한다. set_position은 매 호출이 IPC라
/// 60Hz로 때리지 않는다 (KTD2).
const TICK_MS: u64 = 50;
/// 졸고 있을 때의 틱 간격 — 깨어날 시각만 확인하면 되므로 길게 잡는다 (R10).
const SLEEP_TICK_MS: u64 = 500;

pub struct PetState(pub Mutex<Pet>);

/// 펫 창 라벨. `capabilities/default.json`의 `windows`에도 같은 값이 들어 있어야
/// 이벤트가 전달된다 — 빠뜨리면 조용히 아무것도 오지 않는다 (KTD8).
pub const PET_LABEL: &str = "pet";

/// 펭귄 창의 한 변 (논리 px). 스프라이트 바운딩 박스에 맞춘 크기다 —
/// 창을 좁게 유지하는 것이 클릭 통과를 대신하는 전략이다 (KTD3).
pub const PET_SIZE: f64 = 140.0;

pub fn pet_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(PET_LABEL)
}

/// 펫 창을 만든다. 이미 있으면 그것을 돌려준다 (중복 생성 방지).
pub fn create_pet_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = pet_window(app) {
        return Ok(existing);
    }

    WebviewWindowBuilder::new(app, PET_LABEL, WebviewUrl::App("pet.html".into()))
        .title("Penguin Pet")
        .inner_size(PET_SIZE, PET_SIZE)
        .position(120.0, 120.0)
        // 투명 창 — app.macOSPrivateApi(이미 true) + pet.css의 배경 투명이 함께 필요하다
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        // 모든 앱 창 위에 보인다 (A1). Tauri의 alwaysOnBottom은 NSWindow 레벨 -1이라
        // 데스크톱 레벨이 아니므로 "창 뒤"를 원하면 ns_window()를 직접 만져야 한다
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        // 첫 클릭이 앱 활성화에 먹히지 않게 한다 — 없으면 펭귄을 두 번 눌러야 반응한다
        .accept_first_mouse(true)
        // 키보드 포커스를 뺏지 않는다 (R9)
        .focused(false)
        .visible(true)
        .build()
        .inspect(|window| {
            // Accessory 정책 아래에서는 빌더의 visible만으로 화면에 나오지 않는 경우가 있다
            // (tauri#5122 계열) — 명시적으로 한 번 더 띄운다. 포커스는 주지 않는다.
            let _ = window.show();
        })
}

/// 펫 창을 닫는다. 없으면 아무것도 하지 않는다.
pub fn close_pet_window(app: &AppHandle) {
    if let Some(window) = pet_window(app) {
        let _ = window.close();
    }
}

/// 모니터의 작업 영역(물리 px)을 펭귄이 걸어다닐 논리 좌표 영역으로 바꾼다.
///
/// 창 위치는 좌상단 기준이라 오른쪽·아래 한계에서 창 크기를 빼야 화면 밖으로
/// 나가지 않는다. 이 계산을 순수 함수로 떼어 낸 이유는 배율 2.0(Retina)에서
/// 어긋나기 쉬운 부분이라 테스트로 고정하기 위해서다.
pub fn bounds_from_work_area(
    origin: (i32, i32),
    size: (u32, u32),
    scale: f64,
    pet_size: f64,
) -> Bounds {
    let left = f64::from(origin.0) / scale;
    let top = f64::from(origin.1) / scale;
    let width = f64::from(size.0) / scale;
    let height = f64::from(size.1) / scale;
    Bounds {
        left,
        // 영역이 펭귄보다 좁은 극단적 경우에도 right < left가 되지 않게 한다
        right: (left + width - pet_size).max(left),
        floor_y: (top + height - pet_size).max(top),
    }
}

/// 지금 펭귄이 놓인 모니터의 이동 영역. 모니터를 못 읽으면 이전 경계를 쓰도록
/// `None`을 돌려준다 — 임의의 기본값으로 펭귄을 순간이동시키지 않는다.
fn current_bounds(window: &WebviewWindow) -> Option<Bounds> {
    let monitor = window.current_monitor().ok().flatten()?;
    let area = monitor.work_area();
    Some(bounds_from_work_area(
        (area.position.x, area.position.y),
        (area.size.width, area.size.height),
        monitor.scale_factor(),
        PET_SIZE,
    ))
}

/// 위치·동작 틱. 트레이(`set_title`)와 달리 `set_position`은 어느 스레드에서
/// 불러도 안전하다 — tauri-runtime-wry가 메인 스레드가 아니면 이벤트 루프로
/// 넘기고, tao의 macOS 구현이 다시 메인 스레드로 디스패치한다 (KTD5).
/// 그러니 여기서는 `run_on_main_thread`로 감싸지 않는다.
pub fn spawn_pet_tick_thread(app: AppHandle) {
    std::thread::spawn(move || {
        let mut bounds: Option<Bounds> = None;
        loop {
            let Some(window) = pet_window(&app) else {
                // 설정에서 껐거나 아직 만들기 전 — 창이 생길 때까지 느리게 돈다
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            };
            bounds = current_bounds(&window).or(bounds);
            let Some(area) = bounds else {
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            };

            let snapshot = {
                let state = app.state::<PetState>();
                let mut pet = state.0.lock().unwrap();
                pet.step(now_ms(), area)
            };

            let interval = if snapshot.behavior.moves_window() {
                apply(&window, snapshot);
                TICK_MS
            } else {
                // 자는 동안에는 창을 옮기지도, 이벤트를 쏘지도 않는다 (R10)
                SLEEP_TICK_MS
            };
            std::thread::sleep(Duration::from_millis(interval));
        }
    });
}

/// 스냅샷을 창 위치와 웹뷰 상태에 반영한다.
fn apply(window: &WebviewWindow, snapshot: Snapshot) {
    let _ = window.set_position(LogicalPosition::new(snapshot.x, snapshot.y));
    let _ = window.emit(EVENT_PET_STATE, snapshot);
}

/// 커맨드가 상태를 바꾼 뒤 즉시 화면에 반영한다 — 다음 틱(최대 500ms)을
/// 기다리면 클릭·드래그 반응이 굼떠 보인다.
fn flush(app: &AppHandle) -> Option<Snapshot> {
    let window = pet_window(app)?;
    let snapshot = app.state::<PetState>().0.lock().unwrap().snapshot();
    apply(&window, snapshot);
    Some(snapshot)
}

/// 펭귄 클릭 — 놀라게 하고 팝오버를 연다 (R5).
#[tauri::command]
pub fn pet_poke(state: State<'_, PetState>, app: AppHandle) {
    state.0.lock().unwrap().poke(now_ms());
    flush(&app);
    crate::toggle_popover(&app);
}

/// 드래그 시작 — 자율 이동을 멈춘다 (R6).
#[tauri::command]
pub fn pet_drag_start(state: State<'_, PetState>, app: AppHandle) {
    state.0.lock().unwrap().drag_start(now_ms());
    flush(&app);
}

/// 드래그 이동량(논리 px). 창 위치의 소유자는 Rust 하나뿐이라 웹뷰는
/// 이동량만 보내고 직접 `setPosition`을 부르지 않는다 (KTD4).
#[tauri::command]
pub fn pet_drag_by(dx: f64, dy: f64, state: State<'_, PetState>, app: AppHandle) {
    state.0.lock().unwrap().drag_by(dx, dy);
    flush(&app);
}

/// 드래그 놓기 — 떨어뜨린다 (R6).
#[tauri::command]
pub fn pet_drag_end(state: State<'_, PetState>, app: AppHandle) {
    state.0.lock().unwrap().drag_end(now_ms());
    flush(&app);
}

/// 웹뷰가 처음 뜰 때 현재 상태를 한 번 받아 간다 (첫 틱을 기다리지 않게).
#[tauri::command]
pub fn pet_get_state(state: State<'_, PetState>) -> Snapshot {
    let snapshot = state.0.lock().unwrap().snapshot();
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 작업_영역을_논리_좌표_경계로_변환한다() {
        // 배율 1.0, 메뉴바 25px를 뺀 1440x875 영역
        let b = bounds_from_work_area((0, 25), (1440, 875), 1.0, 140.0);
        assert_eq!(b.left, 0.0);
        assert_eq!(b.right, 1440.0 - 140.0);
        assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
    }

    #[test]
    fn 레티나_배율에서도_논리_좌표로_환산한다() {
        // 물리 2880x1750 = 논리 1440x875 (배율 2.0)
        let b = bounds_from_work_area((0, 50), (2880, 1750), 2.0, 140.0);
        assert_eq!(b.left, 0.0);
        assert_eq!(b.right, 1440.0 - 140.0);
        assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
    }

    #[test]
    fn 보조_모니터처럼_원점이_음수여도_경계가_밀린다() {
        let b = bounds_from_work_area((-1920, 0), (1920, 1080), 1.0, 140.0);
        assert_eq!(b.left, -1920.0);
        assert_eq!(b.right, -1920.0 + 1920.0 - 140.0);
    }

    #[test]
    fn 영역이_펭귄보다_좁아도_경계가_뒤집히지_않는다() {
        let b = bounds_from_work_area((0, 0), (100, 100), 1.0, 140.0);
        assert!(b.right >= b.left, "right가 left보다 작아지면 clamp가 패닉한다");
        assert!(b.floor_y >= 0.0);
    }
}
