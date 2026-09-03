//! 펭귄 코어 — Tauri 무의존 순수 상태머신.
//!
//! 시간은 epoch ms로, 놀 수 있는 영역은 [`World`](화면마다 자기 [`Bounds`]를 갖는
//! [`Screen`]의 목록)로 주입받는다. 난수도 코어가
//! 소유한 시드 PRNG라 같은 시드 + 같은 타임스탬프열은 항상 같은 동작 시퀀스를 낳는다 —
//! 그래야 "스스로 움직이는" 동작을 테스트로 고정할 수 있다 (KTD1).

use serde::Serialize;

#[cfg(test)]
mod test_support;
mod tuning;

use tuning::*;
/// 브릿지가 창 크기를 계산하는 데 쓴다 — 코어 밖으로 나가는 튜닝 값은 이 둘뿐이다.
pub use tuning::{BOWLING_BALL_SIZE, PET_SIZE, VOLLEY_BALL_SIZE};

mod behavior;
mod bowling;
mod volleyball;
mod yacha;

use bowling::{dist2_to_segment, pin_positions};
pub use bowling::{BallSnapshot, BoardPhase, Bowling};

pub use volleyball::{Court, CourtPhase, Side, VolleyBallSnapshot, VolleySnapshot, Volleyball};

use volleyball::{assign_sides, both_sides_present};
pub use yacha::{Arena, Punch, QueenPose, QueenSnapshot, RingPhase, Yacha, YachaSnapshot};

pub use behavior::{
    Behavior, BowlingPhase, Facing, FishingPhase, FreakoutPhase, IdleKind, SassyKind, Speech,
    Vertical, VolleyPhase, YachaPhase,
};
use behavior::{IDLE_KINDS, SASSY_KINDS};

mod motion;
mod world;

pub use world::{Bounds, Screen, ScreenId, World};

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct Snapshot {
    pub x: f64,
    pub y: f64,
    pub facing: Facing,
    pub vertical: Vertical,
    /// 바닥에서 떠 있는가. 동작만으로는 알 수 없다 — 공중에서 클릭하면
    /// 지상 동작인 반응을 하면서도 떠 있다.
    pub air: bool,
    /// 지금 떠 있는 말풍선. 없으면 조용하다.
    pub speech: Option<Speech>,
    /// 빠따를 맞은 횟수. 늘어날 때마다 웹뷰가 방망이를 한 번 휘두른다.
    /// 연타해도 매번 보이려면 상태가 아니라 **횟수**여야 한다.
    pub whack_seq: u64,
    /// 핀볼 모드인가. 웹뷰는 이걸로 **커서를 채로 바꾼다** — 저장소를 다시
    /// 읽게 하면 토글이 즉시 반영되지 않는다.
    pub pinball: bool,
    /// 야차에서 **이 마리가 이번 라운드의 대표 타격인가.** 늘어날 때마다
    /// 웹뷰가 "퍽"을 한 발 낸다.
    ///
    /// **라운드마다 딱 한 마리만 오른다** (판이 고른다). 맞는 마리마다 올리면
    /// 여덟 마리에서 라운드당 네 발이 겹쳐 기관총이 된다. `whack_seq`와 같은
    /// 꼴인 이유는 웹뷰의 소리 판정(`soundsFor`)이 보는 것이 `Snapshot`
    /// 하나뿐이라서다 — 판 스냅샷은 안 본다.
    pub punch_seq: u64,
    /// 그 한 발이 **쓰러뜨린 한 방**인가. 웹뷰가 반음을 낮춰 더 낮고 길게 낸다.
    pub punch_down: bool,
    /// 그 한 발이 **막혔는가.** 화남 표시가 회색으로 작게 뜨고 소리도 둔탁하다.
    ///
    /// **국면으로는 알 수 없다** — 막히면 맞은 쪽이 `Guard` 그대로라 `Behavior`가
    /// 안 바뀐다. 그래서 신호를 따로 싣는다.
    pub punch_blocked: bool,
    pub behavior: Behavior,
}

pub struct Pet {
    x: f64,
    y: f64,
    facing: Facing,
    behavior: Behavior,
    /// 현재 동작이 끝나는 시각. `Dragged`처럼 끝이 없는 동작에서는 무시된다.
    behavior_until_ms: u64,
    last_step_ms: u64,
    /// 마지막 자극(클릭·드래그) 시각 — 졸기 진입 판정의 기준이다.
    last_stimulus_ms: u64,
    /// 직전 유휴 종류 — 같은 동작이 연달아 나오지 않게 한다 (R3).
    last_idle: Option<IdleKind>,
    /// 직전 반응 종류 — 연타할 때 같은 반응만 나오면 심심하다.
    last_sassy: Option<SassyKind>,
    speech: Option<Speech>,
    speech_until_ms: u64,
    speech_seq: u64,
    whack_seq: u64,
    /// 간격이 짧은 클릭이 지금까지 몇 번 이어졌는가. 문턱을 넘으면 빽빽거린다.
    whack_run: u64,
    /// 마지막 빠따 시각. **`last_stimulus_ms`를 쓸 수 없다** — 그쪽은 드래그로도
    /// 갱신되므로 집었다 놓은 것이 연타로 세어진다.
    last_whack_ms: Option<u64>,
    /// 지금 빽빽거리는 판이 끝나는 시각. 0이면 빽빽거리는 중이 아니다.
    squawk_until_ms: u64,
    /// 다음 한마디 시각. 말은 클릭이 아니라 시간에 맞춰 나온다.
    next_taunt_ms: u64,
    /// 좌우 속도 (논리 px/초) — 던져졌을 때만 0이 아니다.
    vx: f64,
    /// 낙하 속도 (논리 px/초).
    vy: f64,
    /// 바닥에서 떠 있는가. 동작만으로 판정하면 공중에서 클릭했을 때
    /// (지상 동작인) 반응 동작이 펭귄을 바닥으로 끌어내린다.
    air: bool,
    /// 헤엄쳐 갈 목적지.
    target: (f64, f64),
    /// 직전 step의 y — 세로 방향(오름/내림)을 이걸로 판정한다.
    last_y: f64,
    /// 지금 헤엄이 **내려앉는 구간인가.**
    swim_descending: bool,
    /// 이번 슬라이딩의 출발 속도 (논리 px/초). 진입할 때 한 번 뽑는다 —
    /// 길이는 고정이고 이 값이 거리를 정한다.
    slide_speed: f64,
    /// 비치발리볼에서 **자기 팀이 뛸 수 있는 x 범위** (좌상단 기준). 판이 목적지를
    /// 자기 코트 안에만 주지만, 그 보장이 마리 쪽에도 있어야 판이 실수해도
    /// 네트를 넘어가는 그림이 안 나온다.
    volley_span: (f64, f64),
    /// 비치발리볼에서 **네트를 보는 방향.** 뛰는 동안에도 이쪽을 본다 — 진행
    /// 방향으로 돌면 옆걸음이 아니라 도망가는 그림이 된다.
    volley_face: Facing,
    /// **핀볼 모드인가.** 켜면 착지 등급 판정을 우회하고 벽·천장·바닥이 전부
    /// 반사면이 된다 (`landing`, `PINBALL_DAMPING`).
    pinball: bool,
    /// 지금 하는 발작 한 판이 끝나는 시각.
    freakout_until_ms: u64,
    /// 지금 하는 얼음낚시 한 판이 끝나는 시각. **절대 시각 하나로 갖는다** —
    /// 국면마다 남은 시간을 빼 나가면 국면이 늘 때마다 계산이 갈라진다.
    fishing_until_ms: u64,
    /// 야차에서 대표 타격으로 뽑힌 횟수 (`Snapshot::punch_seq`).
    punch_seq: u64,
    /// 그 대표 타격이 쓰러뜨린 한 방이었는가 (`Snapshot::punch_down`).
    punch_down: bool,
    /// 그 대표 타격이 막혔는가 (`Snapshot::punch_blocked`).
    punch_blocked: bool,
    rng: u64,
}

/// 펭귄 식별자. 창 라벨(`pet-<id>`)과 짝을 이룬다.
pub type PetId = u32;

/// 한 마리가 **이번 틱에 지나온 자취** — 틱 시작의 몸통 중심과 걸린 시간(초).
///
/// 부딪힘 판정(`Pet::bump_of`)이 `vx`/`vy` 대신 이걸 쓴다. 그 둘은 던져졌을 때만
/// 0이 아니라서, 한 틱 안에서 날아와 착지까지 끝낸 마리는 틱 끝에 속도가 0이고
/// 미끄러지거나 헤엄치는 마리는 처음부터 0이다 — 실제로 화면을 가로질렀는데 판정에는
/// 서 있는 것으로 보인다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::pet) struct Sweep {
    from: (f64, f64),
    seconds: f64,
}

impl Sweep {
    /// 자취가 말해 주는 평균 속도 (논리 px/초). 시간이 0이면 잰 것이 없다.
    fn velocity(self, to: (f64, f64)) -> (f64, f64) {
        if self.seconds <= f64::EPSILON {
            return (0.0, 0.0);
        }
        (
            (to.0 - self.from.0) / self.seconds,
            (to.1 - self.from.1) / self.seconds,
        )
    }
}

/// 동시에 띄울 수 있는 최대 마릿수. 창 하나가 웹뷰 하나이고 각각 수십 MB를 쓴다.
/// 사용자가 **고른** 마릿수를 막지 않되, 실수로 눌러 100마리가 되는 길은 닫는다.
pub const MAX_PETS: usize = 8;

/// 여러 마리를 담는 자리. `BTreeMap`인 이유는 순회 순서가 안정적이어서
/// 틱이 매번 같은 순서로 돌기 때문이다.
#[derive(Default)]
pub struct Pets {
    pets: std::collections::BTreeMap<PetId, Pet>,
    /// **증가만 한다.** 지운 자리의 id를 다시 쓰면, 닫히는 중인 창과 새 창이
    /// 같은 라벨을 다퉈 창 이동이 엉뚱한 쪽으로 간다.
    next_id: PetId,
    /// 지금 도는 볼링 판. 없으면 볼링 중이 아니다. **`Pet`이 아니라 여기가
    /// 소유한다** — 지우기와 판 정합성을 한 자리에서 원자적으로 처리해야 한다 (KTD2).
    bowling: Option<Bowling>,
    /// 지금 도는 비치발리볼 판. 볼링과 같은 이유로 여기가 소유한다.
    /// **둘은 서로를 배제한다** — 동시에 열리면 한쪽이 상대 판의 마리를 끌어간다.
    volleyball: Option<Volleyball>,
    /// 지금 도는 단체 야차 판. 위 둘과 같은 이유로 여기가 소유하고,
    /// **셋이 서로를 배제한다.**
    yacha: Option<Yacha>,
}

/// 야차 판을 못 여는 이유. 버튼이 눌렸는데 아무 일도 안 일어나면 고장으로
/// 읽히므로, 설정 창이 이유를 그대로 보여 준다.
///
/// **비치발리볼과 달리 `Odd`가 없다** — 팀이 없는 난투라 홀수도 정상이다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum YachaRefusal {
    /// 이미 다른 판이 돌고 있다.
    BoardBusy,
    /// 화면이 좁아 링이 안 들어간다.
    NoRoom,
    /// 때릴 상대가 없다 (한 마리).
    TooFew,
}

/// 비치발리볼 판을 못 여는 이유. 버튼이 눌렸는데 아무 일도 안 일어나면 고장으로
/// 읽히므로, 설정 창이 이유를 그대로 보여 준다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VolleyRefusal {
    /// 이미 볼링이나 비치발리볼 판이 돌고 있다.
    BoardBusy,
    /// 화면이 좁아 코트가 안 들어간다.
    NoRoom,
    /// 참여할 마리가 둘이 안 된다.
    TooFew,
    /// 마릿수가 홀수다. **팀이 갈리지 않으면 열지 않는다** (2026-09-02 사용자 지시) —
    /// 한 팀이 한 마리 많으면 그쪽이 덜 뛰고, 그 차이가 "누가 받으러 뛰는가"라는
    /// 이 판의 유일한 볼거리를 한쪽으로 기울인다. 거절하는 편이 정직하다.
    Odd,
}

impl Pets {
    pub fn new() -> Self {
        Pets::default()
    }

    /// 한 마리 추가. 상한에 걸리면 `None`.
    pub fn add(
        &mut self,
        seed_base: u64,
        now_ms: u64,
        world: &World,
        start_x: f64,
    ) -> Option<PetId> {
        if self.pets.len() >= MAX_PETS {
            return None;
        }
        let id = self.next_id.wrapping_add(1);
        self.next_id = id;
        let seed = seed_base ^ u64::from(id).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.pets
            .insert(id, Pet::new_at(seed, now_ms, world, start_x));
        Some(id)
    }

    /// 한 마리 삭제. **마지막 한 마리는 거부한다** — 전부 없애는 것은 on/off의 일이고,
    /// 두 장치가 같은 일을 다투면 안 된다.
    pub fn remove(&mut self, id: PetId) -> bool {
        if self.pets.len() <= 1 {
            return false;
        }
        let removed = self.pets.remove(&id).is_some();
        if removed {
            self.leave_bowling(id);
            self.leave_volleyball(id);
            self.leave_yacha(id);
        }
        removed
    }

    /// 창이 사라진 펭귄을 정리한다. 마지막 한 마리 보호를 받지 않는다 —
    /// 창이 없는 펭귄은 사용자의 선택이 아니라 이미 없어진 것이다.
    pub fn forget(&mut self, id: PetId) {
        self.pets.remove(&id);
        self.leave_bowling(id);
        self.leave_volleyball(id);
        self.leave_yacha(id);
    }

    pub fn get_mut(&mut self, id: PetId) -> Option<&mut Pet> {
        self.pets.get_mut(&id)
    }

    pub fn get(&self, id: PetId) -> Option<&Pet> {
        self.pets.get(&id)
    }

    pub fn ids(&self) -> Vec<PetId> {
        self.pets.keys().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.pets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pets.is_empty()
    }

    /// 전부 비운다 (설정에서 펭귄을 껐을 때). id는 계속 증가하므로 다시 켜도
    /// 닫히는 중인 창과 라벨이 겹치지 않는다.
    pub fn clear(&mut self) {
        self.pets.clear();
        self.bowling = None;
        self.volleyball = None;
        // **야차도 지운다.** 안 지우면 마리가 0이 된 뒤 틱이 `ids.is_empty()`에서
        // 먼저 끊겨 `step_yacha`가 정리할 기회를 못 얻고, `pet_summary().yacha`가
        // 영원히 `true`로 남아 **판 버튼 셋이 계속 잠긴다.**
        self.yacha = None;
    }

    /// 한 마리를 볼링 판에서 뺀다. 마지막 참여 마리가 빠지면 판을 접는다 (R11).
    fn leave_bowling(&mut self, id: PetId) {
        let Some(board) = self.bowling.as_mut() else {
            return;
        };
        board.leave(id);
        if board.is_empty() {
            self.bowling = None;
        }
    }

    /// 지금 도는 판. 브릿지가 공 창을 만들지 정하는 데 쓴다.
    pub fn bowling(&self) -> Option<&Bowling> {
        self.bowling.as_ref()
    }

    /// 볼링 한 판을 연다 — **화면의 펭귄 전부**가 참여한다 (R1). 이미 판이
    /// 도는 중이면 무시한다 (A3). 들려 있는 마리는 빠지고, 아무도 못 서면
    /// 판을 열지 않는다.
    pub fn start_bowling(&mut self, now_ms: u64, lane: Bounds) -> bool {
        // **비치발리볼·야차와 서로를 배제한다.** 셋 다 전 마리를 모는 판이라,
        // 동시에 열리면 이쪽이 남의 판에 선 마리를 핀 자세로 끌어가고 상대 판만
        // 덩그러니 남는다.
        if self.bowling.is_some() || self.volleyball.is_some() || self.yacha.is_some() {
            return false;
        }
        let ids = self.ids();
        let mut pins = std::collections::BTreeMap::new();
        for (id, (pin_x, pin_y)) in ids.iter().zip(pin_positions(ids.len(), lane)) {
            let joined = self
                .pets
                .get_mut(id)
                .is_some_and(|pet| pet.start_bowling(now_ms, pin_x, pin_y));
            if joined {
                pins.insert(*id, (pin_x, pin_y));
            }
        }
        if pins.is_empty() {
            return false;
        }
        self.bowling = Some(Bowling::new(pins, lane, now_ms));
        true
    }

    /// 공을 집는다. 판이 없거나 굴러가는 중이면 `false`.
    pub fn ball_drag_start(&mut self) -> bool {
        self.bowling.as_mut().is_some_and(Bowling::grab)
    }

    /// 집은 공을 가로로 옮긴다. **세로는 받지 않는다** — 조준 각도가 없다 (R6).
    pub fn ball_drag_by(&mut self, dx: f64) {
        if let Some(board) = self.bowling.as_mut() {
            board.drag(dx);
        }
    }

    /// 공을 놓는다. 놓는 순간의 **가로** 속도가 굴러가는 거리를 정한다 (R5).
    pub fn ball_drag_end(&mut self, now_ms: u64, vx: f64) {
        if let Some(board) = self.bowling.as_mut() {
            board.release(now_ms, vx);
        }
    }

    /// 빠따 — 맞은 마리를 때리고, **그 마리가 보는 방향의 사거리 안에 있는 다른
    /// 마리도 함께 날린다.** 겉모습이 바뀐 마리의 id를 전부 돌려주므로 브릿지가
    /// 그 창들을 한꺼번에 옮길 수 있다.
    ///
    /// **여기가 자리인 이유** — `Pet::whack`은 자기 자신만 본다(#44의 이음매
    /// 규칙). 그렇다고 틱(`step_all`)에 얹을 일도 아니다: 넉백은 클릭 한 번의
    /// 결과라 그 순간 한 번만 판정하면 되고, 20Hz로 전 마리를 훑을 이유가 없다.
    ///
    /// **넉백이 나는 조건은 "스윙이 실제로 나왔는가" 하나다.** 빠따는 상황에 따라
    /// 셋으로 갈린다 — 스윙, 연타 빽빽거리기, 핀볼 채(`flip`). 방망이를 휘두르지
    /// 않은 갈래는 이웃도 건드리지 않아야 하는데, 갈래마다 조건을 따로 쓰면 네
    /// 번째가 생겼을 때 조용히 틀린다. 진입 뒤 동작을 한 번 보면 한 줄로 갈린다.
    ///
    /// **연쇄는 없다** — 날아간 이웃은 다시 훑지 않는다. 마리끼리 부딪히는 판정은
    /// 핀볼 쪽에서 양방향으로 따로 설계한다.
    pub fn whack(&mut self, id: PetId, now_ms: u64, world: &World, nx: f64, ny: f64) -> Vec<PetId> {
        let Some(pet) = self.pets.get_mut(&id) else {
            return Vec::new();
        };
        pet.whack(now_ms, world, nx, ny);
        let mut hit = vec![id];
        if pet.behavior != Behavior::Swing {
            return hit;
        }
        // 휘두른 마리의 자리와 방향은 **루프 전에 한 번** 읽는다.
        let (ox, oy) = (pet.center_x(), pet.center_y());
        let forward = match pet.facing {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        };
        let width = world.width();

        for other in self.ids() {
            if other == id {
                continue;
            }
            let Some(target) = self.pets.get_mut(&other) else {
                continue;
            };
            // 손이 잡고 있는 마리는 손이 이기고, 판에 선 마리는 판이 갖는다 —
            // 방망이로 핀을 넘어뜨리면 공이 할 일이 없어진다.
            //
            // **비치발리볼도 같다.** 여기 빠지면 스윙 사거리(200px)가 코트의
            // 이웃 간격(~117px)보다 넓어서, **안 건드린 이웃까지 랠리에서 빠진다.**
            // 두 마리 판(최소 마릿수이자 흔한 경우)에서는 그 한 번이 방금 시작한
            // 20초짜리 판을 즉시 끝낸다. **맞은 당사자는 그대로 빠진다** — 그건
            // 사용자가 겨눈 결과이고 PRD §5.10이 명시한 동작이라 안 건드린다.
            // **야차는 셋 중 가장 위험하다** — 스윙 사거리(200px)가 링의 이웃
            // 간격(96px)의 두 배라, 빠지면 한 번 휘두를 때 링의 절반이 날아간다.
            if target.behavior == Behavior::Dragged
                || target.is_bowling()
                || target.is_volleying()
                || target.is_yachaing()
            {
                continue;
            }
            let 앞으로 = (target.center_x() - ox) * forward;
            if !(0.0..=SWING_REACH).contains(&앞으로) {
                continue;
            }
            if (target.center_y() - oy).abs() > SWING_REACH_V {
                continue;
            }
            target.swing_knocked(now_ms, forward, width);
            hit.push(other);
        }
        hit
    }

    /// 판을 지금 접는다. **코어 밖의 이유로 판을 이어갈 수 없을 때** 쓴다 —
    /// 브릿지가 공 창을 못 만든 경우가 그렇다. 참여 마리는 전부 흩어져
    /// 평소로 돌아간다.
    pub fn end_bowling(&mut self, now_ms: u64) {
        let Self { pets, bowling, .. } = self;
        let Some(board) = bowling.take() else {
            return;
        };
        for id in board.participants() {
            if let Some(pet) = pets.get_mut(&id) {
                pet.bowling_scatter(now_ms);
            }
        }
    }

    /// 볼링 판을 한 틱 진행시킨다. **마리별 `step`보다 먼저** 돈다 — 판이
    /// 마리를 몰지 그 반대가 아니라서, 이번 틱에 정해진 국면이 곧바로 그 틱의
    /// 마리 동작에 반영되어야 한다 (KTD8).
    ///
    /// 여기가 **전 마리를 가로지르는 유일한 자리**다. "공이 지나갔는가"는
    /// 마리 하나의 `step`으로는 답할 수 없다.
    fn step_bowling(&mut self, now_ms: u64) {
        let Self { pets, bowling, .. } = self;
        let Some(board) = bowling.as_mut() else {
            return;
        };

        // 1) 판을 떠난 마리를 추린다. 드래그·빠따로 다른 동작에 넘어갔거나(A4)
        //    사라진(AE4) 마리가 여기서 빠진다. 맞아서 `Thrown`이 된 마리도
        //    여기서 자연히 빠져나간다 — 맞은 상태는 볼링 국면이 아니다.
        for id in board.participants() {
            if !pets.get(&id).is_some_and(Pet::is_bowling) {
                board.leave(id);
            }
        }
        // 착지한 마리는 더는 아무것도 못 친다.
        board.retain_knocked(|id| pets.get(&id).is_some_and(Pet::is_flying));

        // 굴러가는 중에는 핀이 하나도 안 남아도 판을 접지 않는다 — 스트라이크가
        // 나면 공이 아직 날아가는 중인데 창이 닫혀 버린다.
        let 접을까 =
            board.is_empty() && matches!(board.phase(), BoardPhase::Gathering | BoardPhase::Ready);
        if 접을까 {
            *bowling = None;
            return;
        }

        // 2) 판이 통째로 시간을 다 썼으면 접는다. 마리별 안전 상한만으로는
        //    부족하다 — 서로 다른 시각에 만료되면 마지막 한 마리가 빠질 때까지
        //    판과 공 창이 남는다 (R11).
        if board.expired(now_ms) {
            for id in board.participants() {
                if let Some(pet) = pets.get_mut(&id) {
                    pet.bowling_scatter(now_ms);
                }
            }
            *bowling = None;
            return;
        }

        let dt = board.tick(now_ms);
        let world = board.lane_width();
        match board.phase() {
            BoardPhase::Gathering => {
                let 다_섰다 = board
                    .participants()
                    .iter()
                    .all(|id| pets.get(id).is_some_and(Pet::bowling_stood));
                if 다_섰다 {
                    board.open_ball();
                }
            }
            // 사용자가 굴리기 전까지는 아무 일도 없다.
            BoardPhase::Ready => {}
            BoardPhase::Rolling => {
                board.roll(dt);

                // 공이 핀을 친다. **판정도 2차원이다** — 공이 지나는 줄에서 먼
                // 핀은 직접 맞지 않고 아래의 연쇄로만 쓰러진다.
                for id in board.participants() {
                    let Some(pet) = pets.get(&id) else { continue };
                    let at = (pet.center_x(), pet.center_y());
                    let Some((dx, dy)) = board.ball_hit(at.0, at.1) else {
                        continue;
                    };
                    if let Some(pet) = pets.get_mut(&id) {
                        pet.bowling_knocked(now_ms, dx, dy, world);
                    }
                    board.knock(id, at);
                }

                // 튕겨 나간 마리가 아직 선 핀을 친다 — **연쇄**. 이게 없으면
                // 공이 지나는 한 줄만 쓰러져 삼각형을 세운 보람이 없다.
                //
                // **판정은 지나온 구간으로 한다.** 튕기는 속도가 세계 폭에
                // 비례하므로 넓은 화면에서는 한 틱에 140px 넘게 날아, 지금
                // 위치만 보면 이웃 옆을 스쳐 지나가면서도 어느 틱에도 반경
                // 안에 안 잡힌다 (공 판정과 똑같은 함정이다).
                let 반경2 = BOWLING_KNOCK_RADIUS * BOWLING_KNOCK_RADIUS;
                for (hitter, 직전) in board.knocked() {
                    let Some(h) = pets.get(&hitter) else { continue };
                    let 지금 = (h.center_x(), h.center_y());
                    for id in board.participants() {
                        let Some(pet) = pets.get(&id) else { continue };
                        let at = (pet.center_x(), pet.center_y());
                        let (d2, 가까운) = dist2_to_segment(at, 직전, 지금);
                        if d2 > 반경2 {
                            continue;
                        }
                        // 지나간 자리에서 **밀려나는** 방향이다. 정확히 겹쳤으면
                        // 때린 쪽이 가던 방향으로 민다.
                        let (mut dx, mut dy) = (at.0 - 가까운.0, at.1 - 가까운.1);
                        if dx * dx + dy * dy <= f64::EPSILON {
                            dx = 지금.0 - 직전.0;
                            dy = 지금.1 - 직전.1;
                        }
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.bowling_knocked(now_ms, dx, dy, world);
                        }
                        board.knock(id, at);
                    }
                    board.track_knocked(hitter, 지금);
                }

                if board.ball_done() {
                    board.settle(now_ms);
                }
            }
            BoardPhase::Settling => {
                if board.settled(now_ms) {
                    for id in board.participants() {
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.bowling_scatter(now_ms);
                        }
                    }
                    *bowling = None;
                }
            }
        }
    }

    /// 지금 도는 비치발리볼 판. 브릿지가 코트·공 창을 만들지 정하는 데 쓴다.
    pub fn volleyball(&self) -> Option<&Volleyball> {
        self.volleyball.as_ref()
    }

    /// 한 마리를 비치발리볼 판의 참여 목록에서 뺀다.
    ///
    /// **여기서 판을 접지 않는다.** 혼자 남으면 랠리가 성립하지 않지만, 접는
    /// 일에는 **남은 마리를 귀결 국면으로 풀어 주는 일이 딸려 있고** 그건 시각이
    /// 필요하다. `Pets`는 시계를 갖지 않으므로(코어는 시간을 주입받는다) 그
    /// 판단을 시각을 아는 한 곳 — [`step_volleyball`](Self::step_volleyball) —
    /// 에 모은다. 노출은 다음 틱까지(최대 50ms)이고 그동안 판은 한 마리로 산다.
    ///
    /// **판만 버리고 마리를 안 풀면 그 마리가 국면에 갇힌다** — 국면의 시각은
    /// 길이가 아니라 안전 상한(60초)이라, 코트도 공도 사라진 바탕화면에
    /// 훌라 차림 그대로 1분을 서 있게 된다.
    fn leave_volleyball(&mut self, id: PetId) {
        if let Some(board) = self.volleyball.as_mut() {
            board.leave(id);
        }
    }

    /// 비치발리볼 한 판을 연다 — **화면의 펭귄 전부**가 참여한다 (R2).
    ///
    /// 거절하는 경우 셋: 이미 어느 판이든 돌고 있다(KTD9), 코트가 화면에 안
    /// 들어간다, 참여 마리가 둘 미만이다(R3). `seed`는 랠리의 원천이라 브릿지가
    /// `now_ms()`를, 테스트가 고정값을 넘긴다.
    pub fn start_volleyball(
        &mut self,
        now_ms: u64,
        bounds: Bounds,
        seed: u64,
    ) -> Result<(), VolleyRefusal> {
        if self.volleyball.is_some() || self.bowling.is_some() || self.yacha.is_some() {
            return Err(VolleyRefusal::BoardBusy);
        }
        let Some(court) = Court::new(bounds) else {
            return Err(VolleyRefusal::NoRoom);
        };
        let ids = self.ids();
        if ids.len() < VOLLEY_MIN_PETS {
            return Err(VolleyRefusal::TooFew);
        }
        // **홀수면 열지 않는다.** `TooFew`보다 뒤에 본다 — 한 마리일 때는 "홀수라서"가
        // 아니라 "둘이 안 돼서"가 사용자에게 맞는 설명이다.
        if ids.len() % 2 != 0 {
            return Err(VolleyRefusal::Odd);
        }

        let sides = assign_sides(ids.len());
        let 좌수 = sides.iter().filter(|s| **s == Side::Left).count();
        let 우수 = ids.len() - 좌수;
        let (mut 좌, mut 우) = (0usize, 0usize);
        let mut players: std::collections::BTreeMap<PetId, Side> =
            std::collections::BTreeMap::new();
        for (id, side) in ids.iter().zip(sides) {
            let (k, total) = if side == Side::Left {
                좌 += 1;
                (좌 - 1, 좌수)
            } else {
                우 += 1;
                (우 - 1, 우수)
            };
            let spot = court.spot_of(side, k, total);
            let (lo, hi) = court.span_of(side);
            // 마리가 쓰는 좌표는 좌상단이라 몸통 가운데 범위를 옮겨 넘긴다.
            let span = (lo - PET_SIZE / 2.0, hi - PET_SIZE / 2.0);
            // 네트를 본다 — 왼쪽 팀은 오른쪽을, 오른쪽 팀은 왼쪽을.
            let face = if side == Side::Left {
                Facing::Right
            } else {
                Facing::Left
            };
            let joined = self
                .pets
                .get_mut(id)
                .is_some_and(|pet| pet.start_volley(now_ms, spot, span, face));
            if joined {
                players.insert(*id, side);
            }
        }
        // 들려 있는 마리가 빠져 둘이 안 되거나 **한 팀이 통째로 비면** 판을 안
        // 연다. 마릿수만 세면 세 마리 중 오른쪽 하나를 들고 있는 경우를 놓치는데,
        // 그러면 서브 한 번에 공이 빈 코트로 떨어져 "2초짜리 판"이 된다.
        if players.len() < VOLLEY_MIN_PETS || !both_sides_present(&players) {
            for id in players.keys() {
                if let Some(pet) = self.pets.get_mut(id) {
                    pet.volley_finish(now_ms, true);
                }
            }
            return Err(VolleyRefusal::TooFew);
        }
        self.volleyball = Some(Volleyball::new(players, court, now_ms, seed));
        Ok(())
    }

    /// 판을 지금 접는다. **코어 밖의 이유로 판을 이어갈 수 없을 때** 쓴다 —
    /// 브릿지가 코트나 공 창을 못 만든 경우가 그렇다.
    pub fn end_volleyball(&mut self, now_ms: u64) {
        let Self {
            pets, volleyball, ..
        } = self;
        let Some(board) = volleyball.take() else {
            return;
        };
        for id in board.participants() {
            if let Some(pet) = pets.get_mut(&id) {
                // 아무도 안 졌다 — 판이 그냥 사라진 것이라 전부 같은 그림으로 끝낸다.
                pet.volley_finish(now_ms, true);
            }
        }
    }

    /// 비치발리볼 판을 한 틱 진행시킨다. **마리별 `step`보다 먼저** 돈다 —
    /// 판이 마리를 몰지 그 반대가 아니라서, 이번 틱에 정해진 국면이 곧바로
    /// 그 틱의 마리 동작에 반영되어야 한다 (볼링과 같은 규칙).
    // ── 단체 야차 ────────────────────────────────────────

    pub fn yacha(&self) -> Option<&Yacha> {
        self.yacha.as_ref()
    }

    /// 판이 도는 중에 마리가 사라졌다. **판을 여기서 닫지 않는다** — 닫을지는
    /// 시계가 있어야 정할 수 있고 `Pets`에는 없다. 판단은 `step_yacha` 한 곳에
    /// 모은다 (발리볼과 같은 규칙).
    pub fn leave_yacha(&mut self, id: PetId) {
        if let Some(board) = self.yacha.as_mut() {
            board.leave(id);
        }
    }

    /// 단체 야차 한 판을 연다. 화면의 펭귄 **전부**가 참여한다.
    pub fn start_yacha(
        &mut self,
        now_ms: u64,
        bounds: Bounds,
        seed: u64,
    ) -> Result<(), YachaRefusal> {
        if self.yacha.is_some() || self.bowling.is_some() || self.volleyball.is_some() {
            return Err(YachaRefusal::BoardBusy);
        }
        let Some(arena) = Arena::new(bounds) else {
            return Err(YachaRefusal::NoRoom);
        };
        let ids = self.ids();
        if ids.len() < YACHA_MIN_PETS {
            return Err(YachaRefusal::TooFew);
        }

        let board = Yacha::new(ids, arena, now_ms, seed);
        let 참가: Vec<PetId> = board
            .participants()
            .into_iter()
            .filter(|id| {
                let Some((at, _, face)) = board.pose_of(*id) else {
                    return false;
                };
                self.pets
                    .get_mut(id)
                    .is_some_and(|pet| pet.start_yacha(now_ms, at, face))
            })
            .collect();

        // 들려 있는 마리가 빠져 둘이 안 되면 판을 안 연다. 붙인 것은 되돌린다.
        if 참가.len() < YACHA_MIN_PETS {
            for id in &참가 {
                if let Some(pet) = self.pets.get_mut(id) {
                    pet.leave_ring(now_ms, bounds);
                }
            }
            return Err(YachaRefusal::TooFew);
        }

        let mut board = board;
        for id in board.participants() {
            if !참가.contains(&id) {
                board.leave(id);
            }
        }
        self.yacha = Some(board);
        Ok(())
    }

    /// 판을 밖에서 끝낸다 (브릿지가 창을 못 만들었을 때).
    pub fn end_yacha(&mut self, now_ms: u64, bounds: Bounds) {
        let Some(board) = self.yacha.take() else {
            return;
        };
        for id in board.participants() {
            if let Some(pet) = self.pets.get_mut(&id) {
                pet.leave_ring(now_ms, bounds);
            }
        }
    }

    /// 판을 한 틱 굴린다. **판이 마리를 몰지 그 반대가 아니다.**
    fn step_yacha(&mut self, now_ms: u64) {
        let Self { pets, yacha, .. } = self;
        let Some(board) = yacha.as_mut() else {
            return;
        };

        // 1) 판을 떠난 마리를 추린다 (드래그·빠따·삭제).
        //
        // **`in_yacha`로 본다 — `is_yachaing`이 아니다.** 그쪽은 `Champ`을 빼는데,
        // 여기서 그걸 쓰면 벨트를 찬 챔피언이 판에서 쫓겨나 아무도 안 풀려난다.
        let 참여 = board.participants();
        for id in 참여 {
            if !pets.get(&id).is_some_and(Pet::in_yacha) {
                board.leave(id);
            }
        }

        let bounds = board.arena().bounds();
        let 세레모니 = !matches!(board.phase(), RingPhase::Gathering | RingPhase::Brawl);

        // 2) 아무도 안 남았으면 접는다. **세레모니 중에는 안 접는다** — 그때는
        //    챔피언이 `Champ`이라 참여자에서 빠지므로, 접었다가는 벨트 수여가
        //    나오기 전에 미녀가 사라진다.
        if (!세레모니 && board.standing().is_empty()) || board.expired(now_ms) {
            for id in board.participants() {
                if let Some(pet) = pets.get_mut(&id) {
                    pet.leave_ring(now_ms, bounds);
                }
            }
            *yacha = None;
            return;
        }

        match board.phase() {
            RingPhase::Gathering => {
                let 다_섰다 = board
                    .participants()
                    .iter()
                    .all(|id| pets.get(id).is_some_and(Pet::yacha_stood));
                if 다_섰다 || board.phase_over(now_ms) {
                    board.begin_brawl(now_ms);
                }
            }
            RingPhase::Brawl => {
                board.step_brawl(now_ms);

                // **대표 타격 하나만 소리를 낸다** (KTD7). 한 걸음에 주먹이 여럿
                // 나도 발수는 하나다 — 여덟 마리에서 겹치면 기관총이 된다.
                //
                // 신호는 **맞은 쪽**에 실린다. 막힘 여부까지 함께 싣는 이유는
                // 국면으로는 알 수 없어서다 — 막히면 맞은 쪽이 `Guard` 그대로라
                // `Behavior`가 안 바뀐다.
                let punches: Vec<_> = board.punches().to_vec();
                if let Some(p) = punches.iter().find(|p| !p.blocked).or(punches.first()) {
                    if let Some(pet) = pets.get_mut(&p.to) {
                        pet.yacha_thud(false, p.blocked);
                    }
                }

                // **쓰러뜨린 한 방은 따로 낸다** — 반음이 낮고 길다. 다운은 누적
                // 피격이 정하므로 이번 걸음의 주먹과 짝이 안 맞을 수 있고, 그래서
                // 판이 "이번에 누가 넘어갔나"를 따로 알려 준다.
                for id in board.downed_now() {
                    if let Some(pet) = pets.get_mut(&id) {
                        pet.yacha_thud(true, false);
                    }
                }

                // 자리와 자세를 마리에게 받아 적힌다.
                for id in board.participants() {
                    if let Some((at, phase, face)) = board.pose_of(id) {
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.yacha_apply(now_ms, at, phase, face);
                        }
                    }
                }

                // **예산이 다 되어야 끝난다.** 마지막 다운이 일찍 나도 난투는
                // 14초를 채운다 — 그래야 한 판 길이가 마릿수와 무관하다.
                // 그 사이 챔피언은 혼자 `Idle`이라 허공에 주먹을 안 내지른다.
                let up = board.standing();
                if board.phase_over(now_ms) {
                    if let Some(champ) = up.first().copied() {
                        board.crown(now_ms, champ);
                        if let Some(pet) = pets.get_mut(&champ) {
                            pet.yacha_win(now_ms);
                        }
                    }
                }
            }
            _ => {
                let 이전 = board.phase();
                board.step_ceremony(now_ms, 0.05);
                // 벨트를 채우고 나면 챔피언이 세레모니 자세로 넘어간다.
                if 이전 != RingPhase::Ceremony && board.phase() == RingPhase::Ceremony {
                    if let Some(champ) = board.champion() {
                        if let Some(pet) = pets.get_mut(&champ) {
                            pet.yacha_champ(now_ms);
                        }
                    }
                }
                // 쓰러진 놈들은 세레모니 내내 그 자리에 누워 있는다.
                for id in board.participants() {
                    if board.champion() == Some(id) {
                        continue;
                    }
                    if let Some((at, phase, face)) = board.pose_of(id) {
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.yacha_apply(now_ms, at, phase, face);
                        }
                    }
                }
                if board.phase() == RingPhase::Done {
                    for id in board.participants() {
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.leave_ring(now_ms, bounds);
                        }
                    }
                    *yacha = None;
                }
            }
        }
    }

    fn step_volleyball(&mut self, now_ms: u64) {
        let Self {
            pets, volleyball, ..
        } = self;
        let Some(board) = volleyball.as_mut() else {
            return;
        };

        // 1) 판을 떠난 마리를 추린다. 드래그·빠따로 다른 동작에 넘어갔거나
        //    사라진 마리가 여기서 빠진다.
        for id in board.participants() {
            if !pets.get(&id).is_some_and(Pet::is_volleying) {
                board.leave(id);
            }
        }

        // 2) 둘이 안 되면 접는다 — 혼자서는 랠리가 성립하지 않는다.
        //    **득점 중에는 참여자가 0이어도 접지 않는다**: 그 순간 전원이
        //    `Cheer`/`Sulk`로 넘어가므로, 접었다가는 축하 그림이 나오기 전에
        //    코트가 사라진다 (볼링이 "굴러가는 중에는 안 접는다"고 한 자리다).
        if board.phase() != CourtPhase::Point && board.player_count() < VOLLEY_MIN_PETS {
            // **남은 마리를 반드시 풀어 준다.** 판만 버리면 그 마리가 국면에
            // 갇히는데, 국면의 시각은 길이가 아니라 안전 상한(60초)이라
            // 코트가 사라진 바탕화면에 훌라 차림 그대로 1분을 서 있게 된다.
            for id in board.participants() {
                if let Some(pet) = pets.get_mut(&id) {
                    pet.volley_finish(now_ms, true);
                }
            }
            *volleyball = None;
            return;
        }

        // 3) 판이 통째로 시간을 다 썼으면 접는다.
        if board.expired(now_ms) {
            for id in board.participants() {
                if let Some(pet) = pets.get_mut(&id) {
                    pet.volley_finish(now_ms, true);
                }
            }
            *volleyball = None;
            return;
        }

        let dt = board.tick(now_ms);
        match board.phase() {
            CourtPhase::Gathering => {
                let 다_섰다 = board
                    .participants()
                    .iter()
                    .all(|id| pets.get(id).is_some_and(Pet::volley_stood));
                if 다_섰다 {
                    // 서브 — 자기에게 보내는 왕복 0번이다.
                    let 위치 = 코트_위치(board, pets);
                    board.serve(now_ms, &위치);
                    if let Some((rid, tcx)) = board.receiver() {
                        if let Some(pet) = pets.get_mut(&rid) {
                            pet.volley_chase(now_ms, tcx - PET_SIZE / 2.0);
                        }
                    }
                }
            }
            CourtPhase::Rally => {
                board.step_ball(dt);

                // 받을 마리가 지금 공을 치는가. **여기가 전 마리를 가로지르는
                // 자리다** — "이 공이 누구에게 가는가"는 마리 하나의 `step`으로
                // 답할 수 없다.
                if let Some((rid, _)) = board.receiver() {
                    let 맞았나 = pets
                        .get(&rid)
                        .is_some_and(|pet| board.contact_at(pet.center_x()));
                    if 맞았나 {
                        let to = board.next_side();
                        let 상대: Vec<(PetId, f64)> = board
                            .ids_on(to)
                            .into_iter()
                            .filter_map(|id| pets.get(&id).map(|p| (id, p.center_x())))
                            .collect();
                        board.hit(now_ms, &상대);
                        if let Some(pet) = pets.get_mut(&rid) {
                            pet.volley_bump(now_ms);
                        }
                        if let Some((nid, tcx)) = board.receiver() {
                            if let Some(pet) = pets.get_mut(&nid) {
                                pet.volley_chase(now_ms, tcx - PET_SIZE / 2.0);
                            }
                        }
                    }
                }

                if board.landed() {
                    let 진_팀 = board.loser();
                    for id in board.participants() {
                        let 이겼나 = board.side_of(id) != Some(진_팀);
                        if let Some(pet) = pets.get_mut(&id) {
                            pet.volley_finish(now_ms, 이겼나);
                        }
                    }
                    board.settle(now_ms);
                }
            }
            CourtPhase::Point => {
                if board.settled(now_ms) {
                    *volleyball = None;
                }
            }
        }
    }

    /// 한 틱 동안 전 마리를 진행시킨다.
    ///
    /// **마리별 `Pet::step`은 자기 자신만 본다.** 여러 마리가 하나의 사건을 공유하는
    /// 판정(공에 맞았는가, 옆 마리와 부딪혔는가)은 마리 단위로는 답할 수 없으므로,
    /// 전 마리를 가로지르는 자리가 하나 있어야 한다. 그 자리가 여기다 —
    /// **`Pet::step`의 시그니처를 건드리지 않고** 그 위에 계층을 얹는다
    /// (인자로 받으면 모든 모션 함수와 테스트를 고치는 대공사가 된다).
    ///
    /// 순회는 `BTreeMap` 순서, 즉 id 오름차순이다. 순서가 틱마다 달라지면
    /// 창 이동이 적용되는 차례가 흔들린다.
    ///
    /// `world_of`가 `None`을 주는 마리는 이번 틱을 쉰다 — 창이 없거나 경계를
    /// 아직 못 읽은 경우다. 세계 조회가 브릿지 쪽 캐시에 있으므로 맵이 아니라
    /// 클로저로 받는다.
    ///
    /// **`world_of`는 호출자가 `Pets`를 잠근 채로 불린다.** 그 안에서 같은 잠금을
    /// 다시 잡으면 즉시 자기 데드락이다 — 브릿지처럼 스레드 로컬 캐시만 읽어라.
    pub fn step_all<'w>(
        &mut self,
        now_ms: u64,
        world_of: impl Fn(PetId) -> Option<&'w World>,
    ) -> Vec<(PetId, Snapshot)> {
        self.step_bowling(now_ms);
        self.step_volleyball(now_ms);
        self.step_yacha(now_ms);
        let mut stepped = Vec::with_capacity(self.pets.len());
        // 부딪힘 판정이 볼 **이번 틱의 자취**와 세계 폭. `Pet`에 필드로 넣지 않고 여기서
        // 들고 있는다 — 필드를 더하면 스냅샷 동치성이 보는 상태가 늘어난다.
        let mut before: Vec<(PetId, Sweep, f64)> = Vec::with_capacity(self.pets.len());
        for id in self.ids() {
            let Some(world) = world_of(id) else {
                continue;
            };
            let Some(pet) = self.pets.get_mut(&id) else {
                continue;
            };
            before.push((id, pet.sweep_from(now_ms), world.width()));
            stepped.push((id, pet.step(now_ms, world)));
        }
        // **핀볼일 때만 돈다.** 꺼져 있으면 여기 오기 전과 한 줄도 다르지 않아야 한다
        // (`여러_마리를_한_번에_돌려도_따로_돌린_것과_같다`가 그걸 본다).
        //
        // **볼링 판이 도는 동안에는 쉰다.** 두 기능은 서로를 끄지 않으므로 동시에
        // 켜질 수 있는데, 그러면 같은 마리를 두 물리가 두고 다툰다: 부딪혀 `Thrown`이
        // 된 핀은 `board.knock`을 거치지 않고 판에서 빠져 **볼링 연쇄가 조용히
        // 끊기고**, 판이 튕긴 핀은 여기서 던지기 상한으로 도로 깎인다. 판이 도는 동안
        // 핀은 판이 몬다 (`step_bowling`의 KTD8과 같은 규칙).
        //
        // **비치발리볼도 같다.** 충돌 반경(104px)이 여덟 마리 코트의 이웃
        // 간격(117px)보다 좁고 받을 마리가 그 사이를 뛰어 지나가므로, 안 쉬면
        // `bumped`가 `Thrown`으로 넘겨 **랠리가 0.4초 만에 찢어진다**(측정값).
        // 핀볼은 모드라 KTD9의 상호 배제로는 못 풀고 이 가드로 푼다 — 켜 둔 채
        // 판을 열 수 있어야 하고, 판이 끝나면 저절로 돌아온다.
        // **야차도 같다.** 오히려 가장 위험하다 — 이웃 간격(96px)이 충돌
        // 반경보다 좁아 링에 서기만 해도 서로를 튕긴다. "서로 튕겨나가지
        // 않는다"가 이 동작의 정의(R7)라 여기가 그 마지막 문이다.
        if before.len() >= 2
            && self.bowling.is_none()
            && self.volleyball.is_none()
            && self.yacha.is_none()
            && self.pets.values().any(Pet::pinball)
        {
            let bumped = self.collide_pinball(now_ms, &before);
            for (id, snapshot) in stepped.iter_mut() {
                if !bumped.contains(id) {
                    continue;
                }
                if let Some(pet) = self.pets.get(id) {
                    *snapshot = pet.snapshot();
                }
            }
        }
        stepped
    }

    /// 핀볼에서 마리끼리 부딪힌 것을 해소한다 — **`step` 뒤에** 돈다. 이번 틱에 지나온
    /// 경로를 봐야 빠른 마리가 서로를 뛰어넘지 않는다 (`bump_of`).
    ///
    /// 판정은 **양방향**이다. 볼링 연쇄는 *튕겨 나간 마리 → 아직 선 마리* 한 방향이고
    /// 맞은 쪽만 속도를 받지만, 여기서는 둘 다 움직이고 둘 다 속도가 바뀐다.
    ///
    /// 쌍은 최대 `MAX_PETS * (MAX_PETS - 1) / 2 = 28`개이고 쌍마다 드는 것은 부동소수
    /// 산술 몇십 번뿐이다 — IPC도 할당도 시스템 호출도 없어서 O(n²)이 50ms 틱에서
    /// 문제가 되지 않는다.
    ///
    /// **임펄스는 쌓이지만 판정은 안 쌓인다.** `bumped`가 속도를 더해 나가는 반면
    /// `bump_of`는 `step` **이전**의 자취(`Sweep`)로 재므로, 앞선 쌍이 준 임펄스가
    /// 뒤 쌍의 접근 판정을 바꾸지 않는다. 이 비대칭이 한 틱 안에서 같은 접근을 두 번
    /// 세지 않게 한다. `before`가 id 오름차순이라 순서는 매 틱 같다.
    ///
    /// **난수를 쓰지 않는다** — 골든 수열을 재기준화하지 않기 위한 제약이다.
    fn collide_pinball(&mut self, now_ms: u64, before: &[(PetId, Sweep, f64)]) -> Vec<PetId> {
        let mut bumped = Vec::new();
        for i in 0..before.len() {
            for j in (i + 1)..before.len() {
                let (a_id, a_sweep, a_world) = before[i];
                let (b_id, b_sweep, b_world) = before[j];
                let Some((a, b)) = self.pets.get(&a_id).zip(self.pets.get(&b_id)) else {
                    continue;
                };
                let Some(((nx, ny), impulse)) = a.bump_of(a_sweep, b, b_sweep) else {
                    continue;
                };
                // 법선만 뒤집어 같은 함수를 쓴다 — 대칭을 코드 모양으로 못 박아
                // 한쪽만 움직이는 실수를 막는다.
                if let Some(pet) = self.pets.get_mut(&a_id) {
                    pet.bumped(now_ms, nx, ny, impulse, a_world);
                }
                if let Some(pet) = self.pets.get_mut(&b_id) {
                    pet.bumped(now_ms, -nx, -ny, impulse, b_world);
                }
                bumped.push(a_id);
                bumped.push(b_id);
            }
        }
        bumped
    }
}

/// 판에 선 마리들의 (id, 팀, **몸통 가운데 x**). 서브할 마리를 고르는 데 쓴다 —
/// 판은 자리를 알지만 마리가 지금 어디 있는지는 모른다 (뛰는 중일 수 있다).
fn 코트_위치(
    board: &Volleyball,
    pets: &std::collections::BTreeMap<PetId, Pet>,
) -> Vec<(PetId, Side, f64)> {
    board
        .participants()
        .into_iter()
        .filter_map(|id| Some((id, board.side_of(id)?, pets.get(&id)?.center_x())))
        .collect()
}

impl Pet {
    /// 시드는 0이면 안 된다 (xorshift가 0에 갇힌다) — 0이 들어오면 대체한다.
    pub fn new(seed: u64, start_ms: u64, world: &World) -> Self {
        let start_x = world.first().bounds.left;
        Pet::new_at(seed, start_ms, world, start_x)
    }

    /// 시작 x를 지정해 만든다. 새로 부른 펭귄이 부른 펭귄 옆에서 나타나게 하려고
    /// 쓴다 — 전부 같은 자리에서 시작하면 겹쳐서 한 마리로 보인다.
    pub fn new_at(seed: u64, start_ms: u64, world: &World, start_x: f64) -> Self {
        let bounds = world.screen_for_x(start_x).bounds;
        let x = start_x.clamp(bounds.left, bounds.right.max(bounds.left));
        let mut pet = Pet {
            x,
            y: bounds.floor_y,
            facing: Facing::Right,
            behavior: Behavior::Walk,
            behavior_until_ms: start_ms + WALK_MS.0,
            last_step_ms: start_ms,
            last_stimulus_ms: start_ms,
            last_idle: None,
            last_sassy: None,
            speech: None,
            speech_until_ms: 0,
            speech_seq: 0,
            whack_seq: 0,
            whack_run: 0,
            last_whack_ms: None,
            squawk_until_ms: 0,
            next_taunt_ms: start_ms + TAUNT_GAP_MS.0,
            vx: 0.0,
            vy: 0.0,
            air: false,
            target: (x, bounds.floor_y),
            last_y: bounds.floor_y,
            slide_speed: 0.0,
            volley_span: (x, x),
            volley_face: Facing::Right,
            pinball: false,
            swim_descending: false,
            freakout_until_ms: 0,
            fishing_until_ms: 0,
            punch_seq: 0,
            punch_down: false,
            punch_blocked: false,
            rng: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        };
        pet.next_taunt_ms = start_ms + pet.range(TAUNT_GAP_MS);
        pet
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            x: self.x,
            y: self.y,
            facing: self.facing,
            vertical: self.vertical(),
            air: self.air,
            speech: self.speech,
            whack_seq: self.whack_seq,
            pinball: self.pinball,
            punch_seq: self.punch_seq,
            punch_down: self.punch_down,
            punch_blocked: self.punch_blocked,
            behavior: self.behavior,
        }
    }

    /// 세로 방향은 직전 step 대비 y 변화로 정한다 — 헤엄·낙하·던지기가
    /// 각자 다른 속도 필드를 쓰므로 위치 변화가 유일하게 공통된 기준이다.
    fn vertical(&self) -> Vertical {
        if !self.air {
            return Vertical::Level;
        }
        let dy = self.y - self.last_y;
        if dy < -0.5 {
            Vertical::Up
        } else if dy > 0.5 {
            Vertical::Down
        } else {
            Vertical::Level
        }
    }

    #[cfg(test)]
    pub(in crate::pet) fn behavior_until_for_test(&self) -> u64 {
        self.behavior_until_ms
    }

    pub fn behavior(&self) -> Behavior {
        self.behavior
    }

    /// 몸통 가운데. `x`/`y`는 왼쪽 위 모서리라 마리끼리·공과의 거리를 잴 때는
    /// 이쪽을 써야 크기(`PET_SIZE`)만큼 어긋나지 않는다.
    pub(in crate::pet) fn center_x(&self) -> f64 {
        self.x + PET_SIZE / 2.0
    }

    pub(in crate::pet) fn center_y(&self) -> f64 {
        self.y + PET_SIZE / 2.0
    }

    /// 이번 틱의 자취를 연다 — **`step`을 부르기 직전에** 잡아야 한다.
    /// 걸린 시간은 `step`이 쓰는 것과 같은 규칙으로 자른다 (`MAX_STEP_MS`).
    pub(in crate::pet) fn sweep_from(&self, now_ms: u64) -> Sweep {
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        Sweep {
            from: (self.center_x(), self.center_y()),
            seconds: elapsed as f64 / 1000.0,
        }
    }

    /// 판정의 기준점 — **발밑 중앙**이다 (PRD §5.2).
    fn anchor(&self) -> (f64, f64) {
        (self.x + PET_SIZE / 2.0, self.y + PET_SIZE)
    }

    /// 지금 발을 딛고 있는 화면의 이동 영역. 어느 화면에도 없으면 가장 가까운
    /// 화면을 쓴다 — 좌표가 잠깐 어긋나도 판정이 멈추지 않아야 한다.
    fn bounds_in(&self, world: &World) -> Bounds {
        let (ax, ay) = self.anchor();
        world
            .screen_at(ax, ay)
            .unwrap_or_else(|| world.nearest(ax, ay))
            .bounds
    }

    /// 시간을 진행시키고 현재 상태를 돌려준다. 브릿지가 매 틱 호출한다.
    pub fn step(&mut self, now_ms: u64, world: &World) -> Snapshot {
        let bounds = self.bounds_in(world);
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        self.last_step_ms = now_ms;
        let dt = elapsed as f64 / 1000.0;
        self.last_y = self.y;
        if self.behavior.silences_speech() {
            // **판이 도는 동안은 조용하다.** 떠 있던 말풍선도 지운다 — 안 지우면
            // 집결 중에 하나가 남아 화면 가운데에 떠 있다.
            self.speech = None;
            // **밀린 대사를 몰아서 뱉지 않는다.** 판이 끝나는 순간 한마디가
            // 튀어나오지 않게 다음 시각을 앞으로 민다. 난수를 안 쓰므로 판에
            // 참여한 마리의 동작 시퀀스가 밀리지 않는다.
            if now_ms >= self.next_taunt_ms {
                self.next_taunt_ms = now_ms + SPEECH_MS;
            }
        } else {
            if self.speech.is_some() && now_ms >= self.speech_until_ms {
                self.speech = None;
            }
            if self.speech.is_none() && now_ms >= self.next_taunt_ms {
                self.say(now_ms);
                let gap = self.range(TAUNT_GAP_MS);
                self.next_taunt_ms = now_ms + SPEECH_MS + gap;
            }
        }

        match self.behavior {
            Behavior::Dragged => {}
            Behavior::Swing => self.tick_swing(now_ms),
            Behavior::Sassy { .. } => self.tick_sassy(now_ms),
            Behavior::Squawk => self.tick_squawk(now_ms),
            Behavior::DontAsk => self.tick_dont_ask(now_ms),
            Behavior::Falling => self.tick_falling(now_ms, bounds, dt),
            Behavior::Swim => self.tick_swim(now_ms, bounds, dt),
            Behavior::Thrown => self.tick_thrown(now_ms, bounds, dt),
            Behavior::Land | Behavior::Splat | Behavior::Sprawl => self.tick_landed(now_ms),
            Behavior::Walk => self.tick_walk(now_ms, bounds, dt),
            Behavior::Turn => self.tick_turn(now_ms),
            Behavior::Slide => self.tick_slide(now_ms, bounds, dt),
            Behavior::Tumble => self.tick_tumble(now_ms, dt),
            Behavior::Freakout { freakout } => self.tick_freakout(now_ms, freakout, bounds, dt),
            Behavior::IceFishing { fishing } => self.tick_fishing(now_ms, fishing),
            Behavior::Bowling { bowling } => self.tick_bowling(now_ms, bowling, bounds, dt),
            Behavior::Volleyball { volley } => self.tick_volley(now_ms, volley, bounds, dt),
            Behavior::Yacha { yacha } => self.tick_yacha(now_ms, yacha, bounds, dt),
            Behavior::Idle { .. } | Behavior::Sleep => {
                if now_ms >= self.behavior_until_ms {
                    self.pick_next(now_ms, bounds);
                }
            }
        }

        self.clamp(bounds);
        self.snapshot()
    }

    /// 킹받는 한마디를 띄운다. 문구는 웹뷰가 고른다.
    pub fn say(&mut self, now_ms: u64) {
        self.speech_seq += 1;
        let roll = self.next_u64() % 100_000;
        self.speech = Some(Speech {
            seq: self.speech_seq,
            roll,
        });
        self.speech_until_ms = now_ms + SPEECH_MS;
    }

    fn enter(&mut self, behavior: Behavior, until_ms: u64) {
        if !matches!(behavior, Behavior::Squawk | Behavior::Dragged) {
            self.squawk_until_ms = 0;
        }
        match behavior {
            Behavior::Sassy { .. }
            | Behavior::Dragged
            | Behavior::Swing
            | Behavior::Squawk
            | Behavior::DontAsk
            | Behavior::IceFishing { .. } => {}
            Behavior::Land | Behavior::Splat | Behavior::Sprawl | Behavior::Tumble => {
                self.air = false
            }
            other => self.air = other.is_airborne(),
        }
        self.behavior = behavior;
        self.behavior_until_ms = until_ms;
    }

    /// 유휴 동작 하나를 고른다 — 직전과 같은 종류는 피한다 (R3).
    fn enter_idle(&mut self, now_ms: u64) {
        let mut idle = IDLE_KINDS[self.range((0, 3)) as usize];
        if Some(idle) == self.last_idle {
            let next = (IDLE_KINDS.iter().position(|k| *k == idle).unwrap() + 1) % IDLE_KINDS.len();
            idle = IDLE_KINDS[next];
        }
        self.last_idle = Some(idle);
        let until = now_ms + self.range(IDLE_MS);
        self.enter(Behavior::Idle { idle }, until);
    }

    /// 동작이 끝났을 때 다음 동작을 고른다.
    fn pick_next(&mut self, now_ms: u64, bounds: Bounds) {
        if now_ms.saturating_sub(self.last_stimulus_ms) >= SLEEP_AFTER_MS
            && self.behavior != Behavior::Sleep
        {
            let until = now_ms + self.range(SLEEP_MS);
            self.enter(Behavior::Sleep, until);
            return;
        }
        if self.behavior == Behavior::Sleep {
            self.last_stimulus_ms = now_ms;
            self.last_idle = Some(IdleKind::Stretch);
            let until = now_ms + self.range(IDLE_MS);
            self.enter(
                Behavior::Idle {
                    idle: IdleKind::Stretch,
                },
                until,
            );
            return;
        }
        if !self.air && self.range((0, FREAKOUT_ONE_IN - 1)) == 0 {
            self.enter_freakout(now_ms);
            return;
        }
        if !self.air && self.range((0, 999)) < ICE_FISHING_PERMILLE {
            self.enter_ice_fishing(now_ms);
            return;
        }
        if matches!(self.behavior, Behavior::Walk)
            && bounds.right > bounds.left
            && self.range((0, 99)) < SLIDE_AFTER_WALK_PERCENT
        {
            self.enter_slide(now_ms);
            return;
        }
        if bounds.floor_y - bounds.top > 1.0 && self.range((0, 99)) < SWIM_PERCENT {
            self.enter_swim(now_ms, bounds);
            return;
        }
        if matches!(self.behavior, Behavior::Walk) {
            self.enter_idle(now_ms);
        } else if self.range((0, 99)) < WALK_AGAIN_PERCENT {
            let until = now_ms + self.range(WALK_MS);
            self.enter(Behavior::Walk, until);
        } else {
            self.enter_idle(now_ms);
        }
    }

    fn clamp(&mut self, bounds: Bounds) {
        if self.behavior == Behavior::Dragged {
            return;
        }
        self.x = self.x.clamp(bounds.left, bounds.right.max(bounds.left));
        if self.air {
            self.y = self.y.clamp(bounds.top.min(bounds.floor_y), bounds.floor_y);
        } else {
            self.y = bounds.floor_y;
        }
    }

    /// xorshift64 — 테스트에서 시퀀스를 재현하기 위해 코어가 난수를 소유한다.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    /// 0.0 이상 1.0 미만의 실수 하나 — 목적지를 고르는 데 쓴다.
    fn fraction(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `lo..=hi` 범위의 값 하나.
    fn range(&mut self, (lo, hi): (u64, u64)) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "yacha_pets_tests.rs"]
mod yacha_pets_tests;

#[cfg(test)]
#[path = "speech_tests.rs"]
mod speech_tests;
