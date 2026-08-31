//! 펫 브릿지 — 바탕화면 펭귄 창의 생성과 수명 관리.
//!
//! 창 플래그는 전부 여기 한 곳에서 정한다. 창 레벨을 "항상 위"에서 "데스크톱 뒤"로
//! 뒤집고 싶어지면 고칠 곳도 여기 하나다 (KTD3).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

use crate::pet::{Behavior, Bounds, Facing, PetId, Pets, Snapshot, Vertical, MAX_PETS};
use crate::timer_bridge::now_ms;

/// 웹뷰가 구독하는 상태 이벤트.
pub const EVENT_PET_STATE: &str = "pet://state";

/// 위치·동작 갱신 주기. 스프라이트 프레임은 CSS가 담당하므로 이 주기는
/// "얼마나 부드럽게 이동하느냐"만 정한다. set_position은 매 호출이 IPC라
/// 60Hz로 때리지 않는다 (KTD2).
const TICK_MS: u64 = 50;
/// 졸고 있을 때의 틱 간격 — 깨어날 시각만 확인하면 되므로 길게 잡는다 (R10).
const SLEEP_TICK_MS: u64 = 500;
/// 모니터 작업 영역을 다시 읽는 주기.
const BOUNDS_REFRESH_MS: u64 = 2_000;
/// 상태와 창이 어긋난 것을 보고 **정리하기까지 기다리는 시간**.
///
/// 펭귄을 추가할 때는 (1) 상태에 넣고 (2) 창을 만든다. 그 사이 몇 ms 동안은
/// "상태엔 있는데 창이 없는" 정상적인 어긋남이 생긴다. 한 번 보고 바로 지우면
/// **방금 부른 펭귄을 곧바로 지워 버린다** — 추가가 아무 일도 안 하는 것처럼 보인다.
/// 삭제도 (1) 상태에서 빼고 (2) 창을 닫는 순서라 같은 창이 열린다.
const RECONCILE_GRACE_MS: u64 = 1_000;

/// 어긋남을 처음 본 시각들 중, 유예를 다 쓴 것들.
///
/// 순수 함수로 떼어 낸 이유는 이 판단이 틀리면 **정상 동작이 조용히 취소되기** 때문이다.
/// 틱 스레드 안에 두면 눈으로만 잡힌다.
fn due_for_cleanup(mismatch_since: &HashMap<PetId, u64>, now_ms: u64) -> Vec<PetId> {
    mismatch_since
        .iter()
        .filter(|(_, since)| now_ms.saturating_sub(**since) >= RECONCILE_GRACE_MS)
        .map(|(id, _)| *id)
        .collect()
}

pub struct PetState {
    pub pets: Mutex<Pets>,
    /// 마지막으로 우클릭된 펭귄. 팝오버(`main` 창)는 자기가 **어느 펭귄 때문에**
    /// 열렸는지 모르므로, 삭제 대상을 알려면 여는 쪽이 남겨 줘야 한다 (KTD6).
    pub focused: Mutex<Option<PetId>>,
}

impl PetState {
    pub fn new(pets: Pets) -> Self {
        PetState {
            pets: Mutex::new(pets),
            focused: Mutex::new(None),
        }
    }
}

/// 팝오버가 버튼 상태를 정하는 데 쓰는 요약. 누를 수 없는 버튼은 **비활성**으로
/// 보여야 한다 — 눌리는데 아무 일도 없으면 고장으로 읽힌다.
#[derive(Serialize)]
pub struct PetSummary {
    pub count: usize,
    pub max: usize,
    pub focused: Option<PetId>,
}

/// 웹뷰가 보는 "겉모습" — 이게 바뀔 때만 상태를 다시 알린다.
/// **CSS 클래스에 영향을 주는 값은 빠짐없이 들어가야 한다.** 하나라도 빠지면
/// 그 값만 바뀌는 전이가 웹뷰에 영영 도달하지 않는다(조용한 실패).
pub type Look = (Behavior, Facing, Vertical, bool, Option<u64>, u64);

pub fn look_of(snapshot: &Snapshot) -> Look {
    (
        snapshot.behavior,
        snapshot.facing,
        snapshot.vertical,
        snapshot.air,
        // 말풍선과 빠따는 동작을 바꾸지 않고도 화면을 바꾼다 — 빠뜨리면
        // 말이 안 뜨거나 방망이가 한 번만 보인다
        snapshot.speech.map(|s| s.seq),
        snapshot.whack_seq,
    )
}

/// 겉모습이 달라졌는가. 매 틱 emit하면 웹뷰가 이유 없이 20Hz로 리렌더한다.
pub fn should_notify(last: Option<Look>, now: Look) -> bool {
    last != Some(now)
}

/// 프론트의 `settings.ts`와 공유하는 저장 위치. 웹뷰가 저장하고 Rust가 읽는다 —
/// 시작 시점에 펭귄을 띄울지는 웹뷰가 뜨기 전에 정해져야 깜빡임이 없다.
const SETTINGS_FILE: &str = "settings.json";
const PET_KEY: &str = "pet";

/// 저장된 켜짐 여부. 값이 없으면 켜짐이 기본이다 — 사용자가 직접 요청한
/// 기능이라 opt-in으로 숨기지 않는다 (A2).
pub fn pet_enabled(app: &AppHandle) -> bool {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|store| store.get(PET_KEY))
        .and_then(|value| value.get("enabled").and_then(|v| v.as_bool()))
        .unwrap_or(true)
}

/// 저장된 마릿수. 없으면 한 마리, 범위를 벗어나면 조인다 —
/// 저장 파일이 손으로 고쳐져도 0마리나 100마리로 뜨지 않게.
pub fn pet_count(app: &AppHandle) -> usize {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|store| store.get(PET_KEY))
        .and_then(|value| value.get("count").and_then(|v| v.as_u64()))
        .map_or(1, |n| (n as usize).clamp(1, MAX_PETS))
}

/// 마릿수를 저장한다. **읽고-고쳐-쓰기**여야 한다 — `pet` 키 아래 `enabled`가
/// 함께 살아서, 객체를 통째로 덮어쓰면 켜짐/꺼짐 설정이 날아간다 (KTD3).
fn save_pet_count(app: &AppHandle, count: usize) {
    let Ok(store) = app.store(SETTINGS_FILE) else {
        return;
    };
    let mut value = store
        .get(PET_KEY)
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        obj.insert("count".into(), serde_json::json!(count));
        store.set(PET_KEY, value);
        let _ = store.save();
    }
}

/// 저장된 마릿수만큼 펭귄을 만든다. 이미 있으면 모자란 만큼만 채운다.
pub fn spawn_saved_pets(app: &AppHandle) -> tauri::Result<()> {
    let wanted = pet_count(app);
    let now = now_ms();
    let bounds = bounds_or_flat_any(app);
    while app.state::<PetState>().pets.lock().unwrap().len() < wanted {
        // 첫 마리는 왼쪽 끝, 그다음부터는 앞 마리 옆에 세운다 — 전부 같은 자리에
        // 겹쳐 뜨면 한 마리로 보인다
        let start_x = {
            let state = app.state::<PetState>();
            let pets = state.pets.lock().unwrap();
            pets.ids()
                .last()
                .and_then(|id| pets.get(*id))
                .map_or(bounds.left, |p| next_to(p.snapshot().x, bounds))
        };
        let Some(id) = app
            .state::<PetState>()
            .pets
            .lock()
            .unwrap()
            .add(now, now, bounds, start_x)
        else {
            break;
        };
        if let Err(err) = create_pet_window(app, id, window_origin(start_x, bounds.floor_y)) {
            app.state::<PetState>().pets.lock().unwrap().forget(id);
            return Err(err);
        }
    }
    Ok(())
}

/// 펫 창 라벨 접두어. 마리마다 `pet-<id>`가 된다.
///
/// `capabilities/default.json`의 `windows`에 **`pet-*` 글롭**이 들어 있어야 이벤트와
/// 커맨드가 전달된다 — 빠뜨리면 컴파일·테스트는 전부 통과하고 런타임에서만 조용히
/// reject된다 (KTD8, `docs/solutions/best-practices/tauri-command-registration-silent-failure.md`).
pub const PET_LABEL_PREFIX: &str = "pet-";

/// 펭귄 id → 창 라벨.
pub fn pet_label(id: PetId) -> String {
    format!("{PET_LABEL_PREFIX}{id}")
}

/// 창 라벨 → 펭귄 id. 펫 창이 아니면 `None`.
///
/// 커맨드가 "누가 불렀는가"를 이걸로 정한다 — 웹뷰가 id를 인자로 보내면 틀린 id를
/// 보낼 수도, 남의 펭귄을 조작할 수도 있다. 라벨은 위조할 수 없다 (KTD1).
pub fn pet_id_from_label(label: &str) -> Option<PetId> {
    label.strip_prefix(PET_LABEL_PREFIX)?.parse().ok()
}

/// 펭귄 자체가 차지하는 한 변 (논리 px). 이동 경계는 이 값으로 계산한다.
pub const PET_SIZE: f64 = 140.0;
/// 펭귄 좌우로 비워 두는 여백 — 방망이가 휘둘러질 자리.
pub const PET_PAD_X: f64 = 52.0;
/// 펭귄 위로 비워 두는 여백 — 말풍선이 뜰 자리.
pub const PET_PAD_TOP: f64 = 80.0;

/// 창은 펭귄보다 크다. 창을 펭귄 크기에 딱 맞추면 말풍선과 방망이가 잘린다.
///
/// **여백도 클릭을 먹는다.** 투명하다고 통과되지 않는다 — macOS에서는 투명한
/// 창 영역도 그 창이 히트 테스트를 가져가고, CSS `pointer-events`로는 다른 앱에
/// 넘길 수 없다(KTD3에서 클릭 통과를 안 쓰기로 했다). 그래서 여백은 말풍선과
/// 방망이에 꼭 필요한 만큼만 둔다.
pub const PET_WINDOW_W: f64 = PET_SIZE + PET_PAD_X * 2.0;
pub const PET_WINDOW_H: f64 = PET_SIZE + PET_PAD_TOP;

/// 펭귄 좌표 → 창 좌표. 펭귄이 창 안에서 (PAD_X, PAD_TOP)에 놓이므로
/// 창은 그만큼 왼쪽·위로 물러나 있어야 한다.
pub fn window_origin(pet_x: f64, pet_y: f64) -> (f64, f64) {
    (pet_x - PET_PAD_X, pet_y - PET_PAD_TOP)
}

pub fn pet_window(app: &AppHandle, id: PetId) -> Option<WebviewWindow> {
    app.get_webview_window(&pet_label(id))
}

/// 살아 있는 아무 펫 창 하나. 화면 경계처럼 "어느 펭귄이든 상관없는" 조회에 쓴다.
fn any_pet_window(app: &AppHandle) -> Option<WebviewWindow> {
    let ids = app.state::<PetState>().pets.lock().unwrap().ids();
    ids.into_iter().find_map(|id| pet_window(app, id))
}

/// 펫 창을 만든다. 이미 있으면 그것을 돌려준다 (중복 생성 방지).
pub fn create_pet_window(app: &AppHandle, id: PetId, at: (f64, f64)) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = pet_window(app, id) {
        return Ok(existing);
    }

    WebviewWindowBuilder::new(app, pet_label(id), WebviewUrl::App("pet.html".into()))
        .title("Penguin Pet")
        .inner_size(PET_WINDOW_W, PET_WINDOW_H)
        .position(at.0, at.1)
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
        // 다른 Space로 가리거나 가려져도 CSS 애니메이션이 스로틀되지 않게 한다
        // (솔루션 문서 함정 1의 보조 대응. macOS 14+)
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()
        .inspect(|window| {
            // Accessory 정책 아래에서는 빌더의 visible만으로 화면에 나오지 않는 경우가 있다
            // (tauri#5122 계열) — 명시적으로 한 번 더 띄운다. 포커스는 주지 않는다.
            let _ = window.show();
        })
}

/// 펫 창 하나를 닫는다. 없으면 아무것도 하지 않는다.
pub fn close_pet_window(app: &AppHandle, id: PetId) {
    if let Some(window) = pet_window(app, id) {
        let _ = window.close();
    }
}

/// 모든 펫 창을 닫는다 (설정에서 껐을 때).
pub fn close_all_pet_windows(app: &AppHandle) {
    let ids = app.state::<PetState>().pets.lock().unwrap().ids();
    for id in ids {
        close_pet_window(app, id);
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
    // 경계는 **창 전체**가 화면 안에 들어오도록 잡는다. 펭귄 크기만 빼면
    // 창의 여백(말풍선·방망이)이 화면 밖으로 나가, 정작 벽에 붙었을 때
    // 방망이가 안 보이고 위쪽에서는 말풍선이 메뉴바 뒤로 숨는다.
    let min_x = left + PET_PAD_X;
    let max_x = left + width - pet_size - PET_PAD_X;
    let min_y = top + PET_PAD_TOP;
    let max_y = top + height - pet_size;
    Bounds {
        left: min_x,
        // 영역이 창보다 좁은 극단적 경우에도 right < left가 되지 않게 한다
        right: max_x.max(min_x),
        top: min_y.min(max_y),
        floor_y: max_y.max(min_y),
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
        // 경계와 겉모습은 **마리별**이다. 두 펭귄이 다른 모니터에 있을 수 있고,
        // 한 마리가 자는 동안 다른 마리는 걷는다.
        let mut bounds: HashMap<PetId, (Bounds, u64)> = HashMap::new();
        let mut last_look: HashMap<PetId, Look> = HashMap::new();
        // 상태와 창이 어긋난 것을 **처음 본** 시각. 곧바로 정리하지 않는 이유는
        // 추가·삭제가 두 단계라 정상적으로도 잠깐 어긋나기 때문이다.
        let mut mismatch_since: HashMap<PetId, u64> = HashMap::new();
        loop {
            let ids = app.state::<PetState>().pets.lock().unwrap().ids();
            let now = now_ms();

            // 상태에 없는데 창만 남은 펭귄 — 삭제할 때 `close()`가 실패하면 아무도
            // 움직이지 않는 **얼어붙은 펭귄**이 화면에 영영 남고, 상태에 없으니 다시
            // 지울 수도 없다. 사용자가 겪은 "펭귄이 두 마리" 그 모양이다.
            let mut orphan_windows: HashMap<PetId, WebviewWindow> = HashMap::new();
            for (label, window) in app.webview_windows() {
                if let Some(orphan) = pet_id_from_label(&label) {
                    if !ids.contains(&orphan) {
                        orphan_windows.insert(orphan, window);
                    }
                }
            }

            // 이번 틱에 어긋난 것들을 표시하고, 맞는 것들은 표시를 지운다
            let mut mismatched: Vec<PetId> = orphan_windows.keys().copied().collect();
            for id in &ids {
                if pet_window(&app, *id).is_none() {
                    mismatched.push(*id);
                }
            }
            mismatch_since.retain(|id, _| mismatched.contains(id));
            for id in &mismatched {
                mismatch_since.entry(*id).or_insert(now);
            }

            // 유예를 다 쓴 것만 정리한다
            for id in due_for_cleanup(&mismatch_since, now) {
                if let Some(window) = orphan_windows.get(&id) {
                    let _ = window.close();
                } else {
                    // 창이 사라졌다 — 사용자의 선택이 아니라 이미 없어진 것이므로
                    // 마지막 한 마리 보호를 받지 않고 정리한다
                    app.state::<PetState>().pets.lock().unwrap().forget(id);
                }
                mismatch_since.remove(&id);
                bounds.remove(&id);
                last_look.remove(&id);
            }

            if ids.is_empty() {
                // 설정에서 껐거나 아직 만들기 전 — 생길 때까지 느리게 돈다
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            }

            // 한 마리라도 움직이면 빠른 주기를 유지한다. 자는 마리 때문에 걷는
            // 마리가 느려지면 안 된다.
            let mut any_moves = false;

            for id in ids {
                let Some(window) = pet_window(&app, id) else {
                    // 아직 만들어지는 중일 수 있다 — 위의 유예가 판단한다
                    continue;
                };

                // current_monitor()는 이벤트 루프를 왕복하는 블로킹 호출이라 20Hz로
                // 부르면 상주 비용이 아깝다. 모니터·해상도는 자주 바뀌지 않으므로
                // 2초에 한 번만 다시 읽는다 — 마리가 늘수록 이 캐시가 더 중요해진다.
                let stale = bounds
                    .get(&id)
                    .is_none_or(|(_, at)| now.saturating_sub(*at) >= BOUNDS_REFRESH_MS);
                if stale {
                    if let Some(area) = current_bounds(&window) {
                        bounds.insert(id, (area, now));
                    }
                }
                let Some((area, _)) = bounds.get(&id).copied() else {
                    continue;
                };

                let snapshot = {
                    let state = app.state::<PetState>();
                    let mut pets = state.pets.lock().unwrap();
                    let Some(pet) = pets.get_mut(id) else {
                        continue;
                    };
                    pet.step(now, area)
                };

                let moves = snapshot.behavior.moves_window();
                any_moves |= moves;
                let look = look_of(&snapshot);
                // 졸기로 "전이하는" 그 스냅샷은 반드시 알려야 한다. 움직임 여부로만
                // 거르면 자는 모습이 웹뷰에 영영 도달하지 않아 직전 동작이 그대로 남는다.
                apply(&window, snapshot, moves, should_notify(last_look.get(&id).copied(), look));
                last_look.insert(id, look);
            }

            // 자는 동안에는 창을 옮기지 않고 틱도 느려진다 (R10)
            let interval = if any_moves { TICK_MS } else { SLEEP_TICK_MS };
            std::thread::sleep(Duration::from_millis(interval));
        }
    });
}

/// 스냅샷을 창 위치와 웹뷰 상태에 반영한다. 창 이동과 상태 통지는 조건이
/// 다르다 — 자는 펭귄은 움직이지 않지만 "잔다"는 사실은 알려야 한다.
fn apply(window: &WebviewWindow, snapshot: Snapshot, move_window: bool, notify: bool) {
    if move_window {
        let (wx, wy) = window_origin(snapshot.x, snapshot.y);
        let _ = window.set_position(LogicalPosition::new(wx, wy));
    }
    if notify {
        let _ = window.emit(EVENT_PET_STATE, snapshot);
    }
}

/// 커맨드가 상태를 바꾼 뒤 즉시 화면에 반영한다 — 다음 틱(최대 500ms)을
/// 기다리면 클릭·드래그 반응이 굼떠 보인다. 커맨드는 항상 동작을 바꾸므로
/// 이동과 통지를 모두 한다.
fn flush(app: &AppHandle, id: PetId) -> Option<Snapshot> {
    let window = pet_window(app, id)?;
    let snapshot = app.state::<PetState>().pets.lock().unwrap().get(id)?.snapshot();
    apply(&window, snapshot, true, true);
    Some(snapshot)
}

/// 커맨드를 부른 창의 펭귄. 펫 창이 아니면 `None` — 빠따·드래그처럼 **자기
/// 펭귄에게만** 가야 하는 조작은 여기서 걸러진다 (KTD1).
fn caller_pet(window: &WebviewWindow) -> Option<PetId> {
    pet_id_from_label(window.label())
}

/// 추가·삭제의 대상. 펫 창이 부르면 자기 자신이고, 팝오버(`main`)가 부르면
/// 마지막으로 우클릭된 펭귄이다 (KTD6).
fn target_pet(window: &WebviewWindow, state: &PetState) -> Option<PetId> {
    caller_pet(window).or_else(|| *state.focused.lock().unwrap())
}

/// 빠따 — 왼쪽 클릭 한 번에 펭귄이 한 번 날아간다 (R14).
#[tauri::command]
pub fn pet_whack(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    let bounds = bounds_or_flat(&app, id);
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.whack(now_ms(), bounds);
    }
    flush(&app, id);
}

/// 오른쪽 클릭 — **펭귄 옆에서** 창을 연다(타이머·설정). 왼쪽 클릭은 빠따가 가져갔다.
/// 메뉴바 밑에서 열면 눌렀는데 화면 반대편에서 뜨는 셈이라 연결이 끊긴다.
#[tauri::command]
pub fn pet_open_popover(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    let Some(snapshot) = state.pets.lock().unwrap().get(id).map(|p| p.snapshot()) else {
        return;
    };
    // 팝오버가 열리기 **전에** 대상을 남긴다 — 팝오버는 자기가 어느 펭귄 때문에
    // 열렸는지 알 방법이 없고, "이 펭귄 삭제"는 그 답을 필요로 한다 (KTD6).
    *state.focused.lock().unwrap() = Some(id);
    let at = popover_anchor(&app, id, snapshot.x, snapshot.y);
    crate::toggle_popover_at(&app, at);
}

/// 현재 이동 영역. 모니터를 못 읽으면 납작한 경계를 쓴다 (보수적으로 동작한다).
fn bounds_or_flat(app: &AppHandle, id: PetId) -> Bounds {
    pet_window(app, id)
        .and_then(|w| current_bounds(&w))
        .unwrap_or(Bounds {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            floor_y: 0.0,
        })
}

/// 펭귄 위치에서 팝오버를 놓을 자리를 구한다. 모니터를 못 읽으면 `None`을
/// 돌려 트레이 밑(기존 동작)으로 떨어진다.
fn popover_anchor(app: &AppHandle, id: PetId, pet_x: f64, pet_y: f64) -> Option<(f64, f64)> {
    let popover = app.get_webview_window("main")?;
    let monitor = pet_window(app, id)?.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    // 팝오버 크기는 **팝오버가 있는 화면**의 배율로 나눠야 한다. 펭귄 쪽 배율을
    // 쓰면 배율이 다른 모니터가 섞였을 때 크기를 절반으로 오인해 화면 밖으로 나간다
    let popover_scale = popover.scale_factor().unwrap_or(scale);
    let size = popover.inner_size().ok()?;
    Some(popover_position_near(
        (pet_x, pet_y),
        PET_SIZE,
        (
            f64::from(size.width) / popover_scale,
            f64::from(size.height) / popover_scale,
        ),
        (
            f64::from(area.position.x) / scale,
            f64::from(area.position.y) / scale,
            f64::from(area.size.width) / scale,
            f64::from(area.size.height) / scale,
        ),
    ))
}

/// 펭귄과 팝오버 사이 여백 (논리 px).
const POPOVER_GAP: f64 = 8.0;

/// 펭귄 옆에 팝오버를 놓을 좌표. 오른쪽을 우선하되 넘치면 왼쪽으로 접고,
/// 그래도 안 되면 영역 안으로 자른다. 세로는 펭귄 높이에 맞추되 화면을 넘지 않는다.
///
/// 순수 함수로 떼어 낸 이유는 화면 밖으로 나가는 실수가 눈으로만 잡히기 때문이다.
/// `area`는 (left, top, width, height).
pub fn popover_position_near(
    pet: (f64, f64),
    pet_size: f64,
    popover: (f64, f64),
    area: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (pet_x, pet_y) = pet;
    let (pop_w, pop_h) = popover;
    let (area_x, area_y, area_w, area_h) = area;
    let (right_edge, bottom_edge) = (area_x + area_w, area_y + area_h);

    let to_right = pet_x + pet_size + POPOVER_GAP;
    let x = if to_right + pop_w <= right_edge {
        to_right
    } else {
        // 오른쪽이 모자라면 펭귄 왼쪽에 붙인다
        pet_x - pop_w - POPOVER_GAP
    };
    // 영역보다 팝오버가 크면 max가 min보다 작아져 clamp가 패닉한다
    let x = x.clamp(area_x, (right_edge - pop_w).max(area_x));
    let y = pet_y.clamp(area_y, (bottom_edge - pop_h).max(area_y));
    (x, y)
}

/// 드래그 시작 — 자율 이동을 멈춘다 (R6).
#[tauri::command]
pub fn pet_drag_start(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_start(now_ms());
    }
    flush(&app, id);
}

/// 드래그 이동량(논리 px). 창 위치의 소유자는 Rust 하나뿐이라 웹뷰는
/// 이동량만 보내고 직접 `setPosition`을 부르지 않는다 (KTD4).
#[tauri::command]
pub fn pet_drag_by(dx: f64, dy: f64, window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_by(dx, dy);
    }
    flush(&app, id);
}

/// 드래그 놓기 (R6, R12). 웹뷰가 잰 놓는 순간의 속도(논리 px/초)를 그대로 넘긴다 —
/// 세게 던졌으면 포물선을 그리고, 살짝 놓았으면 제자리에서 떨어진다.
///
/// 경계를 함께 넘기는 이유는 코어의 **속도 상한이 세계 폭에 비례**하기 때문이다.
/// `pet_whack`과 같은 방식이다.
#[tauri::command]
pub fn pet_drag_end(vx: f64, vy: f64, window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    let bounds = bounds_or_flat(&app, id);
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_end(now_ms(), vx, vy, bounds);
    }
    flush(&app, id);
}

/// 펭귄을 켜고 끈다 (R8). 끄면 창을 숨기지 않고 닫는다 — 틱 스레드도
/// 창이 없으면 느린 대기로 떨어져 자원을 쓰지 않는다.
/// 저장은 웹뷰가 담당한다 (기존 타이머 설정과 같은 방식).
#[tauri::command]
pub fn pet_set_enabled(enabled: bool, app: AppHandle) -> Result<(), String> {
    if enabled {
        spawn_saved_pets(&app).map_err(|e| e.to_string())
    } else {
        close_all_pet_windows(&app);
        app.state::<PetState>().pets.lock().unwrap().clear();
        Ok(())
    }
}

/// 웹뷰가 처음 뜰 때 현재 상태를 한 번 받아 간다 (첫 틱을 기다리지 않게).
#[tauri::command]
pub fn pet_get_state(window: WebviewWindow, state: State<'_, PetState>) -> Option<Snapshot> {
    let id = caller_pet(&window)?;
    let snapshot = state.pets.lock().unwrap().get(id).map(|p| p.snapshot());
    snapshot
}

/// 팝오버가 버튼 상태를 정하는 데 쓰는 요약 (마릿수·상한·우클릭 대상).
#[tauri::command]
pub fn pet_summary(state: State<'_, PetState>) -> PetSummary {
    let count = state.pets.lock().unwrap().len();
    let focused = *state.focused.lock().unwrap();
    PetSummary {
        count,
        max: MAX_PETS,
        focused,
    }
}

/// 펭귄 한 마리를 **부른 펭귄 옆에** 추가한다.
///
/// 전부 같은 자리에서 시작하면 겹쳐서 한 마리로 보이고, 무작위로 흩뿌리면 어디서
/// 생겼는지 모른다. "얘가 하나 더 불렀다"가 눈에 보이는 편이 낫다 (KTD5).
#[tauri::command]
pub fn pet_add(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<PetId, String> {
    let origin = target_pet(&window, &state);
    let bounds = origin
        .map(|id| bounds_or_flat(&app, id))
        .unwrap_or_else(|| bounds_or_flat_any(&app));
    let start_x = origin
        .and_then(|id| state.pets.lock().unwrap().get(id).map(|p| p.snapshot().x))
        .map(|x| next_to(x, bounds))
        .unwrap_or(bounds.left);

    let now = now_ms();
    let id = state
        .pets
        .lock()
        .unwrap()
        .add(now, now, bounds, start_x)
        .ok_or_else(|| format!("펭귄은 {MAX_PETS}마리까지예요"))?;

    let at = window_origin(start_x, bounds.floor_y);
    if let Err(err) = create_pet_window(&app, id, at) {
        // 창을 못 만들면 상태에만 남은 유령이 된다 — 되돌린다
        state.pets.lock().unwrap().forget(id);
        return Err(err.to_string());
    }
    save_pet_count(&app, state.pets.lock().unwrap().len());
    Ok(id)
}

/// 우클릭한 펭귄을 삭제한다. **마지막 한 마리는 거부한다** (PRD §5.5).
#[tauri::command]
pub fn pet_remove(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("어느 펭귄인지 모르겠어요")?;
    if !state.pets.lock().unwrap().remove(id) {
        return Err("마지막 한 마리는 지울 수 없어요. 전부 없애려면 펭귄을 꺼 주세요".into());
    }
    close_pet_window(&app, id);
    if *state.focused.lock().unwrap() == Some(id) {
        *state.focused.lock().unwrap() = None;
    }
    save_pet_count(&app, state.pets.lock().unwrap().len());
    Ok(())
}

/// 부른 펭귄 옆자리. 오른쪽을 우선하되 넘치면 왼쪽으로 접는다.
fn next_to(x: f64, bounds: Bounds) -> f64 {
    let gap = PET_SIZE * 0.75;
    let right = x + gap;
    let candidate = if right <= bounds.right { right } else { x - gap };
    candidate.clamp(bounds.left, bounds.right.max(bounds.left))
}

/// 아무 펫 창이나 기준으로 한 경계. 첫 마리를 만들 때처럼 기준 삼을 펭귄이
/// 없을 때 쓴다.
fn bounds_or_flat_any(app: &AppHandle) -> Bounds {
    any_pet_window(app)
        .and_then(|w| current_bounds(&w))
        .or_else(|| {
            app.get_webview_window("main")
                .and_then(|w| current_bounds(&w))
        })
        .unwrap_or(Bounds {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            floor_y: 0.0,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 등록을 빠뜨리면 컴파일도 되고 테스트도 통과하는데 런타임에서 모든 IPC가
    /// reject된다 — 커맨드는 `pub`이라 dead_code 경고도 안 뜬다. 실제로 한 번
    /// 놓쳤던 사각지대라 소스를 직접 대조한다.
    #[test]
    fn 모든_펫_커맨드가_invoke_handler에_등록되어_있다() {
        let bridge = include_str!("pet_bridge.rs");
        let lib = include_str!("lib.rs");

        let mut commands = Vec::new();
        let mut lines = bridge.lines().peekable();
        while let Some(line) = lines.next() {
            if line.trim() != "#[tauri::command]" {
                continue;
            }
            let signature = lines.peek().expect("커맨드 속성 뒤에 함수가 없다");
            let name = signature
                .trim()
                .strip_prefix("pub fn ")
                .and_then(|rest| rest.split('(').next())
                .expect("`pub fn 이름(` 형태를 기대한다");
            commands.push(name.to_string());
        }

        assert!(!commands.is_empty(), "커맨드를 하나도 찾지 못했다 — 탐지가 깨졌다");
        for name in commands {
            assert!(
                lib.contains(&format!("pet_bridge::{name},")),
                "`{name}`이 lib.rs의 invoke_handler 목록에 없다"
            );
        }
    }

    #[test]
    fn 방금_어긋난_것은_정리하지_않는다() {
        // 펭귄 추가는 (1) 상태에 넣고 (2) 창을 만드는 두 단계다. 그 사이를 보고
        // 바로 지우면 **방금 부른 펭귄이 조용히 사라진다** — 추가가 아무 일도
        // 안 하는 것처럼 보인다. 실제로 한 번 이렇게 깨졌다.
        let mut seen = HashMap::new();
        seen.insert(2, 10_000);
        assert!(due_for_cleanup(&seen, 10_050).is_empty(), "50ms만에 지우면 안 된다");
        assert!(due_for_cleanup(&seen, 10_999).is_empty());
    }

    #[test]
    fn 유예를_다_쓰면_정리한다() {
        let mut seen = HashMap::new();
        seen.insert(2, 10_000);
        assert_eq!(due_for_cleanup(&seen, 11_000), vec![2]);
        assert_eq!(due_for_cleanup(&seen, 30_000), vec![2]);
    }

    #[test]
    fn 시계가_뒤로_가도_정리가_앞당겨지지_않는다() {
        let mut seen = HashMap::new();
        seen.insert(2, 10_000);
        assert!(due_for_cleanup(&seen, 9_000).is_empty(), "음수 대신 0으로 본다");
    }

    #[test]
    fn 라벨에서_펭귄_id를_뽑는다() {
        assert_eq!(pet_label(3), "pet-3");
        assert_eq!(pet_id_from_label("pet-3"), Some(3));
        assert_eq!(pet_id_from_label(&pet_label(42)), Some(42));
    }

    #[test]
    fn 펫이_아닌_라벨은_id가_없다() {
        // 커맨드가 "누가 불렀는가"를 라벨로 정하므로, 여기서 새면 팝오버가
        // 남의 펭귄을 조작하게 된다 (KTD1)
        for label in ["main", "pet", "pet-", "pet-x", "pets-1", "", "PET-1"] {
            assert_eq!(pet_id_from_label(label), None, "`{label}`은 펫 창이 아니다");
        }
    }

    #[test]
    fn 옆자리는_영역_안에_들어온다() {
        let b = Bounds { left: 0.0, right: 1_000.0, top: 0.0, floor_y: 800.0 };
        // 오른쪽에 자리가 있으면 오른쪽
        assert!(next_to(100.0, b) > 100.0);
        // 오른쪽 끝이면 왼쪽으로 접는다
        assert!(next_to(b.right, b) < b.right);
        // 어느 쪽이든 영역 밖으로 나가지 않는다
        for x in [b.left, 500.0, b.right] {
            let n = next_to(x, b);
            assert!(n >= b.left && n <= b.right, "{n}이 영역을 벗어났다");
        }
    }

    #[test]
    fn 영역이_없어도_옆자리_계산이_패닉하지_않는다() {
        let flat = Bounds { left: 0.0, right: 0.0, top: 0.0, floor_y: 0.0 };
        assert_eq!(next_to(0.0, flat), 0.0);
    }

    use crate::pet::{IdleKind, SassyKind, Vertical};

    /// 1440x900 작업 영역, 360x540 팝오버, 140px 펭귄.
    const AREA: (f64, f64, f64, f64) = (0.0, 25.0, 1440.0, 875.0);
    const POP: (f64, f64) = (360.0, 540.0);

    #[test]
    fn 팝오버는_펭귄_오른쪽에_붙는다() {
        // 세로로 여유가 있는 높이에서 — 펭귄 높이에 그대로 맞춘다
        let (x, y) = popover_position_near((200.0, 120.0), PET_SIZE, POP, AREA);
        assert_eq!(x, 200.0 + PET_SIZE + POPOVER_GAP);
        assert_eq!(y, 120.0);
    }

    #[test]
    fn 아래쪽_펭귄에서는_팝오버가_화면_안으로_올라온다() {
        // 팝오버(540)가 작업 영역(875)에서 차지하는 몫이 커, 아래쪽에서는
        // 펭귄 높이에 맞출 수 없다. 잘리는 대신 위로 올라와야 한다
        let (_, y) = popover_position_near((200.0, 760.0), PET_SIZE, POP, AREA);
        assert_eq!(y, AREA.1 + AREA.3 - POP.1);
        assert!(y < 760.0, "펭귄보다 위로 올라와야 한다");
    }

    #[test]
    fn 오른쪽이_모자라면_펭귄_왼쪽에_붙는다() {
        // 오른쪽 끝 근처 — 오른쪽에 붙이면 화면을 넘는다
        let (x, _) = popover_position_near((1200.0, 400.0), PET_SIZE, POP, AREA);
        assert_eq!(x, 1200.0 - POP.0 - POPOVER_GAP);
    }

    #[test]
    fn 어느_위치에서도_화면을_벗어나지_않는다() {
        for px in [0.0, 300.0, 700.0, 1100.0, 1300.0] {
            for py in [25.0, 300.0, 700.0, 760.0] {
                let (x, y) = popover_position_near((px, py), PET_SIZE, POP, AREA);
                assert!(x >= AREA.0, "왼쪽으로 벗어남: {x}");
                assert!(x + POP.0 <= AREA.0 + AREA.2 + 0.001, "오른쪽으로 벗어남: {x}");
                assert!(y >= AREA.1, "위로 벗어남: {y}");
                assert!(y + POP.1 <= AREA.1 + AREA.3 + 0.001, "아래로 벗어남: {y}");
            }
        }
    }

    #[test]
    fn 팝오버가_영역보다_커도_패닉하지_않는다() {
        // clamp는 max < min이면 패닉한다
        let tiny = (0.0, 0.0, 200.0, 200.0);
        let (x, y) = popover_position_near((10.0, 10.0), PET_SIZE, POP, tiny);
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn 겉모습이_그대로면_다시_알리지_않는다() {
        let look = (Behavior::Walk, Facing::Right, Vertical::Level, false, None, 0);
        assert!(!should_notify(Some(look), look));
        assert!(should_notify(None, look), "처음에는 알려야 한다");
    }

    #[test]
    fn 세로_방향만_바뀌어도_웹뷰에_알린다() {
        // 헤엄 중 오름→내림은 동작도 좌우 방향도 그대로다. 이걸 놓치면
        // 몸 기울기가 영영 갱신되지 않는다
        let up = (Behavior::Swim, Facing::Right, Vertical::Up, true, None, 0);
        let down = (Behavior::Swim, Facing::Right, Vertical::Down, true, None, 0);
        assert!(should_notify(Some(up), down));
    }

    #[test]
    fn 좌우_방향만_바뀌어도_웹뷰에_알린다() {
        let right = (Behavior::Walk, Facing::Right, Vertical::Level, false, None, 0);
        let left = (Behavior::Walk, Facing::Left, Vertical::Level, false, None, 0);
        assert!(should_notify(Some(right), left));
    }

    #[test]
    fn 공중_여부만_바뀌어도_웹뷰에_알린다() {
        // 공중에서 클릭하면 동작·방향은 그대로인 채 air만 달라지는 순간이 있다.
        // 놓치면 그림자가 공중에 떠 있는 채로 남는다
        let ground = (Behavior::Sassy { sassy: SassyKind::EyeRoll }, Facing::Right, Vertical::Level, false, None, 0);
        let air = (Behavior::Sassy { sassy: SassyKind::EyeRoll }, Facing::Right, Vertical::Level, true, None, 0);
        assert!(should_notify(Some(ground), air));
    }

    #[test]
    fn 유휴_종류가_바뀌면_웹뷰에_알린다() {
        let a = (
            Behavior::Idle { idle: IdleKind::LookAround },
            Facing::Right,
            Vertical::Level,
            false,
            None,
            0,
        );
        let b = (
            Behavior::Idle { idle: IdleKind::Shake },
            Facing::Right,
            Vertical::Level,
            false,
            None,
            0,
        );
        assert!(should_notify(Some(a), b));
    }

    #[test]
    fn 경계는_창_여백까지_화면_안에_들어오게_잡는다() {
        // 배율 1.0, 메뉴바 25px를 뺀 1440x875 영역.
        // 펭귄만이 아니라 말풍선·방망이 자리까지 화면 안이어야 한다
        let b = bounds_from_work_area((0, 25), (1440, 875), 1.0, 140.0);
        assert_eq!(b.left, PET_PAD_X, "왼쪽 여백만큼 안으로 들어와야 한다");
        assert_eq!(b.right, 1440.0 - 140.0 - PET_PAD_X);
        assert_eq!(b.top, 25.0 + PET_PAD_TOP, "말풍선이 메뉴바 뒤로 숨으면 안 된다");
        assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
    }

    #[test]
    fn 어느_경계에_서도_창_전체가_화면_안이다() {
        let area = (0.0, 25.0, 1440.0, 875.0);
        let b = bounds_from_work_area((0, 25), (1440, 875), 1.0, PET_SIZE);
        for (px, py) in [
            (b.left, b.top),
            (b.right, b.top),
            (b.left, b.floor_y),
            (b.right, b.floor_y),
        ] {
            let (wx, wy) = window_origin(px, py);
            assert!(wx >= area.0 - 0.001, "창이 왼쪽으로 벗어남: {wx}");
            assert!(wx + PET_WINDOW_W <= area.0 + area.2 + 0.001, "오른쪽으로 벗어남");
            assert!(wy >= area.1 - 0.001, "창이 위로 벗어남: {wy}");
            assert!(wy + PET_WINDOW_H <= area.1 + area.3 + 0.001, "아래로 벗어남");
        }
    }

    #[test]
    fn 레티나_배율에서도_논리_좌표로_환산한다() {
        // 물리 2880x1750 = 논리 1440x875 (배율 2.0)
        let b = bounds_from_work_area((0, 50), (2880, 1750), 2.0, 140.0);
        assert_eq!(b.left, PET_PAD_X);
        assert_eq!(b.right, 1440.0 - 140.0 - PET_PAD_X);
        assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
    }

    #[test]
    fn 보조_모니터처럼_원점이_음수여도_경계가_밀린다() {
        let b = bounds_from_work_area((-1920, 0), (1920, 1080), 1.0, 140.0);
        assert_eq!(b.left, -1920.0 + PET_PAD_X);
        assert_eq!(b.right, -1920.0 + 1920.0 - 140.0 - PET_PAD_X);
    }

    #[test]
    fn 영역이_펭귄보다_좁아도_경계가_뒤집히지_않는다() {
        let b = bounds_from_work_area((0, 0), (100, 100), 1.0, 140.0);
        assert!(b.right >= b.left, "right가 left보다 작아지면 clamp가 패닉한다");
        assert!(b.floor_y >= 0.0);
    }
}
