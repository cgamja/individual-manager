//! 볼링 공 창 — 펭귄 창과 **같은 성질의 물체**라 같은 패턴을 쓴다.
//!
//! 공은 "바탕화면에 놓여 있고, 마우스로 끌 수 있고, Rust 틱이 위치를 옮기는
//! 물체"다. 그래서 창 플래그·레벨·드래그 규약이 전부 펭귄 창과 같다 (KTD5).
//! 덕분에 화면 좌표 → 세계 좌표 변환을 새로 짤 필요가 없다: 드래그는 절대
//! 좌표가 아니라 **델타**로 오간다.
//!
//! **화면을 덮는 판을 만들지 않는다** (KTD4). 핀볼 판이 화면을 덮는 이유는
//! "커서가 어디서나 채"가 되어야 해서고, 볼링은 공 하나를 끄는 것뿐이라
//! 덮을 이유가 없다. 덮으면 방해하지 않는다(PRINCIPLE 5)를 근거 없이 어긴다.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::pet::BOWLING_BALL_SIZE;

/// 공 창의 라벨. **`capabilities/default.json`의 `windows`에 있어야 한다** —
/// 없으면 이 창이 부르는 커맨드가 컴파일·테스트를 다 통과하고 **런타임에서만
/// 조용히 reject된다**
/// (`docs/solutions/best-practices/tauri-command-registration-silent-failure.md`).
pub const BALL_LABEL: &str = "bowling-ball";

/// 창은 공에 딱 맞춘다. 펭귄 창과 달리 말풍선도 방망이도 없다.
pub const BALL_WINDOW_SIZE: f64 = BOWLING_BALL_SIZE;

/// 공 **중심** 좌표 → 창 좌상단. 코어가 공을 중심으로 들고 있는 이유는
/// 히트 판정이 중심끼리의 거리라서다.
pub fn ball_window_origin(x: f64, y: f64) -> (f64, f64) {
    (x - BALL_WINDOW_SIZE / 2.0, y - BALL_WINDOW_SIZE / 2.0)
}

pub fn ball_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(BALL_LABEL)
}

/// 공 창을 만든다. 이미 있으면 그것을 돌려준다.
pub fn create_ball_window(app: &AppHandle, at: (f64, f64)) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = ball_window(app) {
        return Ok(existing);
    }
    WebviewWindowBuilder::new(app, BALL_LABEL, WebviewUrl::App("ball.html".into()))
        .title("Bowling Ball")
        .inner_size(BALL_WINDOW_SIZE, BALL_WINDOW_SIZE)
        .position(at.0, at.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        // 레벨은 펭귄과 **같은 3**이다. 핀볼 판처럼 내리지 않는다 — 공은 가려지면
        // 집을 수 없고, 판과 달리 화면을 덮지도 않으므로 펭귄을 가로막지 않는다.
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .accept_first_mouse(true)
        .focused(false)
        .focusable(false)
        .visible(true)
        .build()
        .inspect(|window| {
            let _ = window.show();
        })
}

/// 공 창을 닫는다. 없으면 아무것도 하지 않는다. **`app.hide()`는 절대 부르지
/// 않는다** — macOS 26에서 트레이 아이콘까지 사라진다.
pub fn close_ball_window(app: &AppHandle) {
    if let Some(window) = ball_window(app) {
        let _ = window.close();
    }
}
