//! 단체 야차의 창 하나 — 챔피언에게 벨트를 채워 주는 **미녀 펭귄**.
//!
//! **경기장을 안 그린다.** 볼링·비치발리볼은 판을 깔았지만 야차는 화면 한가운데에
//! 펭귄들이 뭉치는 것이 전부다 (2026-09-03 사용자 지시). 그래서 이 모듈이
//! **`ns_window()`를 한 번도 안 만진다** — 원래 링 레벨을 펭귄 아래로 내리려면
//! 거기 내려가야 했고, 그게 틱에서 불리면 앱이 흔적 없이 죽는 자리였다
//! (`docs/solutions/best-practices/appkit-from-tick-thread-kills-the-app.md`).
//! 미녀 창은 펭귄과 같은 레벨 3이라 내릴 일이 없다.
//!
//! **클릭은 통과시킨다.** 그림뿐인 창이라 먹을 근거가 없고, 먹으면 "방해하지
//! 않는다"(PRINCIPLE 5)를 근거 없이 어긴다. `set_ignore_cursor_events`가
//! 비동기라는 함정은 비치발리볼 모듈의 머리말에 있다 — 여기서도 **안 보이게
//! 만들고 → 플래그를 걸고 → 보인다**, 그리고 **직후에 읽어서 확인하지 않는다.**

use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::pet::{QueenSnapshot, YachaSnapshot};

use super::*;

/// 미녀 창의 라벨. **`capabilities/default.json`의 `windows`에 있어야 한다** —
/// 없으면 이 창의 `listen`·`invoke`가 컴파일·테스트를 다 통과하고 **런타임에서만
/// 조용히 reject된다**
/// (`docs/solutions/best-practices/tauri-command-registration-silent-failure.md`).
pub const QUEEN_LABEL: &str = "yacha-queen";

/// 미녀의 코어 좌표(펭귄 좌상단) → 창 좌상단. **펭귄 창과 같은 규칙이다** —
/// 같은 그림을 같은 여백으로 그리므로 자리 계산도 같아야 한다.
pub fn queen_window_origin(x: f64, y: f64, scale: f64) -> (f64, f64) {
    window_origin(x, y, scale)
}

pub fn queen_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(QUEEN_LABEL)
}

/// 미녀 창을 만든다. 이미 있으면 그것을 돌려준다.
///
/// **레벨은 펭귄과 같은 3이다** — 비치볼이 코트처럼 안 내려간 것과 같은 이유:
/// 벨트를 채우는 배우가 챔피언 뒤로 숨으면 세레모니가 안 보인다.
pub fn create_queen_window(
    app: &AppHandle,
    at: (f64, f64),
    scale: f64,
) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = queen_window(app) {
        return Ok(existing);
    }
    let (w, h) = pet_window_size(scale);
    let window = WebviewWindowBuilder::new(app, QUEEN_LABEL, WebviewUrl::App("queen.html".into()))
        .title("Yacha Queen")
        .initialization_script(&scale_init_script(scale))
        .inner_size(w, h)
        .position(at.0, at.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .focusable(false)
        // **안 보이게 만들고 플래그를 건 뒤에 보인다.** 세터가 비동기라
        // 순서를 이렇게 잡아야 "클릭을 먹는 창"이 뜨는 구간이 가장 짧다.
        .visible(false)
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()?;
    // **여기서 실패하면 반쯤 만든 창을 반드시 닫는다.** 안 닫으면 다음 틱의
    // "이미 있으면 그것을 돌려준다"가 그 숨은 창을 채택해 플래그도 `show`도
    // 다시 안 걸린다 (비치발리볼의 `그림_창`과 같은 규칙).
    let 세우기 = window
        .set_ignore_cursor_events(true)
        .and_then(|()| window.show());
    if let Err(err) = 세우기 {
        let _ = window.close();
        return Err(err);
    }
    Ok(window)
}

/// 미녀 창을 닫는다. **`app.hide()`는 절대 부르지 않는다** — macOS 26에서
/// 트레이 아이콘까지 사라진다.
pub fn close_queen_window(app: &AppHandle) {
    if let Some(window) = queen_window(app) {
        let _ = window.close();
    }
}

/// 틱이 들고 다니는 캐시. 판이 끝나면 전부 비운다.
#[derive(Default)]
pub struct YachaView {
    /// 겹친 마리의 앞뒤 — 창 레벨로 잡는다 (`pet_bridge/depth.rs`).
    depth: DepthView,
    at: Option<(f64, f64)>,
    size: Option<(f64, f64)>,
    look: Option<QueenSnapshot>,
    board_alive: bool,
    fails: u32,
}

pub(super) const YACHA_WINDOW_MAX_FAILS: u32 = 5;

/// 미녀를 창에 반영한다. 판이 없거나 아직 난투 중이면 창을 닫는다.
pub(super) fn apply_yacha(
    app: &AppHandle,
    board: Option<YachaSnapshot>,
    view: &mut YachaView,
    scale: f64,
) {
    let 판이_있나 = board.is_some();
    let 깊이: Vec<PetId> = board.as_ref().map(|s| s.depth.clone()).unwrap_or_default();
    // **겹친 마리의 앞뒤는 창 레벨로 잡는다.** 순서로는 못 잡는다 —
    // macOS에서 같은 레벨 안의 순서는 클릭할 때마다 바뀐다.
    if 판이_있나 {
        apply_depth(app, &깊이, &mut view.depth);
    } else {
        let ids: Vec<PetId> = view.depth.order().to_vec();
        reset_depth(app, &ids, &mut view.depth);
    }

    let queen = board.and_then(|s| s.queen);
    let Some(queen) = queen else {
        // **`||`를 쓰면 안 된다 — 단축평가가 뒤쪽 `take()`를 건너뛴다.**
        // 낡은 `look`이 살아남으면 다음 판의 첫 국면이 "달라진 게 없다"로 걸러져
        // 미녀가 걸어 들어오는 그림을 통째로 놓친다 (`apply_volley`와 같은 함정).
        let 있었다 = view.at.take().is_some() | view.look.take().is_some();
        if 있었다 {
            close_queen_window(app);
            view.size = None;
        }
        // 판 자체가 끝났을 때만 설정 창에 알린다 — 난투 중에는 미녀가 없을 뿐
        // 판은 살아 있다.
        if bowling_over(view.board_alive, 판이_있나) {
            let _ = app.emit(EVENT_YACHA_OVER, ());
        }
        view.board_alive = 판이_있나;
        view.fails = 0;
        return;
    };
    view.board_alive = true;

    let at = queen_window_origin(queen.x, queen.y, scale);
    let size = pet_window_size(scale);
    let window = match queen_window(app) {
        Some(window) => window,
        None => match create_queen_window(app, at, scale) {
            Ok(window) => {
                view.fails = 0;
                view.at = Some(at);
                view.size = Some(size);
                window
            }
            Err(err) => return 실패(app, view, err),
        },
    };

    let 다시_잰다 = view.size != Some(size);
    if (다시_잰다 || view.at != Some(at)) && place_window(&window, at, 다시_잰다.then_some(size)) {
        view.size = Some(size);
        view.at = Some(at);
    }
    if view.look != Some(queen) {
        let _ = window.emit_to(
            EventTarget::webview_window(QUEEN_LABEL),
            EVENT_YACHA_QUEEN,
            queen,
        );
        view.look = Some(queen);
    }
}

/// 창을 못 만들었다. 상한을 넘기면 **판을 접어** 펭귄을 풀어 준다 — 조용히 계속
/// 두드리면 사용자는 굳은 펭귄만 보게 된다.
fn 실패(app: &AppHandle, view: &mut YachaView, err: tauri::Error) {
    view.fails += 1;
    eprintln!(
        "[penguin] 미녀 창을 못 만들었다 ({}/{YACHA_WINDOW_MAX_FAILS}): {err}",
        view.fails
    );
    if view.fails >= YACHA_WINDOW_MAX_FAILS {
        // **락을 한 번만 잡는다.** 경계를 읽고 판을 접는 것을 나눠 잡으면 그
        // 사이에 커맨드가 판을 바꿀 수 있다.
        let state = app.state::<PetState>();
        let mut pets = state.pets.lock().unwrap();
        if let Some(bounds) = pets.yacha().map(|b| b.arena().bounds()) {
            pets.end_yacha(now_ms(), bounds);
        }
        drop(pets);
        close_queen_window(app);
        view.fails = 0;
        view.board_alive = false;
        view.at = None;
        view.size = None;
        view.look = None;
        let _ = app.emit(EVENT_YACHA_OVER, ());
    }
}

#[cfg(test)]
#[path = "yacha_tests.rs"]
mod tests;
