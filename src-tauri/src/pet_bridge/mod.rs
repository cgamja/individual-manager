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

/// 비치발리볼 판이 **끝났음**을 설정 창에 알린다. 판을 끝내는 것은 예산이지
/// 사용자가 아니라서, 이게 없으면 버튼이 비활성인 채로 남는다 (볼링과 같다).
pub const EVENT_VOLLEY_OVER: &str = "volley://over";

/// 볼링 판이 **끝났음**을 설정 창에 알린다. 판을 끝내는 것은 공이지 사용자가
/// 아니라서, 이걸 안 보내면 "볼링 한 판" 버튼이 비활성인 채로 남는다
/// (설정 창을 닫았다 다시 열기 전까지).
pub const EVENT_BOWLING_OVER: &str = "bowling://over";

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
    /// 비치발리볼 판이 도는 중인가. **두 판은 서로를 배제하므로** 어느 쪽이든
    /// 도는 동안 버튼 둘이 함께 비활성된다.
    pub volleyball: bool,
}

/// 웹뷰가 보는 "겉모습" — 이게 바뀔 때만 상태를 다시 알린다.
/// **CSS 클래스에 영향을 주는 값은 빠짐없이 들어가야 한다.** 하나라도 빠지면
/// 그 값만 바뀌는 전이가 웹뷰에 영영 도달하지 않는다(조용한 실패).
pub type Look = (Behavior, Facing, Vertical, bool, Option<u64>, u64, bool);

pub fn look_of(snapshot: &Snapshot) -> Look {
    (
        snapshot.behavior,
        snapshot.facing,
        snapshot.vertical,
        snapshot.air,
        snapshot.speech.map(|s| s.seq),
        snapshot.whack_seq,
        snapshot.pinball,
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
mod volleyball;
mod popover;
mod settings;
mod tick;
mod window;

pub use ball_window::*;
pub use bounds::*;
pub use commands::*;
pub use hit::*;
pub use pinball::*;
pub use volleyball::*;
pub use popover::*;
pub use settings::*;
pub use tick::*;
pub use window::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
