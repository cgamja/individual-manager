pub mod notion;
pub mod notion_bridge;
pub mod pet;
pub mod pet_bridge;
pub mod pomodoro;
pub mod timer_bridge;

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

fn show_popover(app: &AppHandle, window: &WebviewWindow) {
    let _ = window.as_ref().window().move_window(Position::TrayCenter);
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

pub(crate) fn toggle_popover(app: &AppHandle) {
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
        // blur 숨김이 먼저 처리된 같은 클릭 — 닫기 의도로 간주한다
        return;
    }
    show_popover(app, &window);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(ShellState {
                hidden_at: Mutex::new(None),
            });
            app.manage(timer_bridge::TimerState(Mutex::new(
                pomodoro::Pomodoro::new(pomodoro::Config::default()),
            )));
            timer_bridge::spawn_tick_thread(app.handle().clone());

            let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // 트레이는 setup()에서 동기 생성해야 마우스 이벤트를 받는다 (KTD3, tauri#11462)
            TrayIconBuilder::with_id(timer_bridge::TRAY_ID)
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
            let start = timer_bridge::now_ms();
            app.manage(pet_bridge::PetState(Mutex::new(pet::Pet::new(
                start,
                start,
                pet::Bounds { left: 0.0, right: 800.0, floor_y: 400.0 },
            ))));
            if pet_bridge::pet_enabled(app.handle()) {
                if let Err(err) = pet_bridge::create_pet_window(app.handle()) {
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
            timer_bridge::timer_start,
            timer_bridge::timer_pause,
            timer_bridge::timer_resume,
            timer_bridge::timer_reset,
            timer_bridge::timer_get_state,
            timer_bridge::timer_get_config,
            timer_bridge::timer_set_config,
            notion_bridge::notion_save_token,
            notion_bridge::notion_delete_token,
            notion_bridge::notion_set_database,
            notion_bridge::notion_get_status,
            notion_bridge::notion_test_connection,
            notion_bridge::notion_todo_list,
            notion_bridge::notion_todo_create_page,
            notion_bridge::notion_todo_create_row,
            notion_bridge::notion_todo_open_page,
            notion_bridge::notion_todo_add,
            notion_bridge::notion_todo_toggle,
            notion_bridge::notion_todo_edit,
            notion_bridge::notion_todo_set_performance,
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
