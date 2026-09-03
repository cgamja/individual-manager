//! 펫 브릿지 — 바탕화면 펭귄 창의 생성과 수명 관리.
//!
//! 창 플래그는 전부 여기 한 곳에서 정한다. 창 레벨을 "항상 위"에서 "데스크톱 뒤"로
//! 뒤집고 싶어지면 고칠 곳도 여기 하나다 (KTD3).

use std::sync::Mutex;

use serde::Serialize;

use crate::pet::{BallSnapshot, Behavior, Facing, PetId, Pets, Snapshot, Vertical};

/// 지금(epoch ms). 코어(`pet.rs`)는 시간을 주입받는 순수 모듈이라 시계를 갖지 않는다 —
/// 시계를 읽는 곳은 브릿지 하나뿐이어야 테스트가 시간을 마음대로 돌릴 수 있다.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 웹뷰가 구독하는 상태 이벤트.
pub const EVENT_PET_STATE: &str = "pet://state";

/// 설정이 **이 창 밖에서** 바뀌었음을 설정 창에 알린다.
pub const EVENT_PET_SETTINGS: &str = "pet://settings";

/// 공 창이 구독하는 상태 이벤트. 펭귄과 나누는 이유는 받는 창이 다르고
/// 페이로드도 다르기 때문이다 — 하나로 합치면 공 창이 스무 마리치 이벤트를
/// 걸러 내야 한다.
pub const EVENT_BALL_STATE: &str = "bowling://ball";

/// 비치볼 창이 구독하는 상태 이벤트. 공 창이 따로라 이벤트도 따로다.
pub const EVENT_VOLLEY_STATE: &str = "volley://ball";

/// 미녀 펭귄 창이 구독하는 상태 이벤트. 창이 따로라 이벤트도 따로다.
pub const EVENT_YACHA_QUEEN: &str = "yacha://queen";

/// 야차 판이 **끝났음**을 설정 창에 알린다 (볼링·발리볼과 같은 자리).
pub const EVENT_YACHA_OVER: &str = "yacha://over";

/// 비치발리볼 판이 **끝났음**을 설정 창에 알린다. 판을 끝내는 것은 예산이지
/// 사용자가 아니라서, 이게 없으면 버튼이 비활성인 채로 남는다 (볼링과 같다).
pub const EVENT_VOLLEY_OVER: &str = "volley://over";

/// 볼링 판이 **끝났음**을 설정 창에 알린다. 판을 끝내는 것은 공이지 사용자가
/// 아니라서, 이걸 안 보내면 "볼링 한 판" 버튼이 비활성인 채로 남는다
/// (설정 창을 닫았다 다시 열기 전까지).
pub const EVENT_BOWLING_OVER: &str = "bowling://over";

/// 창을 놓는 **유일한 문** — 크기를 먼저 걸고 자리를 나중에 건다.
///
/// **순서가 정확성이다.** macOS에서 `set_position`은 `setFrameTopLeftPoint`(좌상단
/// 기준)이고 `set_size`는 `setContentSize`(좌**하**단 기준)다. 둘 다 부른 순서대로
/// 메인 큐에 실리므로, 자리를 먼저 걸면 뒤따르는 크기 변경이 위 모서리를
/// `높이 차이`만큼 밀어 올린다/내린다. 창이 커지거나 작아지는 순간에만 어긋나고
/// 그 뒤 캐시에 적히면 스스로 못 고친다.
///
/// `size`가 `None`이면 자리만 옮긴다 (크기가 안 변하는 경로).
///
/// **성공 여부를 돌려준다.** 부르는 쪽은 "이만큼 걸었다"를 캐시하는데, 실패한
/// 값을 캐시하면 다음 틱이 "이미 맞다"고 보고 넘어가 화해 장치가 그 자리에서
/// 무력해진다 — 창은 영영 안 맞는다.
#[must_use]
pub fn place_window(
    window: &tauri::WebviewWindow,
    at: (f64, f64),
    size: Option<(f64, f64)>,
) -> bool {
    use tauri::{LogicalPosition, LogicalSize};
    if let Some((w, h)) = size {
        if window.set_size(LogicalSize::new(w, h)).is_err() {
            return false;
        }
    }
    window
        .set_position(LogicalPosition::new(at.0, at.1))
        .is_ok()
}

pub struct PetState {
    pub pets: Mutex<Pets>,
    /// 마지막으로 우클릭된 펭귄. 팝오버(`main` 창)는 자기가 **어느 펭귄 때문에**
    /// 열렸는지 모르므로, 삭제 대상을 알려면 여는 쪽이 남겨 줘야 한다 (KTD6).
    pub focused: Mutex<Option<PetId>>,
    /// 웹뷰가 "포인터가 펭귄 밖이니 통과시켜 달라"고 남긴 요청. **요청일 뿐**
    /// 이고 플래그를 걸고 되돌리는 것은 틱이 한다 (`pet_bridge/hit.rs`).
    ///
    /// **`pets`와 동시에 잡지 않는다** — 락 둘을 겹쳐 쥐면 순서가 갈리는 순간
    /// 데드락이다
    /// (`docs/solutions/best-practices/rust-for-loop-holds-mutex-guard-across-body.md`).
    pub click_through: Mutex<std::collections::HashMap<PetId, bool>>,
}

impl PetState {
    pub fn new(pets: Pets) -> Self {
        PetState {
            pets: Mutex::new(pets),
            focused: Mutex::new(None),
            click_through: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

/// 팝오버가 버튼 상태를 정하는 데 쓰는 요약. 누를 수 없는 버튼은 **비활성**으로
/// 보여야 한다 — 눌리는데 아무 일도 없으면 고장으로 읽힌다.
#[derive(Serialize)]
pub struct PetSummary {
    pub count: usize,
    pub max: usize,
    pub focused: Option<PetId>,
    /// 볼링 판이 도는 중인가. 도는 중에 또 누르면 무시되므로(A3) 버튼을 끈다.
    pub bowling: bool,
    /// 비치발리볼 판이 도는 중인가. **세 판은 서로를 배제하므로** 어느 하나든
    /// 도는 동안 버튼 셋이 함께 비활성된다.
    pub volleyball: bool,
    /// 단체 야차 판이 도는 중인가. 위와 같은 규칙이다.
    pub yacha: bool,
}

/// 웹뷰가 보는 "겉모습" — 이게 바뀔 때만 상태를 다시 알린다.
/// **CSS 클래스에 영향을 주는 값은 빠짐없이 들어가야 한다.** 하나라도 빠지면
/// 그 값만 바뀌는 전이가 웹뷰에 영영 도달하지 않는다(조용한 실패).
/// **`punch_seq`가 들어 있는 이유는 `whack_seq`가 들어 있는 이유와 같다.**
/// 난투 중에는 연타가 흔한데(스윙 뒤 또 스윙이 64%) 그때 국면은 `Punch` 그대로고,
/// 막힌 주먹은 맞은 쪽을 `Guard` 그대로 둔다. 번호를 안 보면 **그 스냅샷이
/// 아예 안 나가** 퍽이 유실되고 그림도 안 다시 뜬다.
pub type Look = (Behavior, Facing, Vertical, bool, Option<u64>, u64, bool, u64);

pub fn look_of(snapshot: &Snapshot) -> Look {
    (
        snapshot.behavior,
        snapshot.facing,
        snapshot.vertical,
        snapshot.air,
        snapshot.speech.map(|s| s.seq),
        snapshot.whack_seq,
        snapshot.pinball,
        snapshot.punch_seq,
    )
}

/// 겉모습이 달라졌는가. 매 틱 emit하면 웹뷰가 이유 없이 20Hz로 리렌더한다.
pub fn should_notify(last: Option<Look>, now: Look) -> bool {
    last != Some(now)
}

/// 공 웹뷰가 보는 "겉모습". 위치는 창이 옮기므로 여기 들어가지 않는다 —
/// 넣으면 굴러가는 내내 20Hz로 리렌더한다.
pub type BallLook = (bool, bool);

pub fn ball_look_of(ball: &BallSnapshot) -> BallLook {
    (ball.rolling, ball.held)
}

/// 이번 틱에 "판이 끝났다"를 알려야 하는가.
///
/// **공이 아니라 판을 보고 정한다.** 공은 전부 서기 전에는 없으므로(R4),
/// 모으는 중에 참여 마리가 전부 빠져 판이 끝나면 공 쪽 기억만으로는 끝난 줄을
/// 모른다 — 그러면 설정 창의 "볼링 한 판" 버튼이 비활성인 채로 남는다.
pub fn bowling_over(was_alive: bool, is_alive: bool) -> bool {
    was_alive && !is_alive
}

mod ball_window;
mod bounds;
pub mod commands;
mod hit;
mod pinball;
mod depth;
mod volleyball;
mod yacha;
mod popover;
mod scale;
mod settings;
mod tick;
mod window;

pub use ball_window::*;
pub use bounds::*;
pub use commands::*;
pub use hit::*;
pub use pinball::*;
pub use depth::*;
pub use volleyball::*;
pub use yacha::*;
pub use popover::*;
pub use scale::*;
pub use settings::*;
pub use tick::*;
pub use window::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
