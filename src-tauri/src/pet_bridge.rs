//! 펫 브릿지 — 바탕화면 펭귄 창의 생성과 수명 관리.
//!
//! 창 플래그는 전부 여기 한 곳에서 정한다. 창 레벨을 "항상 위"에서 "데스크톱 뒤"로
//! 뒤집고 싶어지면 고칠 곳도 여기 하나다 (KTD3).

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

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
        .build()
}

/// 펫 창을 닫는다. 없으면 아무것도 하지 않는다.
pub fn close_pet_window(app: &AppHandle) {
    if let Some(window) = pet_window(app) {
        let _ = window.close();
    }
}
