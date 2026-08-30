//! 펭귄 코어 — Tauri 무의존 순수 상태머신 (`pomodoro.rs` 전례).
//!
//! 시간은 epoch ms로, 걸어다닐 수 있는 영역은 [`Bounds`]로 주입받는다. 난수도 코어가
//! 소유한 시드 PRNG라 같은 시드 + 같은 타임스탬프열은 항상 같은 동작 시퀀스를 낳는다 —
//! 그래야 "스스로 움직이는" 동작을 테스트로 고정할 수 있다 (KTD1).

use serde::Serialize;

/// 걷는 속도 (논리 px/초).
const WALK_SPEED: f64 = 42.0;
/// 낙하 가속도 (논리 px/초²).
const GRAVITY: f64 = 900.0;
/// 한 step이 정산하는 최대 시간. 시스템 슬립 등으로 틱이 밀렸을 때
/// 펭귄이 화면을 가로질러 순간이동하지 않게 잘라낸다.
const MAX_STEP_MS: u64 = 250;

const TURN_MS: u64 = 250;
const STARTLED_MS: u64 = 400;
const LAND_MS: u64 = 300;
/// 마지막 자극(클릭·드래그) 이후 이만큼 지나면 졸기로 넘어간다.
/// 길게 잡는다 — 펭귄이 깨어서 돌아다니는 게 이 기능의 목적이고, 졸기는 양념이다.
const SLEEP_AFTER_MS: u64 = 300_000;

const WALK_MS: (u64, u64) = (2_500, 6_000);
const IDLE_MS: (u64, u64) = (1_200, 3_200);
/// 졸기는 끝이 있다 — 깨어나 다시 움직인다. 종착 상태가 아니다.
const SLEEP_MS: (u64, u64) = (12_000, 25_000);
/// 유휴가 끝났을 때 다시 걸을 확률(%). 멈춰 있는 시간보다 걷는 시간이 길어야 한다.
const WALK_AGAIN_PERCENT: u64 = 72;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    fn flipped(self) -> Self {
        match self {
            Facing::Left => Facing::Right,
            Facing::Right => Facing::Left,
        }
    }

    /// x축 진행 부호.
    fn sign(self) -> f64 {
        match self {
            Facing::Left => -1.0,
            Facing::Right => 1.0,
        }
    }
}

/// 제자리 유휴 동작의 종류. 창은 움직이지 않고 웹뷰 CSS만 달라진다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleKind {
    /// 두리번거리기
    LookAround,
    /// 기지개
    Stretch,
    /// 몸 털기
    Shake,
    /// 발 갈아 딛기
    ShiftFeet,
}

const IDLE_KINDS: [IdleKind; 4] = [
    IdleKind::LookAround,
    IdleKind::Stretch,
    IdleKind::Shake,
    IdleKind::ShiftFeet,
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Behavior {
    Walk,
    /// 제자리에서 도는 중 — 끝나면 방향이 뒤집힌다 (R4)
    Turn,
    Idle { idle: IdleKind },
    Sleep,
    /// 클릭에 놀람 (R5)
    Startled,
    /// 사용자가 집어 든 상태 — 자율 이동을 하지 않는다 (R6)
    Dragged,
    Falling,
    /// 착지 스쿼시
    Land,
}

impl Behavior {
    /// 창을 실제로 옮겨야 하는 동작인가. 졸기는 아니다 (R10).
    pub fn moves_window(self) -> bool {
        !matches!(self, Behavior::Sleep)
    }
}

/// 펭귄이 돌아다닐 수 있는 영역 (논리 좌표). `left`/`right`는 창의 좌상단 x가
/// 가질 수 있는 최소·최대값이고, `floor_y`는 바닥에 섰을 때의 y다.
/// 창 크기 보정은 이 값을 만드는 쪽(브릿지)이 이미 끝낸 상태로 넘긴다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bounds {
    pub left: f64,
    pub right: f64,
    pub floor_y: f64,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct Snapshot {
    pub x: f64,
    pub y: f64,
    pub facing: Facing,
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
    /// 낙하 속도 (논리 px/초).
    vy: f64,
    rng: u64,
}

impl Pet {
    /// 시드는 0이면 안 된다 (xorshift가 0에 갇힌다) — 0이 들어오면 대체한다.
    pub fn new(seed: u64, start_ms: u64, bounds: Bounds) -> Self {
        Pet {
            x: bounds.left,
            y: bounds.floor_y,
            facing: Facing::Right,
            behavior: Behavior::Walk,
            behavior_until_ms: start_ms + WALK_MS.0,
            last_step_ms: start_ms,
            last_stimulus_ms: start_ms,
            last_idle: None,
            vy: 0.0,
            rng: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            x: self.x,
            y: self.y,
            facing: self.facing,
            behavior: self.behavior,
        }
    }

    pub fn behavior(&self) -> Behavior {
        self.behavior
    }

    /// 시간을 진행시키고 현재 상태를 돌려준다. 브릿지가 매 틱 호출한다.
    pub fn step(&mut self, now_ms: u64, bounds: Bounds) -> Snapshot {
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        self.last_step_ms = now_ms;
        let dt = elapsed as f64 / 1000.0;

        match self.behavior {
            // 사용자가 들고 있는 동안에는 물리도 자율 이동도 없다 (R6)
            Behavior::Dragged => {}
            Behavior::Falling => {
                self.vy += GRAVITY * dt;
                self.y += self.vy * dt;
                if self.y >= bounds.floor_y {
                    self.y = bounds.floor_y;
                    self.vy = 0.0;
                    self.enter(Behavior::Land, now_ms + LAND_MS);
                }
            }
            Behavior::Walk => {
                self.x += self.facing.sign() * WALK_SPEED * dt;
                // 걸어다닐 폭이 없는 화면(펭귄보다 좁은 작업 영역)에서는 양쪽 경계가
                // 겹쳐 매 step마다 Turn으로 들어가 영원히 제자리에서 돈다.
                // 그럴 때는 회전을 건너뛰고 평소처럼 유휴로 넘어가게 둔다.
                if bounds.right <= bounds.left {
                    self.x = bounds.left;
                    if now_ms >= self.behavior_until_ms {
                        self.pick_next(now_ms);
                    }
                } else if self.x <= bounds.left {
                    self.x = bounds.left;
                    self.enter(Behavior::Turn, now_ms + TURN_MS);
                } else if self.x >= bounds.right {
                    self.x = bounds.right;
                    self.enter(Behavior::Turn, now_ms + TURN_MS);
                } else if now_ms >= self.behavior_until_ms {
                    self.pick_next(now_ms);
                }
            }
            Behavior::Turn => {
                if now_ms >= self.behavior_until_ms {
                    self.facing = self.facing.flipped();
                    let until = now_ms + self.range(WALK_MS);
                    self.enter(Behavior::Walk, until);
                }
            }
            Behavior::Startled | Behavior::Land => {
                if now_ms >= self.behavior_until_ms {
                    // 놀람·착지 뒤에는 유휴로 한 박자 쉰다
                    self.enter_idle(now_ms);
                }
            }
            Behavior::Idle { .. } | Behavior::Sleep => {
                if now_ms >= self.behavior_until_ms {
                    self.pick_next(now_ms);
                }
            }
        }

        // 모니터가 바뀌거나 해상도가 달라지면 영역 밖에 남을 수 있다 — 항상 되돌린다
        self.clamp(bounds);
        self.snapshot()
    }

    /// 클릭 — 졸고 있어도 깨워서 놀라게 한다 (R5).
    pub fn poke(&mut self, now_ms: u64) {
        self.last_stimulus_ms = now_ms;
        self.enter(Behavior::Startled, now_ms + STARTLED_MS);
    }

    /// 드래그 시작 — 자율 이동을 멈춘다 (R6).
    pub fn drag_start(&mut self, now_ms: u64) {
        self.last_stimulus_ms = now_ms;
        self.vy = 0.0;
        self.enter(Behavior::Dragged, now_ms);
    }

    /// 드래그 이동량 반영. 들고 있는 동안에는 영역 밖으로도 따라간다 —
    /// 경계 정산은 놓는 시점(step의 clamp)에서 한다.
    pub fn drag_by(&mut self, dx: f64, dy: f64) {
        if self.behavior == Behavior::Dragged {
            self.x += dx;
            self.y += dy;
        }
    }

    /// 드래그 놓기 — 떨어뜨린다 (R6).
    pub fn drag_end(&mut self, now_ms: u64) {
        self.last_stimulus_ms = now_ms;
        self.vy = 0.0;
        self.enter(Behavior::Falling, now_ms);
    }

    fn enter(&mut self, behavior: Behavior, until_ms: u64) {
        self.behavior = behavior;
        self.behavior_until_ms = until_ms;
    }

    /// 유휴 동작 하나를 고른다 — 직전과 같은 종류는 피한다 (R3).
    fn enter_idle(&mut self, now_ms: u64) {
        let mut idle = IDLE_KINDS[self.range((0, 3)) as usize];
        if Some(idle) == self.last_idle {
            // 한 칸 밀어 같은 동작의 연속을 끊는다
            let next = (IDLE_KINDS.iter().position(|k| *k == idle).unwrap() + 1) % IDLE_KINDS.len();
            idle = IDLE_KINDS[next];
        }
        self.last_idle = Some(idle);
        let until = now_ms + self.range(IDLE_MS);
        self.enter(Behavior::Idle { idle }, until);
    }

    /// 동작이 끝났을 때 다음 동작을 고른다.
    fn pick_next(&mut self, now_ms: u64) {
        // 한참 건드리지 않았으면 존다 (R3, R10). 졸다 깨면 다시 활동한다
        if now_ms.saturating_sub(self.last_stimulus_ms) >= SLEEP_AFTER_MS
            && self.behavior != Behavior::Sleep
        {
            let until = now_ms + self.range(SLEEP_MS);
            self.enter(Behavior::Sleep, until);
            return;
        }
        if self.behavior == Behavior::Sleep {
            // 깨어나면 기지개부터 켠다
            self.last_stimulus_ms = now_ms;
            self.last_idle = Some(IdleKind::Stretch);
            let until = now_ms + self.range(IDLE_MS);
            self.enter(Behavior::Idle { idle: IdleKind::Stretch }, until);
            return;
        }
        // 걷다 쉬거나, 쉬다 걷는다
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
        // 들고 있는 동안에는 사용자가 원하는 곳에 둘 수 있어야 한다
        if self.behavior == Behavior::Dragged {
            return;
        }
        self.x = self.x.clamp(bounds.left, bounds.right);
        if self.behavior != Behavior::Falling {
            self.y = bounds.floor_y;
        } else if self.y > bounds.floor_y {
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

    /// `lo..=hi` 범위의 값 하나.
    fn range(&mut self, (lo, hi): (u64, u64)) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Bounds = Bounds {
        left: 0.0,
        right: 1000.0,
        floor_y: 800.0,
    };

    fn pet() -> Pet {
        Pet::new(42, 0, BOUNDS)
    }

    /// `from`부터 `to`까지 `dt` 간격으로 진행시키며 스냅샷을 모은다.
    fn drive(pet: &mut Pet, from: u64, to: u64, dt: u64, bounds: Bounds) -> Vec<Snapshot> {
        let mut out = Vec::new();
        let mut t = from;
        while t <= to {
            out.push(pet.step(t, bounds));
            t += dt;
        }
        out
    }

    #[test]
    fn 걷기_중에는_진행_방향으로_위치가_이동한다() {
        let mut p = pet();
        let before = p.snapshot();
        assert_eq!(before.behavior, Behavior::Walk);
        assert_eq!(before.facing, Facing::Right);

        let after = p.step(1_000, BOUNDS);
        // 1초에 WALK_SPEED만큼 — MAX_STEP_MS(250ms)로 잘리므로 그 몫만 이동한다
        assert!(after.x > before.x, "오른쪽을 보면 x가 커져야 한다");
    }

    #[test]
    fn 왼쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
        let mut p = Pet::new(7, 0, BOUNDS);
        p.facing = Facing::Left;
        p.x = 5.0;

        let s = p.step(200, BOUNDS);
        assert_eq!(s.x, BOUNDS.left, "경계를 넘어가면 안 된다");
        assert_eq!(s.behavior, Behavior::Turn);
    }

    #[test]
    fn 오른쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
        let mut p = Pet::new(7, 0, BOUNDS);
        p.x = BOUNDS.right - 3.0;

        let s = p.step(200, BOUNDS);
        assert_eq!(s.x, BOUNDS.right);
        assert_eq!(s.behavior, Behavior::Turn);
    }

    #[test]
    fn 방향_전환이_끝나면_반대_방향으로_걷는다() {
        let mut p = Pet::new(7, 0, BOUNDS);
        p.x = BOUNDS.right - 3.0;
        p.step(200, BOUNDS);
        assert_eq!(p.snapshot().facing, Facing::Right);

        let s = p.step(200 + TURN_MS + 10, BOUNDS);
        assert_eq!(s.facing, Facing::Left, "전환이 끝나면 방향이 뒤집힌다");
        assert_eq!(s.behavior, Behavior::Walk);
    }

    #[test]
    fn 같은_시드는_같은_동작_시퀀스를_낳는다() {
        let mut a = Pet::new(2024, 0, BOUNDS);
        let mut b = Pet::new(2024, 0, BOUNDS);
        let seq_a: Vec<Behavior> = drive(&mut a, 100, 60_000, 100, BOUNDS)
            .iter()
            .map(|s| s.behavior)
            .collect();
        let seq_b: Vec<Behavior> = drive(&mut b, 100, 60_000, 100, BOUNDS)
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert_eq!(seq_a, seq_b);
        // 시드가 다르면 시퀀스도 달라야 난수가 실제로 쓰이는 것이다
        let mut c = Pet::new(999, 0, BOUNDS);
        let seq_c: Vec<Behavior> = drive(&mut c, 100, 60_000, 100, BOUNDS)
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert_ne!(seq_a, seq_c);
    }

    #[test]
    fn 여러_종류의_동작이_나타난다() {
        let mut p = pet();
        let kinds: std::collections::HashSet<Behavior> =
            drive(&mut p, 100, 80_000, 100, BOUNDS).iter().map(|s| s.behavior).collect();
        assert!(
            kinds.len() >= 3,
            "80초 동안 최소 3가지 동작이 나와야 한다 (실제: {kinds:?})"
        );
    }

    #[test]
    fn 유휴_동작은_연속으로_같은_종류가_반복되지_않는다() {
        let mut p = pet();
        let idles: Vec<IdleKind> = drive(&mut p, 100, 80_000, 100, BOUNDS)
            .iter()
            .filter_map(|s| match s.behavior {
                Behavior::Idle { idle } => Some(idle),
                _ => None,
            })
            .collect();
        // 같은 유휴가 이어지는 구간을 압축한 뒤 인접 중복이 없는지 본다
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
        let seen = drive(&mut p, 100, SLEEP_AFTER_MS + 30_000, 250, BOUNDS);
        assert!(
            seen.iter().any(|s| s.behavior == Behavior::Sleep),
            "자극 없이 오래 두면 졸기가 나와야 한다"
        );
    }

    #[test]
    fn 졸기_전까지는_걷는_시간이_멈춰_있는_시간보다_길다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, BOUNDS);
        let walking = seen
            .iter()
            .filter(|s| matches!(s.behavior, Behavior::Walk | Behavior::Turn))
            .count();
        assert!(
            walking * 2 > seen.len(),
            "걷는 비중이 절반을 넘어야 한다 (걷기 {walking} / 전체 {})",
            seen.len()
        );
    }

    #[test]
    fn 졸기_상태에서는_위치가_변하지_않는다() {
        let mut p = pet();
        // 졸 때까지 진행시킨다
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, BOUNDS);
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep, "졸기에 도달해야 한다");

        let x = p.snapshot().x;
        for _ in 0..20 {
            t += 250;
            p.step(t, BOUNDS);
            if p.behavior() != Behavior::Sleep {
                break;
            }
        }
        assert_eq!(p.snapshot().x, x, "자는 동안에는 움직이지 않는다");
        assert!(!Behavior::Sleep.moves_window(), "졸기는 창을 옮기지 않는다");
    }

    #[test]
    fn 클릭은_졸기_상태에서도_놀람으로_깨운다() {
        let mut p = pet();
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, BOUNDS);
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep);

        p.poke(t);
        assert_eq!(p.behavior(), Behavior::Startled);
    }

    #[test]
    fn 놀람이_끝나면_유휴로_돌아온다() {
        let mut p = pet();
        p.poke(1_000);
        let s = p.step(1_000 + STARTLED_MS + 10, BOUNDS);
        assert!(matches!(s.behavior, Behavior::Idle { .. }));
    }

    #[test]
    fn 드래그_중에는_자율_이동이_멈추고_주어진_위치를_따른다() {
        let mut p = pet();
        p.drag_start(1_000);
        let before = p.snapshot();

        // 자율 이동이 없어야 한다
        let s = p.step(2_000, BOUNDS);
        assert_eq!(s.x, before.x);
        assert_eq!(s.behavior, Behavior::Dragged);

        // 드래그 이동량은 그대로 반영된다
        p.drag_by(100.0, -200.0);
        let moved = p.step(2_100, BOUNDS);
        assert_eq!(moved.x, before.x + 100.0);
        assert_eq!(moved.y, before.y - 200.0);
    }

    #[test]
    fn 드래그는_영역_밖으로도_따라가고_놓을_때_정산한다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(5_000.0, -500.0);
        // 들고 있는 동안에는 clamp하지 않는다 — 사용자가 끄는 대로 간다
        assert_eq!(p.step(1_100, BOUNDS).x, BOUNDS.left + 5_000.0);

        p.drag_end(1_200);
        let s = p.step(1_300, BOUNDS);
        assert_eq!(s.x, BOUNDS.right, "놓으면 영역 안으로 정산된다");
    }

    #[test]
    fn 드래그를_놓으면_낙하해_바닥에서_멈춘다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -400.0);
        p.step(1_100, BOUNDS);
        p.drag_end(1_200);
        assert_eq!(p.behavior(), Behavior::Falling);

        let mut t = 1_200;
        while p.behavior() == Behavior::Falling && t < 6_000 {
            t += 50;
            p.step(t, BOUNDS);
        }
        assert_eq!(p.behavior(), Behavior::Land, "바닥에 닿으면 착지한다");
        assert_eq!(p.snapshot().y, BOUNDS.floor_y);
    }

    #[test]
    fn 걸어다닐_폭이_없는_화면에서도_영원히_돌지_않는다() {
        // 작업 영역이 펭귄보다 좁으면 양쪽 경계가 겹쳐 매 step이 Turn이 된다
        let narrow = Bounds { left: 10.0, right: 10.0, floor_y: 50.0 };
        let mut p = Pet::new(5, 0, narrow);
        let seen = drive(&mut p, 100, 40_000, 250, narrow);
        assert!(
            seen.iter().any(|s| !matches!(s.behavior, Behavior::Turn)),
            "회전 말고 다른 동작으로 넘어가야 한다"
        );
        assert!(seen.iter().all(|s| s.x == narrow.left));
    }

    #[test]
    fn 작업_영역이_바뀌면_다음_step에서_경계_안으로_들어온다() {
        let mut p = pet();
        p.x = 900.0;
        let narrow = Bounds {
            left: 0.0,
            right: 400.0,
            floor_y: 600.0,
        };
        let s = p.step(1_000, narrow);
        assert!(s.x <= narrow.right, "좁아진 영역 안으로 들어와야 한다");
        assert_eq!(s.y, narrow.floor_y, "바닥도 새 영역을 따른다");
    }
}
