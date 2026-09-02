//! 펭귄 코어 — Tauri 무의존 순수 상태머신.
//!
//! 시간은 epoch ms로, 놀 수 있는 영역은 [`World`](화면마다 자기 [`Bounds`]를 갖는
//! [`Screen`]의 목록)로 주입받는다. 난수도 코어가
//! 소유한 시드 PRNG라 같은 시드 + 같은 타임스탬프열은 항상 같은 동작 시퀀스를 낳는다 —
//! 그래야 "스스로 움직이는" 동작을 테스트로 고정할 수 있다 (KTD1).

use serde::Serialize;

mod tuning;
#[cfg(test)]
mod test_support;

/// 브릿지가 창 크기를 계산하는 데 쓴다 — 코어 밖으로 나가는 유일한 튜닝 값이다.
pub use tuning::PET_SIZE;
use tuning::*;

mod behavior;

pub use behavior::{Behavior, Facing, FishingPhase, FreakoutPhase, IdleKind, SassyKind, Speech, Vertical};
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
    /// **핀볼 모드인가.** 켜면 착지 등급 판정을 우회하고 벽·천장·바닥이 전부
    /// 반사면이 된다 (`landing`, `PINBALL_DAMPING`).
    pinball: bool,
    /// 지금 하는 발작 한 판이 끝나는 시각.
    freakout_until_ms: u64,
    /// 지금 하는 얼음낚시 한 판이 끝나는 시각. **절대 시각 하나로 갖는다** —
    /// 국면마다 남은 시간을 빼 나가면 국면이 늘 때마다 계산이 갈라진다.
    fishing_until_ms: u64,
    rng: u64,
}

/// 펭귄 식별자. 창 라벨(`pet-<id>`)과 짝을 이룬다.
pub type PetId = u32;

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
        self.pets.remove(&id).is_some()
    }

    /// 창이 사라진 펭귄을 정리한다. 마지막 한 마리 보호를 받지 않는다 —
    /// 창이 없는 펭귄은 사용자의 선택이 아니라 이미 없어진 것이다.
    pub fn forget(&mut self, id: PetId) {
        self.pets.remove(&id);
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
    }
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
            pinball: false,
            swim_descending: false,
            freakout_until_ms: 0,
            fishing_until_ms: 0,
            rng: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
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

    pub fn behavior(&self) -> Behavior {
        self.behavior
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
        if self.speech.is_some() && now_ms >= self.speech_until_ms {
            self.speech = None;
        }
        if self.speech.is_none() && now_ms >= self.next_taunt_ms {
            self.say(now_ms);
            let gap = self.range(TAUNT_GAP_MS);
            self.next_taunt_ms = now_ms + SPEECH_MS + gap;
        }

        match self.behavior {
            Behavior::Dragged => {}
            Behavior::Swing => self.tick_swing(now_ms),
            Behavior::Sassy { .. } => self.tick_sassy(now_ms),
            Behavior::Squawk => self.tick_squawk(now_ms),
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
            self.enter(Behavior::Idle { idle: IdleKind::Stretch }, until);
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
mod tests {
    use super::*;
    use super::test_support::*;

    #[test]
    fn 같은_시드는_같은_동작_시퀀스를_낳는다() {
        let mut a = Pet::new(2024, 0, &world());
        let mut b = Pet::new(2024, 0, &world());
        let seq_a: Vec<Behavior> = drive(&mut a, 100, 60_000, 100, &world())
            .iter()
            .map(|s| s.behavior)
            .collect();
        let seq_b: Vec<Behavior> = drive(&mut b, 100, 60_000, 100, &world())
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert_eq!(seq_a, seq_b);
        let mut c = Pet::new(999, 0, &world());
        let seq_c: Vec<Behavior> = drive(&mut c, 100, 60_000, 100, &world())
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert_ne!(seq_a, seq_c);
    }

    #[test]
    fn 여러_종류의_동작이_나타난다() {
        let mut p = pet();
        let kinds: std::collections::HashSet<Behavior> =
            drive(&mut p, 100, 80_000, 100, &world()).iter().map(|s| s.behavior).collect();
        assert!(
            kinds.len() >= 3,
            "80초 동안 최소 3가지 동작이 나와야 한다 (실제: {kinds:?})"
        );
    }

    #[test]
    fn 유휴_동작은_연속으로_같은_종류가_반복되지_않는다() {
        let mut p = pet();
        let idles: Vec<IdleKind> = drive(&mut p, 100, 80_000, 100, &world())
            .iter()
            .filter_map(|s| match s.behavior {
                Behavior::Idle { idle } => Some(idle),
                _ => None,
            })
            .collect();
        let mut compressed: Vec<IdleKind> = Vec::new();
        for k in idles {
            if compressed.last() != Some(&k) {
                compressed.push(k);
            }
        }
        for pair in compressed.windows(2) {
            assert_ne!(pair[0], pair[1], "같은 유휴 동작이 연달아 선택됐다");
        }
    }

    #[test]
    fn 오랫동안_자극이_없으면_졸기로_전이한다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, SLEEP_AFTER_MS + 30_000, 250, &world());
        assert!(
            seen.iter().any(|s| s.behavior == Behavior::Sleep),
            "자극 없이 오래 두면 졸기가 나와야 한다"
        );
    }

    #[test]
    fn 졸기_전까지는_움직이는_시간이_멈춰_있는_시간보다_길다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
        let moving = seen
            .iter()
            .filter(|s| {
                matches!(s.behavior, Behavior::Walk | Behavior::Turn) || s.behavior.is_airborne()
            })
            .count();
        assert!(
            moving * 2 > seen.len(),
            "움직이는 비중이 절반을 넘어야 한다 (이동 {moving} / 전체 {})",
            seen.len()
        );
    }

    #[test]
    fn 졸기_상태에서는_위치가_변하지_않는다() {
        let mut p = pet();
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, &world());
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep, "졸기에 도달해야 한다");

        let x = p.snapshot().x;
        for _ in 0..20 {
            t += 250;
            p.step(t, &world());
            if p.behavior() != Behavior::Sleep {
                break;
            }
        }
        assert_eq!(p.snapshot().x, x, "자는 동안에는 움직이지 않는다");
        assert!(!Behavior::Sleep.moves_window(), "졸기는 창을 옮기지 않는다");
    }

    #[test]
    fn 대사_추첨값은_배정밀도에서_안전한_범위다() {
        let mut p = pet();
        for i in 0..200u64 {
            p.say(1_000 + i);
            let roll = p.snapshot().speech.unwrap().roll;
            assert!(roll < (1u64 << 53), "배정밀도로 정확히 표현돼야 한다: {roll}");
        }
    }

    #[test]
    fn 같은_대사가_연달아_나와도_새_발화로_구분된다() {
        let mut p = pet();
        p.say(1_000);
        let first = p.snapshot().speech.unwrap();
        p.say(1_100);
        let second = p.snapshot().speech.unwrap();
        assert!(second.seq > first.seq, "발화 번호가 늘어야 한다");
    }

    #[test]
    fn 말풍선은_시간이_지나면_사라진다() {
        let mut p = pet();
        p.say(1_000);
        assert!(p.step(1_500, &world()).speech.is_some(), "금방 사라지면 못 읽는다");
        assert!(
            p.step(1_000 + SPEECH_MS + 100, &world()).speech.is_none(),
            "계속 떠 있으면 안 된다"
        );
    }

    #[test]
    fn 가만_둬도_가끔_한마디_한다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
        let spoke: std::collections::HashSet<u64> =
            seen.iter().filter_map(|s| s.speech.map(|v| v.seq)).collect();
        assert!(spoke.len() >= 2, "2분 동안 한마디도 안 하면 심심하다 (실제 {})", spoke.len());
    }

    fn pets_with_one() -> Pets {
        let mut pets = Pets::new();
        pets.add(1, 0, &world(), BOUNDS.left).expect("첫 마리는 들어간다");
        pets
    }

    #[test]
    fn 같은_순간에_태어나도_첫마디_시각이_다르다() {
        let mut pets = Pets::new();
        let a = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
        let b = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
        let first = |pets: &mut Pets, id| {
            let mut t = 0;
            while t < 60_000 {
                t += 100;
                if pets.get_mut(id).unwrap().step(t, &world()).speech.is_some() {
                    return t;
                }
            }
            panic!("60초 안에 한마디도 안 했다");
        };
        assert_ne!(first(&mut pets, a), first(&mut pets, b));
    }

    #[test]
    fn 펭귄을_추가하면_새_id를_받는다() {
        let mut pets = pets_with_one();
        let second = pets.add(1, 0, &world(), 300.0).expect("두 번째도 들어간다");
        assert_eq!(pets.len(), 2);
        assert!(pets.get(second).is_some());
    }

    #[test]
    fn 지운_id는_다시_쓰이지_않는다() {
        let mut pets = pets_with_one();
        let second = pets.add(1, 0, &world(), 300.0).unwrap();
        assert!(pets.remove(second));
        let third = pets.add(1, 0, &world(), 300.0).unwrap();
        assert_ne!(
            second, third,
            "닫히는 중인 창과 새 창이 같은 라벨을 다투면 창 이동이 엉뚱한 쪽으로 간다"
        );
    }

    #[test]
    fn 마지막_한_마리는_삭제되지_않는다() {
        let mut pets = pets_with_one();
        let only = pets.ids()[0];
        assert!(!pets.remove(only), "전부 없애는 것은 on/off의 일이다");
        assert_eq!(pets.len(), 1);
    }

    #[test]
    fn 창이_사라진_펭귄은_마지막_한_마리여도_정리된다() {
        let mut pets = pets_with_one();
        let only = pets.ids()[0];
        pets.forget(only);
        assert!(pets.is_empty(), "창이 없는 펭귄은 사용자의 선택이 아니다");
    }

    #[test]
    fn 상한을_넘겨_추가하면_거부된다() {
        let mut pets = Pets::new();
        for _ in 0..MAX_PETS {
            assert!(pets.add(1, 0, &world(), BOUNDS.left).is_some());
        }
        assert!(pets.add(1, 0, &world(), BOUNDS.left).is_none());
        assert_eq!(pets.len(), MAX_PETS);
    }

    #[test]
    fn 마리마다_시드가_달라_다르게_움직인다() {
        let mut pets = Pets::new();
        let a = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
        let b = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
        let mut diverged = false;
        let mut t = 0;
        while t < 60_000 && !diverged {
            t += 100;
            let sa = pets.get_mut(a).unwrap().step(t, &world());
            let sb = pets.get_mut(b).unwrap().step(t, &world());
            diverged = sa.x != sb.x || sa.behavior != sb.behavior;
        }
        assert!(diverged, "시드가 같으면 한 마리가 복제된 것처럼 보인다");
    }

    #[test]
    fn 새_펭귄은_지정한_x에서_시작한다() {
        let mut pets = Pets::new();
        let id = pets.add(1, 0, &world(), 640.0).unwrap();
        assert_eq!(pets.get(id).unwrap().snapshot().x, 640.0);
    }

    #[test]
    fn 시작_x는_영역_밖으로_나가지_않는다() {
        let mut pets = Pets::new();
        let id = pets.add(1, 0, &world(), BOUNDS.right + 5_000.0).unwrap();
        assert_eq!(pets.get(id).unwrap().snapshot().x, BOUNDS.right);
    }

    #[test]
    fn 작업_영역이_바뀌면_다음_step에서_경계_안으로_들어온다() {
        let mut p = pet();
        p.x = 900.0;
        let narrow = Bounds {
            left: 0.0,
            right: 400.0,
            top: 0.0,
            floor_y: 600.0,
        };
        let s = p.step(1_000, &World::single(narrow));
        assert!(s.x <= narrow.right, "좁아진 영역 안으로 들어와야 한다");
        assert_eq!(s.y, narrow.floor_y, "바닥도 새 영역을 따른다");
    }

    #[test]
    fn 기준점은_펭귄_발밑_중앙이다() {
        let mut p = pet();
        p.x = 300.0;
        p.y = 400.0;
        assert_eq!(p.anchor(), (300.0 + PET_SIZE / 2.0, 400.0 + PET_SIZE));
    }

}
