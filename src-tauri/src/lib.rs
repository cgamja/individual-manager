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

/// 테마를 **창에만** 건다 (웹뷰의 prefers-color-scheme이 뒤집힌다).
pub(crate) fn apply_theme(app: &AppHandle, theme: pet_bridge::Theme) {
    use pet_bridge::Theme;
    app.set_theme(match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    });
}

/// 설정 창의 테마 선택. 저장은 웹뷰(`savePetSettings`)가 한다 — 여기는
/// 지금 떠 있는 창과 트레이에 즉시 거는 쪽이다.
#[tauri::command]
fn pet_set_theme(app: AppHandle, theme: String) {
    apply_theme(
        &app,
        pet_bridge::theme_from(Some(&serde_json::json!({ "theme": theme }))),
    );
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
            let _ = app.show();
            let _ = window.show();
            let _ = window.set_focus();
        }
        None => show_popover(app, &window),
    }
}

fn show_popover(app: &AppHandle, window: &WebviewWindow) {
    let handle = window.as_ref().window();
    if handle.move_window(Position::TrayCenter).is_err() {
        let _ = handle.move_window(Position::TopRight);
    }
    let _ = app.show();
    let _ = window.show();
    let _ = window.set_focus();
}

fn hide_popover(_app: &AppHandle, window: &WebviewWindow) {
    let _ = window.hide();
}

/// 트레이 클릭 — 메뉴바 밑에서 연다.
pub(crate) fn toggle_popover(app: &AppHandle) {
    *app.state::<pet_bridge::PetState>().focused.lock().unwrap() = None;
    toggle_popover_at(app, None);
}

/// 두 번째 실행이 들어왔을 때 이미 떠 있는 인스턴스가 하는 일.
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

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
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

            apply_theme(&app.handle().clone(), pet_bridge::pet_theme(app.handle()));

            app.manage(pet_bridge::PetState::new(pet::Pets::new()));
            if pet_bridge::pet_enabled(app.handle()) {
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
            pet_bridge::commands::pet_whack,
            pet_bridge::commands::pet_open_popover,
            pet_bridge::commands::pet_drag_start,
            pet_bridge::commands::pet_drag_by,
            pet_bridge::commands::pet_drag_end,
            pet_bridge::commands::pet_get_state,
            pet_bridge::commands::pet_set_enabled,
            pet_bridge::commands::pet_set_pinball,
            pet_bridge::commands::pet_summary,
            pet_bridge::commands::pet_add,
            pet_bridge::commands::pet_remove,
            pet_bridge::commands::pet_fish,
            pet_bridge::commands::pet_slide,
            pet_bridge::commands::pet_squawk,
            pet_bridge::commands::pet_freakout,
            pet_bridge::commands::pet_dont_ask,
            pet_bridge::commands::bowling_start,
            pet_bridge::commands::volleyball_start,
            pet_bridge::commands::volley_get_state,
            pet_bridge::commands::ball_drag_start,
            pet_bridge::commands::ball_drag_by,
            pet_bridge::commands::ball_drag_end,
            pet_set_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
