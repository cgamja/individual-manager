//! 비치발리볼의 창 둘 — 코트(모래 + 네트)와 비치볼.
//!
//! **둘 다 클릭을 통과시킨다.** 핀볼 판이 클릭을 **먹는** 근거는 "커서가 어디서나
//! 채가 된다"였고, 그래서 나가는 문이 둘 필요했다 (PRD §5.8). 비치발리볼은
//! **그림뿐이라 그 근거가 없다** — 먹으면 "방해하지 않는다"(PRINCIPLE 5)를 근거
//! 없이 어긴다. 화면을 날아다니는 56px짜리 공이 클릭을 채 가면 그게 정확히 방해다.
//!
//! **`set_ignore_cursor_events`는 비동기다** — `Ok(())`를 즉시 돌려주지만 적용은
//! 이벤트 루프를 거친다 (`set_position`이 메인 스레드로 디스패치되는 것과 같은
//! 성질). 그래서 **호출 직후에 읽어서 확인하면 안 된다**: 반환값도 `Ok`, 로그도
//! 깨끗한데 읽으면 `false`라 "안 먹는다"는 오답이 나온다.
//! → `docs/solutions/best-practices/tauri-ignore-cursor-events-is-async.md`
//!
//! 대신 창을 **안 보이게 만들고 → 플래그를 걸고 → 보인다.** 둘 다 같은 이벤트
//! 루프를 순서대로 지나므로 이게 간극을 가장 좁힌다. **그래도 한 프레임은 남을
//! 수 있고, CSS로는 그 구간을 못 메운다** — `pointer-events: none`은 웹뷰가
//! 반응하지 않게 할 뿐 네이티브 창이 클릭을 먹는 것은 그대로다.
//!
//! **레벨과 클릭 통과는 서로를 대신하지 못한다.** 코트는 펭귄보다 아래여야 하고
//! (`ns_window()`로 레벨을 내린다), 그것과 별개로 클릭을 통과시켜야 한다 —
//! 레벨만 내리면 한 번 클릭했을 때 도로 올라온다
//! (`docs/solutions/ui-bugs/macos-window-order-is-not-stable-level-is.md`).

use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::pet::{VolleyBallSnapshot, VolleySnapshot, VOLLEY_BALL_SIZE};

use super::*;

/// 코트 창의 라벨. **`capabilities/default.json`의 `windows`에 있어야 한다** —
/// 없으면 이 창의 `listen`·`invoke`가 컴파일·테스트를 다 통과하고 **런타임에서만
/// 조용히 reject된다**
/// (`docs/solutions/best-practices/tauri-command-registration-silent-failure.md`).
pub const COURT_LABEL: &str = "volley-court";

/// 비치볼 창의 라벨. 같은 규칙이다.
pub const VBALL_LABEL: &str = "volley-ball";

/// 공 창의 크기 — 공에 딱 맞춘다 (배율 1 기준).
pub const VBALL_WINDOW_SIZE: f64 = VOLLEY_BALL_SIZE;

/// 화면에 놓이는 비치볼 한 변. **그림은 `width: 100%`라 창만 줄이면 따라온다.**
pub fn vball_window_size(scale: f64) -> f64 {
    VBALL_WINDOW_SIZE * scale
}

/// 코트의 코어 사각형 → 화면 사각형. `VolleyView`는 **이 결과**를 기억한다 —
/// 코어 좌표를 기억하면 배율만 바뀐 틱에서 "달라진 게 없다"로 걸러져 코트만
/// 옛 크기로 남는다.
pub fn court_rect_on_screen(rect: (f64, f64, f64, f64), scale: f64) -> (f64, f64, f64, f64) {
    (
        to_screen(rect.0, scale),
        to_screen(rect.1, scale),
        to_screen(rect.2, scale),
        to_screen(rect.3, scale),
    )
}

/// 코트 창의 레벨 — 펭귄(3)보다 **아래**다. 핀볼 판과 같은 값이지만 상수를
/// 빌려 쓰지 않는다: 이름이 "핀볼"이면 다음 사람이 핀볼을 튜닝하다 코트를
/// 조용히 움직이게 된다.
#[cfg(target_os = "macos")]
pub const COURT_WINDOW_LEVEL: isize = 2;

/// 공 **중심**의 코어 좌표 → 화면 좌표의 창 좌상단.
pub fn vball_window_origin(x: f64, y: f64, scale: f64) -> (f64, f64) {
    let side = vball_window_size(scale);
    (to_screen(x, scale) - side / 2.0, to_screen(y, scale) - side / 2.0)
}

pub fn court_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(COURT_LABEL)
}

pub fn vball_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(VBALL_LABEL)
}

/// 그림만 그리는 창의 공통 플래그. 코트와 공이 **정확히 같은 성질**이라
/// 한 곳에서 만든다 — 한쪽만 클릭을 먹게 되는 갈래를 아예 없앤다.
fn 그림_창(
    app: &AppHandle,
    label: &str,
    url: &str,
    at: (f64, f64),
    size: (f64, f64),
) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title("Beach Volleyball")
        .inner_size(size.0, size.1)
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
        // 순서를 이렇게 잡아야 "클릭을 먹는 코트"가 화면에 뜨는 구간이 가장 짧다.
        .visible(false)
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()?;
    // **여기서 실패하면 반쯤 만든 창을 반드시 닫는다.**
    //
    // 창은 이미 라벨로 등록됐으므로, 그냥 `Err`를 돌려주면 다음 틱의
    // "이미 있으면 그것을 돌려준다"가 그 **숨은 창을 채택한다** — 플래그도
    // `show`도 다시 안 걸리고, `apply_volley`는 성공 갈래로 가서 `실패`를
    // 다시는 안 부른다. 재시도 예산이 통째로 죽고 20초짜리 판이 보이지 않는
    // 코트·공으로 조용히 돌아간다. (핀볼 판의 `build_all_or_none`이 반쯤 깔린
    // 판을 남기지 않는 것과 같은 규칙이다.)
    let 세우기 = window
        .set_ignore_cursor_events(true)
        .and_then(|()| window.show());
    if let Err(err) = 세우기 {
        let _ = window.close();
        return Err(err);
    }
    Ok(window)
}

/// 코트 창을 만든다. 이미 있으면 그것을 돌려준다.
pub fn create_court_window(
    app: &AppHandle,
    rect: (f64, f64, f64, f64),
) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = court_window(app) {
        return Ok(existing);
    }
    let window = 그림_창(
        app,
        COURT_LABEL,
        "volley-court.html",
        (rect.0, rect.1),
        (rect.2, rect.3),
    )?;
    sink_court_below_pets(app);
    Ok(window)
}

/// 공 창을 만든다. **레벨은 펭귄과 같은 3이다** — 코트처럼 내리지 않는다:
/// 날아다니는 공이 펭귄 뒤로 숨으면 랠리가 안 보인다.
pub fn create_vball_window(
    app: &AppHandle,
    at: (f64, f64),
    scale: f64,
) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = vball_window(app) {
        return Ok(existing);
    }
    let side = vball_window_size(scale);
    그림_창(app, VBALL_LABEL, "volley-ball.html", at, (side, side))
}

/// 코트를 펭귄보다 **한 레벨 아래**로 내린다. 핀볼 판과 같은 이유·같은 방법이다.
#[cfg(target_os = "macos")]
pub fn sink_court_below_pets(app: &AppHandle) {
    // **반드시 메인 스레드에서 부른다.** 코트 창은 핀볼 판과 달리 커맨드가 아니라
    // **20Hz 틱 스레드**가 만든다(`apply_volley`). AppKit 객체를 그 스레드에서
    // 직접 만지면 앱이 **통째로 죽는다** — 패닉도 Tauri 종료 이벤트도 없이
    // 사라져서 로그에 아무 단서가 안 남는다.
    //
    // `set_position`을 아무 스레드에서나 불러도 되는 것과 헷갈리면 안 된다(KTD5):
    // 그쪽은 tauri-runtime-wry가 이벤트 루프로 넘겨 주지만, `ns_window()`로 꺼낸
    // 포인터를 직접 만지는 것은 그 디스패치를 **건너뛴다.**
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        use objc2_app_kit::NSWindow;

        let Some(window) = court_window(&app) else {
            return;
        };
        let ptr = match window.ns_window() {
            Ok(ptr) if !ptr.is_null() => ptr,
            _ => {
                eprintln!("[penguin] 코트의 창 레벨을 못 내렸다 — 펭귄이 가려질 수 있다");
                return;
            }
        };
        unsafe {
            let ns = &*(ptr as *const NSWindow);
            ns.setLevel(COURT_WINDOW_LEVEL);
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn sink_court_below_pets(_app: &AppHandle) {}

/// 창 둘을 닫는다. **`app.hide()`는 절대 부르지 않는다** — macOS 26에서
/// 트레이 아이콘까지 사라진다.
pub fn close_volley_windows(app: &AppHandle) {
    for label in [COURT_LABEL, VBALL_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.close();
        }
    }
}

/// 틱 스레드가 비치발리볼에 대해 들고 있는 유일한 기억.
#[derive(Default)]
pub struct VolleyView {
    /// 웹뷰에 마지막으로 알린 공의 겉모습. `Some`이면 공 창이 떠 있다는 뜻이다.
    look: Option<VolleyLook>,
    /// 마지막으로 공 창에 건 위치. 같으면 `set_position`을 안 부른다.
    ball_at: Option<(f64, f64)>,
    /// 마지막으로 코트 창에 건 사각형 — **화면 좌표다** ([`court_rect_on_screen`]).
    /// 코트는 판이 도는 동안 안 변하지만, 모니터나 배율이 바뀌면 다시 잰다.
    court_rect: Option<(f64, f64, f64, f64)>,
    /// 직전 틱에 판이 살아 있었는가 — 설정 창에 "끝났다"를 알리는 데 쓴다.
    board_alive: bool,
    /// 잇달아 창 만들기에 실패한 횟수.
    fails: u32,
}

/// 창 만들기를 이만큼 잇달아 실패하면 판을 접는다. **재시도에 끝이 없으면**
/// 20Hz로 영원히 두드리면서 사용자에게는 아무 신호도 안 가고 펭귄은 코트에
/// 굳은 채로 남는다 (볼링 공 창과 같은 규칙).
pub(super) const VOLLEY_WINDOW_MAX_FAILS: u32 = 5;

/// 코트와 공을 창에 반영한다. 판이 없으면 창 둘을 닫는다.
pub(super) fn apply_volley(
    app: &AppHandle,
    board: Option<VolleySnapshot>,
    view: &mut VolleyView,
    scale: f64,
) {
    let Some(snapshot) = board else {
        // **`||`를 쓰면 안 된다 — 단축평가가 `view.look.take()`를 건너뛴다.**
        // 코트가 먼저 서므로 `court_rect`는 거의 항상 `Some`이고, 그러면 랠리
        // 도중에 끝난 판의 `look`(= `Some(true)`)이 그대로 살아남는다. 다음 판의
        // 첫 공도 `flying == true`라 "달라진 게 없다"로 걸러져 **새 창에 상태가
        // 한 번도 안 가고 공이 판 내내 안 돈다.**
        let 있었다 = view.court_rect.take().is_some() | view.look.take().is_some();
        if 있었다 {
            close_volley_windows(app);
            view.ball_at = None;
        }
        if bowling_over(view.board_alive, false) {
            let _ = app.emit(EVENT_VOLLEY_OVER, ());
        }
        view.board_alive = false;
        view.fails = 0;
        return;
    };
    view.board_alive = true;

    // 코트를 먼저 세운다 — 모래가 깔리기 전에 공이 뜨면 허공에서 튀는 그림이 된다.
    let court = court_rect_on_screen(snapshot.court, scale);
    match court_window(app) {
        Some(window) => {
            if view.court_rect != Some(court) {
                let (x, y, w, h) = court;
                let _ = window.set_position(LogicalPosition::new(x, y));
                let _ = window.set_size(LogicalSize::new(w, h));
                view.court_rect = Some(court);
            }
        }
        None => match create_court_window(app, court) {
            Ok(_) => {
                view.fails = 0;
                view.court_rect = Some(court);
            }
            Err(err) => return 실패(app, view, "코트", err),
        },
    }

    let Some(ball) = snapshot.ball else {
        // 아직 모이는 중이다 — 공은 다 서야 나온다.
        return;
    };
    let at = vball_window_origin(ball.x, ball.y, scale);
    let window = match vball_window(app) {
        Some(window) => window,
        None => match create_vball_window(app, at, scale) {
            Ok(window) => {
                view.fails = 0;
                view.ball_at = Some(at);
                window
            }
            Err(err) => return 실패(app, view, "비치볼", err),
        },
    };

    if view.ball_at != Some(at) {
        let _ = window.set_position(LogicalPosition::new(at.0, at.1));
        view.ball_at = Some(at);
    }
    let look = volley_look_of(&ball);
    if view.look != Some(look) {
        let _ = window.emit_to(
            EventTarget::webview_window(VBALL_LABEL),
            EVENT_VOLLEY_STATE,
            ball,
        );
        view.look = Some(look);
    }
}

/// 창을 못 만들었다. 상한을 넘기면 **판을 접어** 펭귄을 코트에서 풀어 준다 —
/// 조용히 계속 두드리면 사용자는 굳은 펭귄만 보게 된다.
fn 실패(app: &AppHandle, view: &mut VolleyView, 무엇: &str, err: tauri::Error) {
    view.fails += 1;
    eprintln!(
        "[penguin] {무엇} 창을 못 만들었다 ({}/{VOLLEY_WINDOW_MAX_FAILS}): {err}",
        view.fails
    );
    if view.fails >= VOLLEY_WINDOW_MAX_FAILS {
        app.state::<PetState>()
            .pets
            .lock()
            .unwrap()
            .end_volleyball(now_ms());
        close_volley_windows(app);
        view.fails = 0;
        view.board_alive = false;
        view.court_rect = None;
        view.look = None;
        view.ball_at = None;
        let _ = app.emit(EVENT_VOLLEY_OVER, ());
    }
}

/// 공 웹뷰가 보는 "겉모습". 위치는 창이 옮기므로 여기 안 들어간다 — 넣으면
/// 날아가는 내내 20Hz로 리렌더한다.
pub type VolleyLook = bool;

pub fn volley_look_of(ball: &VolleyBallSnapshot) -> VolleyLook {
    ball.flying
}

#[cfg(test)]
#[path = "volleyball_tests.rs"]
mod tests;
