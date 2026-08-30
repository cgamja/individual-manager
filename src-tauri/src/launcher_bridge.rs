//! 런처가 고른 URL을 Chrome으로 여는 브릿지.
//!
//! `tauri-plugin-opener`는 **기본 브라우저**를 열기 때문에 PRD §5.2의 "Chrome 창으로
//! 열린다"를 만족하지 못한다. macOS `open -a`는 지정한 앱으로 열고, 그 앱을 앞으로
//! 가져오며, 이미 실행 중이면 새 프로세스 대신 기존 인스턴스의 새 탭으로 연다 —
//! PRD Q5의 "이미 열려 있으면 앞으로"를 앱 수준에서 만족한다.

use tauri::AppHandle;

/// macOS에서 `open -a`에 넘길 앱 이름.
#[cfg(target_os = "macos")]
const CHROME_APP: &str = "Google Chrome";

/// 열어도 되는 URL인가.
///
/// URL은 설정 저장소를 거쳐 웹뷰에서 넘어오므로 여기서 다시 검증한다. 이걸 열어 두면
/// `open -a`에 임의 파일 경로나 다른 스킴을 태울 수 있다. 프론트에도 같은 검증이 있지만
/// 웹뷰의 판단을 신뢰하지 않는 게 요점이다.
pub fn is_launchable_url(url: &str) -> bool {
    let trimmed = url.trim();
    // 앞뒤 공백만 잘라 낸 게 원본과 다르면 거절한다 — 스킴 앞에 공백을 끼워 넣는
    // 우회를 막고, 실제로 실행되는 문자열과 검증한 문자열을 같게 유지한다
    if trimmed != url || trimmed.is_empty() {
        return false;
    }
    // 제어 문자가 섞인 URL은 인자 경계를 흐릴 수 있어 받지 않는다
    if url.chars().any(char::is_control) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("http://") && url.len() > "http://".len())
        || (lower.starts_with("https://") && url.len() > "https://".len())
}

/// 런처 항목을 Chrome으로 연다. 잘못된 URL이면 프로세스를 띄우지 않고 거절한다.
#[tauri::command]
pub fn launcher_open(app: AppHandle, url: String) -> Result<(), String> {
    if !is_launchable_url(&url) {
        return Err(format!("열 수 없는 URL이에요: {url}"));
    }
    open_url(&app, &url)
}

#[cfg(target_os = "macos")]
fn open_url(app: &AppHandle, url: &str) -> Result<(), String> {
    // `status()`로 기다리지 않는다 — 브라우저가 뜨는 동안 팝오버가 멈춘다
    match std::process::Command::new("open")
        .args(["-a", CHROME_APP, url])
        .spawn()
    {
        Ok(_) => Ok(()),
        // Chrome이 없을 수 있다. 그때는 기본 브라우저로라도 연다
        Err(_) => open_with_default(app, url),
    }
}

#[cfg(not(target_os = "macos"))]
fn open_url(app: &AppHandle, url: &str) -> Result<(), String> {
    open_with_default(app, url)
}

fn open_with_default(app: &AppHandle, url: &str) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|err| format!("브라우저를 열지 못했어요: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http와_https만_열_수_있다() {
        assert!(is_launchable_url("https://calendar.google.com/calendar/r/week"));
        assert!(is_launchable_url("http://localhost:3000"));
        assert!(is_launchable_url("HTTPS://EXAMPLE.TEST"));
    }

    #[test]
    fn 빈_url과_공백은_거절한다() {
        assert!(!is_launchable_url(""));
        assert!(!is_launchable_url("   "));
        assert!(!is_launchable_url(" https://example.test"));
        assert!(!is_launchable_url("https://example.test "));
    }

    #[test]
    fn 스킴만_있고_주소가_없으면_거절한다() {
        assert!(!is_launchable_url("https://"));
        assert!(!is_launchable_url("http://"));
    }

    #[test]
    fn file과_javascript_스킴은_거절한다() {
        assert!(!is_launchable_url("file:///etc/passwd"));
        assert!(!is_launchable_url("javascript:alert(1)"));
        assert!(!is_launchable_url("/Applications/Calculator.app"));
        assert!(!is_launchable_url("-a"));
    }

    #[test]
    fn 제어_문자가_섞인_url은_거절한다() {
        assert!(!is_launchable_url("https://example.test\nopen -a Terminal"));
    }

    /// 등록을 빠뜨리면 컴파일도 되고 테스트도 통과하는데 런타임에서 IPC가 조용히
    /// reject된다. `pet_bridge.rs`가 같은 이유로 두고 있는 검사를 여기에도 둔다.
    #[test]
    fn 모든_런처_커맨드가_invoke_handler에_등록되어_있다() {
        let bridge = include_str!("launcher_bridge.rs");
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
                lib.contains(&format!("launcher_bridge::{name},")),
                "`{name}`이 lib.rs의 invoke_handler 목록에 없다"
            );
        }
    }
}
