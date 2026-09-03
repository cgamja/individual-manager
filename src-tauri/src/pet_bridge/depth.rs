//! 겹친 펭귄의 **앞뒤**를 정한다 — 야차에서만 쓴다.
//!
//! 야차는 마리들이 위아래로도 다니므로 누가 앞인지가 매 프레임 바뀐다. 아티팩트는
//! SVG 하나에 다 그려서 그리는 순서로 공짜였지만, **앱은 마리마다 창이 따로다.**
//!
//! **창 순서로는 못 잡는다.** macOS에서 같은 레벨 안의 순서는 클릭할 때마다 바뀌고,
//! 올려 두는 방식은 이 레포에서 전부 실패했다
//! (`docs/solutions/ui-bugs/macos-window-order-is-not-stable-level-is.md`).
//! 그 문서의 결론이 그대로 답이다 — **순서가 아니라 레벨로 잡는다.**
//!
//! 그래서 아래에 있는(= 가까운) 마리일수록 높은 레벨을 준다. 레벨 범위는
//! `PET_DEPTH_BASE`(3, 평소 펭귄 레벨)부터 마릿수만큼 위로다.
//!
//! **왜 이 범위가 안전한가**
//! - 핀볼 판·코트는 2라 여전히 전부 아래다 (그쪽 근거는 각 모듈에 있다).
//! - 메뉴바는 24, 팝업 메뉴는 101이라 위로 뚫지 않는다 — 트레이가 "나가는 문"이라
//!   가려지면 안 된다 (PRD §5.8).
//! - 설정 창(`main`)은 `always_on_top`이 아니라 0이므로, 펭귄이 그 위에 오는 것은
//!   **지금도 그렇다** — 이 기능이 바꾸는 관계가 아니다.
//!
//! **`ns_window()` 아래는 반드시 메인 스레드다.** 여기는 20Hz 틱에서 불리므로
//! `run_on_main_thread`를 빠뜨리면 앱이 흔적 없이 죽는다
//! (`docs/solutions/best-practices/appkit-from-tick-thread-kills-the-app.md`).

use tauri::{AppHandle, Manager};

use super::QUEEN_LABEL;
use crate::pet::{PetId, MAX_PETS};

/// 평소 펭귄 창의 레벨 (`always_on_top`이 주는 값).
pub const PET_DEPTH_BASE: isize = 3;

/// 미녀는 늘 맨 앞이다 — 벨트를 채우는 배우가 챔피언 뒤로 숨으면 세레모니가
/// 안 보인다.
pub const QUEEN_DEPTH_LEVEL: isize = PET_DEPTH_BASE + MAX_PETS as isize;

/// 창 레벨이 뚫으면 안 되는 천장 — 메뉴바(24)다. 트레이는 핀볼의 "나가는 문
/// 둘" 중 하나라 절대 가려지면 안 된다.
pub const MENU_BAR_LEVEL: isize = 24;

/// 틱이 들고 다니는 캐시. **순서가 안 바뀌면 AppKit을 안 만진다** — 매 틱
/// 메인 스레드로 8번씩 건너가면 그것대로 비싸다.
#[derive(Default)]
pub struct DepthView {
    order: Vec<PetId>,
    /// 미녀 창에 레벨을 걸어 뒀는가. **창이 생긴 뒤에 걸어야 하므로** 순서와
    /// 따로 기억한다.
    queen: bool,
}

impl DepthView {
    /// 판이 끝났다 — 다음 판에서 반드시 다시 걸도록 잊는다.
    pub fn forget(&mut self) {
        self.order.clear();
        self.queen = false;
    }

    /// 지금 레벨이 걸려 있는 마리들. 판이 끝날 때 되돌릴 대상이다.
    pub fn order(&self) -> &[PetId] {
        &self.order
    }
}

/// `order`는 **뒤에서 앞** 순서다 (먼 마리가 앞).
///
/// **미녀는 이 순서에 안 낀다.** 정렬에 섞이면 y에 따라 3~10 중 하나를 받아
/// 앞에 있는 펭귄에게 덮인다 — 특히 **쓰러진 마리**가 위험하다: 넘어지면 y가
/// 바뀌어 순서가 다시 계산되기 때문이다. 그래서 미녀만 정렬 밖의 고정 레벨
/// (`QUEEN_DEPTH_LEVEL`)을 받는다. 아티팩트가 정렬한 뒤 미녀를 **마지막에**
/// 붙이는 것과 같은 규칙이다.
///
/// **한 번 걸고 끝내지 않는다.** 창 순서는 클릭할 때마다 바뀌므로 매 틱
/// 순서가 달라질 때마다 다시 건다 — 미녀도 같은 자리에서 함께 건다
/// (`docs/solutions/ui-bugs/macos-window-order-is-not-stable-level-is.md`).
#[cfg(target_os = "macos")]
pub fn apply_depth(app: &AppHandle, order: &[PetId], view: &mut DepthView) {
    let 미녀 = app.get_webview_window(QUEEN_LABEL).is_some();
    if view.order == order && view.queen == 미녀 {
        return;
    }
    view.order = order.to_vec();
    view.queen = 미녀;
    let app = app.clone();
    let order = order.to_vec();
    let _ = app.clone().run_on_main_thread(move || {
        for (k, id) in order.iter().enumerate() {
            set_level(&app, &format!("pet-{id}"), depth_level(k));
        }
        // **미녀는 늘 맨 위다.** 벨트를 채우는 배우가 쓰러진 놈에게 가려지면
        // 세레모니가 안 보인다.
        set_level(&app, QUEEN_LABEL, QUEEN_DEPTH_LEVEL);
    });
}

#[cfg(target_os = "macos")]
fn set_level(app: &AppHandle, label: &str, level: isize) {
    use objc2_app_kit::NSWindow;
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    let ptr = match window.ns_window() {
        Ok(ptr) if !ptr.is_null() => ptr,
        _ => return,
    };
    unsafe {
        let ns = &*(ptr as *const NSWindow);
        ns.setLevel(level);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply_depth(_app: &AppHandle, _order: &[PetId], _view: &mut DepthView) {}

/// 판이 끝나면 **전부 평소 레벨로 되돌린다.** 안 되돌리면 야차를 한 번 한 뒤로
/// 그 마리들이 서로 다른 레벨에 남아, 평소에 겹칠 때 앞뒤가 이상해진다.
#[cfg(target_os = "macos")]
pub fn reset_depth(app: &AppHandle, ids: &[PetId], view: &mut DepthView) {
    if view.order.is_empty() {
        return;
    }
    view.forget();
    let app = app.clone();
    let ids = ids.to_vec();
    let _ = app.clone().run_on_main_thread(move || {
        for id in ids {
            set_level(&app, &format!("pet-{id}"), PET_DEPTH_BASE);
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn reset_depth(_app: &AppHandle, _ids: &[PetId], view: &mut DepthView) {
    view.forget();
}

/// 뒤에서 `k`번째 마리의 창 레벨.
pub fn depth_level(k: usize) -> isize {
    PET_DEPTH_BASE + k.min(MAX_PETS - 1) as isize
}

#[cfg(test)]
#[path = "depth_tests.rs"]
mod tests;
