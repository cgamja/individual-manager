pub mod pet;
pub mod pet_bridge;

/// 트레이 아이콘 id. 트레이를 다시 찾을 때 쓴다.
const TRAY_ID: &str = "main-tray";

use std::sync::Mutex;
use std::time::Instant;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

/// blur 숨김 직후의 트레이 클릭을 "닫기 의도"로 판정하기 위한 셸 상태.
struct ShellState {
    hidden_at: Mutex<Option<Instant>>,
}

/// 팝오버가 blur로 숨겨진 직후(~300ms)의 트레이 클릭은 재표시하지 않는다 (토글 경쟁 방지).
const TOGGLE_RACE_WINDOW_MS: u128 = 300;

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// 팝오버를 지정한 자리에 놓고 연다. `at`이 없으면 트레이 밑(기존 동작).
pub(crate) fn toggle_popover_at(app: &AppHandle, at: Option<(f64, f64)>) {
    let Some(window) = main_window(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide_popover(app, &window);
        return;
    }
    let recently_blurred = app
        .state::<ShellState>()
        .hidden_at
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed().as_millis() < TOGGLE_RACE_WINDOW_MS);
    if recently_blurred {
        return;
    }
    match at {
        Some((x, y)) => {
            let _ = window.set_position(tauri::LogicalPosition::new(x, y));
            // Accessory 정책에서는 app.show() → set_focus() 순서여야 포커스가 잡힌다
            let _ = app.show();
            let _ = window.show();
            let _ = window.set_focus();
        }
        None => show_popover(app, &window),
    }
}

fn show_popover(app: &AppHandle, window: &WebviewWindow) {
    // TrayCenter는 positioner가 트레이 이벤트에서 캐시해 둔 좌표를 쓴다. 트레이를
    // 한 번도 건드리지 않은 채 펭귄 클릭으로 들어오면 캐시가 비어 실패하므로,
    // 그때는 메뉴바 근처(TopRight)로 떨어뜨린다 — 조용히 화면 좌상단에 뜨지 않게.
    let handle = window.as_ref().window();
    if handle.move_window(Position::TrayCenter).is_err() {
        let _ = handle.move_window(Position::TopRight);
    }
    // Accessory 정책에서는 app.show() → set_focus() 순서여야 키보드 포커스가 잡힌다 (KTD4)
    let _ = app.show();
    let _ = window.show();
    let _ = window.set_focus();
}

fn hide_popover(_app: &AppHandle, window: &WebviewWindow) {
    // app.hide()는 쓰지 않는다 — macOS 26(Tahoe)에서 상태바 아이템 창까지 함께 숨겨져
    // 메뉴바 펭귄 아이콘이 사라진다. blur로 닫힐 때는 포커스가 이미 다른 앱으로
    // 넘어간 뒤라 창만 숨겨도 충분하다.
    let _ = window.hide();
}

/// 트레이 클릭 — 메뉴바 밑에서 연다.
pub(crate) fn toggle_popover(app: &AppHandle) {
    // 트레이로 열면 "이 펭귄"이 없다. 지우지 않으면 아까 우클릭했던 펭귄이
    // 대상으로 남아, 보고 있지도 않은 펭귄이 조용히 지워진다.
    *app.state::<pet_bridge::PetState>().focused.lock().unwrap() = None;
    toggle_popover_at(app, None);
}

/// 두 번째 실행이 들어왔을 때 이미 떠 있는 인스턴스가 하는 일.
///
/// **토글이 아니라 열기다.** 사용자가 앱을 다시 실행한 것은 "띄워 달라"는 뜻인데,
/// 마침 팝오버가 열려 있었다는 이유로 닫아 버리면 정반대로 동작한다.
fn greet_second_launch(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.set_focus();
        return;
    }
    show_popover(app, &window);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 단일 인스턴스 가드는 **다른 플러그인보다 먼저** 등록한다 (플러그인 문서).
        // 두 번째 실행은 여기서 걸려 스스로 물러나고, 그 대신 이미 떠 있는 인스턴스가
        // 팝오버를 열어 "나 여기 있다"를 알린다 — 아무 반응이 없으면 실행에 실패한
        // 것으로 오해한다. Rust에서만 쓰므로 capabilities 등록은 필요 없다.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            greet_second_launch(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(ShellState {
                hidden_at: Mutex::new(None),
            });

            let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // 트레이는 setup()에서 동기 생성해야 마우스 이벤트를 받는다 (KTD3, tauri#11462)
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(false)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // positioner에 항상 먼저 전달해야 TrayCenter 좌표가 계산된다 (KTD3)
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle());
                    }
                })
                .build(app)?;

            // 바탕화면 펭귄 — 실패해도 앱 본체는 계속 뜬다 (장식 기능이 셸을 막지 않는다).
            // 실제 이동 영역은 첫 틱에서 모니터를 읽어 정정하므로 여기서는 잠정값이다.
            app.manage(pet_bridge::PetState::new(pet::Pets::new()));
            if pet_bridge::pet_enabled(app.handle()) {
                // 저장된 마릿수만큼 만든다. 실패해도 앱 본체는 계속 뜬다.
                if let Err(err) = pet_bridge::spawn_saved_pets(app.handle()) {
                    eprintln!("펭귄 창 생성 실패: {err}");
                }
            }
            pet_bridge::spawn_pet_tick_thread(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                if window.label() == "main" {
                    let app = window.app_handle();
                    if let Some(main) = main_window(app) {
                        hide_popover(app, &main);
                    }
                    *app.state::<ShellState>().hidden_at.lock().unwrap() = Some(Instant::now());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            pet_bridge::pet_whack,
            pet_bridge::pet_open_popover,
            pet_bridge::pet_drag_start,
            pet_bridge::pet_drag_by,
            pet_bridge::pet_drag_end,
            pet_bridge::pet_get_state,
            pet_bridge::pet_set_enabled,
            pet_bridge::pet_set_pinball,
            pet_bridge::pet_summary,
            pet_bridge::pet_add,
            pet_bridge::pet_remove,
            pet_bridge::pet_fish,
            pet_bridge::pet_slide,
            pet_bridge::pet_squawk,
            pet_bridge::pet_freakout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn 테스트_하네스가_구동된다() {
        assert_eq!(1 + 1, 2);
    }
}
