//! 핀볼 판 — 화면마다 하나씩 까는 투명 창. 펭귄 창보다 아래 레벨이다.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 핀볼 덮개 창의 라벨. **`capabilities/default.json`의 `windows`에 있어야 한다** —
/// 없으면 이 창이 부르는 커맨드가 컴파일·테스트를 다 통과하고 **런타임에서만
/// 조용히 reject된다** (`docs/solutions/best-practices/tauri-command-registration-silent-failure.md`).
pub const PINBALL_LABEL_PREFIX: &str = "pinball-board-";

/// 화면 `index`를 덮는 판의 라벨.
pub fn pinball_label(index: usize) -> String {
    format!("{PINBALL_LABEL_PREFIX}{index}")
}

/// 화면 하나를 재서 넘기는 값 — (좌상단 물리 좌표, 물리 크기, 배율).
pub type ScreenSpec = ((i32, i32), (u32, u32), f64);

/// 화면 하나가 논리 좌표로 차지하는 사각형. 크기가 0이면 `None`이다 —
/// `primary_monitor()`는 화면이 하나도 없어도 `Some`을 주면서 크기 0인 핸들을
/// 내놓는다 ([`bounds_of_work_area`]와 같은 이유).
pub(super) fn screen_rect(
    pos: (i32, i32),
    size: (u32, u32),
    scale: f64,
) -> Option<(f64, f64, f64, f64)> {
    if size.0 == 0 || size.1 == 0 || scale <= 0.0 {
        return None;
    }
    Some((
        f64::from(pos.0) / scale,
        f64::from(pos.1) / scale,
        f64::from(size.0) / scale,
        f64::from(size.1) / scale,
    ))
}

/// 덮개가 덮을 사각형들 — **화면 하나에 하나씩**이다.
pub fn pinball_rects_of(screens: &[ScreenSpec]) -> Vec<(f64, f64, f64, f64)> {
    screens
        .iter()
        .filter_map(|(pos, size, scale)| screen_rect(*pos, *size, *scale))
        .collect()
}

/// 핀볼 덮개 창을 만든다. 이미 있으면 그대로 둔다.
pub fn create_pinball_window(app: &AppHandle) -> tauri::Result<()> {
    let screens: Vec<ScreenSpec> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            (
                (m.position().x, m.position().y),
                (m.size().width, m.size().height),
                m.scale_factor(),
            )
        })
        .collect();
    let rects = pinball_rects_of(&screens);
    if rects.is_empty() {
        return Err(tauri::Error::WindowNotFound);
    }

    for (index, (x, y, w, h)) in rects.into_iter().enumerate() {
        let label = pinball_label(index);
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        WebviewWindowBuilder::new(app, &label, WebviewUrl::App("pinball.html".into()))
            .title("Pinball Field")
            .inner_size(w, h)
            .position(x, y)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible_on_all_workspaces(true)
            .accept_first_mouse(true)
            .focused(false)
            .visible(true)
            .build()?
            .show()?;
    }

    sink_pinball_below_pets(app);
    Ok(())
}

/// 판을 펭귄보다 **한 레벨 아래**로 내린다.
#[cfg(target_os = "macos")]
pub fn sink_pinball_below_pets(app: &AppHandle) {
    use objc2_app_kit::NSWindow;

    let labels: Vec<String> = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(PINBALL_LABEL_PREFIX))
        .cloned()
        .collect();
    for label in labels {
        let Some(window) = app.get_webview_window(&label) else {
            continue;
        };
        let ptr = match window.ns_window() {
            Ok(ptr) if !ptr.is_null() => ptr,
            _ => {
                eprintln!("[penguin] 판({label})의 창 레벨을 못 내렸다 — 펭귄이 안 만져질 수 있다");
                continue;
            }
        };
        unsafe {
            let ns = &*(ptr as *const NSWindow);
            ns.setLevel(PINBALL_WINDOW_LEVEL);
        }
    }
}

/// 판의 창 레벨. `NSFloatingWindowLevel`(3, 펭귄이 쓰는 값) 바로 아래.
#[cfg(target_os = "macos")]
pub const PINBALL_WINDOW_LEVEL: isize = 2;

#[cfg(not(target_os = "macos"))]
pub fn sink_pinball_below_pets(_app: &AppHandle) {}

/// 핀볼 덮개 창을 닫는다.
pub fn close_pinball_window(app: &AppHandle) {
    let labels: Vec<String> = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(PINBALL_LABEL_PREFIX))
        .cloned()
        .collect();
    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
}
