//! 펭귄 코어 — Tauri 무의존 순수 상태머신 (`pomodoro.rs` 전례).
//!
//! 시간은 epoch ms로, 걸어다닐 수 있는 영역은 [`Bounds`]로 주입받는다. 난수도 코어가
//! 소유한 시드 PRNG라 같은 시드 + 같은 타임스탬프열은 항상 같은 동작 시퀀스를 낳는다 —
//! 그래야 "스스로 움직이는" 동작을 테스트로 고정할 수 있다 (KTD1).

use serde::Serialize;

/// 걷는 속도 (논리 px/초).
const WALK_SPEED: f64 = 42.0;
/// 헤엄치는 속도 (논리 px/초). 걷기보다 빨라야 "떠서 이동한다"는 느낌이 난다.
const SWIM_SPEED: f64 = 95.0;
/// 낙하 가속도 (논리 px/초²).
const GRAVITY: f64 = 900.0;
/// 벽·천장에 부딪혔을 때 남는 속도 비율. 1.0이면 영원히 튕긴다.
const BOUNCE_DAMPING: f64 = 0.5;
/// 던진 것으로 볼 최소 속도 (논리 px/초). 이보다 느리면 그냥 떨어뜨린 것이다.
const THROW_MIN_SPEED: f64 = 260.0;
/// 던지기 속도 상한 — 손이 미끄러져도 펭귄이 순간이동하지 않게 한다.
const THROW_MAX_SPEED: f64 = 2_600.0;
/// 헤엄 목적지에 도착했다고 볼 거리.
const ARRIVE_EPSILON: f64 = 6.0;
/// 한 step이 정산하는 최대 시간. 시스템 슬립 등으로 틱이 밀렸을 때
/// 펭귄이 화면을 가로질러 순간이동하지 않게 잘라낸다.
const MAX_STEP_MS: u64 = 250;

const TURN_MS: u64 = 250;
/// 싸가지 반응의 길이. 한 박자 확실히 보여야 킹받는다.
const SASSY_MS: u64 = 900;
/// 킹받는 한마디가 떠 있는 시간.
const SPEECH_MS: u64 = 3_200;
/// 한마디와 다음 한마디 사이 간격. 클릭과 무관하게 알아서 떠든다.
const TAUNT_GAP_MS: (u64, u64) = (7_000, 18_000);
/// 맞고 움찔하는 시간. 이 동안 제자리에 있는다.
const BONK_MS: u64 = 460;
/// 방망이가 닿기까지의 시간. 이 동안 펭귄은 제자리에서 움찔하며 기다린다 —
/// 클릭하자마자 날아가면 방망이가 허공을 치는 그림이 된다.
const WINDUP_MS: u64 = 120;
/// 착지 후 약이 올라 한마디 할 확률(%).
const SASSY_AFTER_LAND_PERCENT: u64 = 70;
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
/// 동작이 끝났을 때 공중으로 떠오를 확률(%).
const SWIM_PERCENT: u64 = 30;

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

/// 펭귄이 지금 하는 말. **문구 자체는 웹뷰가 갖는다** — 대사는 표현이고,
/// 코어는 "언제 무엇을 뽑았는지"만 안다.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub struct Speech {
    /// 발화 번호. 같은 대사가 연달아 나와도 웹뷰가 새 말풍선으로 알아본다.
    pub seq: u64,
    /// 대사 추첨값. 웹뷰가 `목록[roll % 목록길이]`로 고른다.
    pub roll: u64,
}

/// 지금 오르는 중인지 내려가는 중인지. 웹뷰가 몸 기울기를 고르는 데 쓴다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Vertical {
    Up,
    Down,
    Level,
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

/// 클릭했을 때의 반응. 놀라는 대신 **싸가지 없게** 군다 — 이게 이 펭귄의 성격이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SassyKind {
    /// 등을 홱 돌린다
    TurnAway,
    /// 고개만 홱 돌려 외면한다
    HeadFlick,
    /// 날개를 휘휘 저어 쫓아낸다
    WingFlick,
    /// 눈을 굴린다
    EyeRoll,
    /// 엉덩이를 흔든다
    ButtWiggle,
}

const SASSY_KINDS: [SassyKind; 5] = [
    SassyKind::TurnAway,
    SassyKind::HeadFlick,
    SassyKind::WingFlick,
    SassyKind::EyeRoll,
    SassyKind::ButtWiggle,
];

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
    /// 공중을 헤엄쳐 목적지로 이동한다 — 위아래로 다니는 수단 (R11)
    Swim,
    Sleep,
    /// 클릭에 대한 반응 — 놀라지 않고 싸가지 없게 군다 (R5)
    Sassy { sassy: SassyKind },
    /// 방망이가 날아오는 중 — 아직 맞지 않았다 (R14)
    WindUp,
    /// 방망이에 맞고 움찔하는 중. **날아가지 않는다** — 나는 건 드래그로 던졌을 때뿐이다
    Bonked,
    /// 사용자가 집어 든 상태 — 자율 이동을 하지 않는다 (R6)
    Dragged,
    Falling,
    /// 던져져 포물선을 그리는 중 — 좌우 속도를 갖는다는 점이 Falling과 다르다 (R12)
    Thrown,
    /// 착지 스쿼시
    Land,
}

impl Behavior {
    /// 창을 실제로 옮겨야 하는 동작인가. 졸기는 아니다 (R10).
    pub fn moves_window(self) -> bool {
        !matches!(self, Behavior::Sleep)
    }

    /// 스스로 고도를 만드는 동작인가 (진입하면 공중 상태가 된다).
    pub fn is_airborne(self) -> bool {
        matches!(self, Behavior::Swim | Behavior::Falling | Behavior::Thrown)
    }
}

/// 펭귄이 돌아다닐 수 있는 영역 (논리 좌표). `left`/`right`는 창의 좌상단 x가
/// 가질 수 있는 최소·최대값이고, `top`은 올라갈 수 있는 최고점,
/// `floor_y`는 바닥에 섰을 때의 y다.
/// 창 크기 보정은 이 값을 만드는 쪽(브릿지)이 이미 끝낸 상태로 넘긴다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bounds {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub floor_y: f64,
}

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
    rng: u64,
}

/// 던지기 속도를 방향은 유지한 채 상한으로 자른다.
fn clamp_throw(vx: f64, vy: f64) -> (f64, f64) {
    let speed = (vx * vx + vy * vy).sqrt();
    if speed <= THROW_MAX_SPEED || speed == 0.0 {
        return (vx, vy);
    }
    let k = THROW_MAX_SPEED / speed;
    (vx * k, vy * k)
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
            last_sassy: None,
            speech: None,
            speech_until_ms: 0,
            speech_seq: 0,
            whack_seq: 0,
            next_taunt_ms: start_ms + TAUNT_GAP_MS.0,
            vx: 0.0,
            vy: 0.0,
            air: false,
            target: (bounds.left, bounds.floor_y),
            last_y: bounds.floor_y,
            rng: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
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

    /// 시간을 진행시키고 현재 상태를 돌려준다. 브릿지가 매 틱 호출한다.
    pub fn step(&mut self, now_ms: u64, bounds: Bounds) -> Snapshot {
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        self.last_step_ms = now_ms;
        let dt = elapsed as f64 / 1000.0;
        self.last_y = self.y;
        if self.speech.is_some() && now_ms >= self.speech_until_ms {
            self.speech = None;
        }
        // 말은 클릭과 무관하게 몇 초 간격으로 알아서 나온다
        if self.speech.is_none() && now_ms >= self.next_taunt_ms {
            self.say(now_ms);
            let gap = self.range(TAUNT_GAP_MS);
            self.next_taunt_ms = now_ms + SPEECH_MS + gap;
        }

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
                        self.pick_next(now_ms, bounds);
                    }
                } else if self.x <= bounds.left {
                    self.x = bounds.left;
                    self.enter(Behavior::Turn, now_ms + TURN_MS);
                } else if self.x >= bounds.right {
                    self.x = bounds.right;
                    self.enter(Behavior::Turn, now_ms + TURN_MS);
                } else if now_ms >= self.behavior_until_ms {
                    self.pick_next(now_ms, bounds);
                }
            }
            Behavior::Turn => {
                if now_ms >= self.behavior_until_ms {
                    self.facing = self.facing.flipped();
                    let until = now_ms + self.range(WALK_MS);
                    self.enter(Behavior::Walk, until);
                }
            }
            Behavior::WindUp => {
                if now_ms >= self.behavior_until_ms {
                    // 방망이가 닿았다 — 그 자리에서 움찔한다
                    self.enter(Behavior::Bonked, now_ms + BONK_MS);
                }
            }
            Behavior::Bonked => {
                if now_ms >= self.behavior_until_ms {
                    if self.air {
                        // 공중에서 맞았다면 이제 마저 떨어진다
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        // 맞고 나면 약이 올라 한 소리 한다
                        self.enter_sassy(now_ms);
                    }
                }
            }
            Behavior::Sassy { .. } => {
                if now_ms >= self.behavior_until_ms {
                    if self.air {
                        // 공중에서 반응했다면 이제 내려앉는다
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
            Behavior::Land => {
                if now_ms >= self.behavior_until_ms {
                    // 맞고 떨어졌으면 일어나서 약을 올린다
                    if self.range((0, 99)) < SASSY_AFTER_LAND_PERCENT {
                        self.enter_sassy(now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
            Behavior::Swim => {
                let (tx, ty) = self.target;
                let (dx, dy) = (tx - self.x, ty - self.y);
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
                    // 도착했거나 너무 오래 걸렸다 — 내려앉는다
                    self.vy = 0.0;
                    self.enter(Behavior::Falling, now_ms);
                } else {
                    let step = (SWIM_SPEED * dt).min(dist);
                    self.x += dx / dist * step;
                    self.y += dy / dist * step;
                    // 진행 방향을 본다 (좌우 성분이 거의 없으면 방향을 유지한다)
                    if dx.abs() > 1.0 {
                        self.facing = if dx > 0.0 { Facing::Right } else { Facing::Left };
                    }
                }
            }
            Behavior::Thrown => {
                self.vy += GRAVITY * dt;
                self.x += self.vx * dt;
                self.y += self.vy * dt;
                // 좌우 벽과 천장에서 튕긴다 — 경계에 붙어 미끄러지지 않게
                if (self.x <= bounds.left && self.vx < 0.0)
                    || (self.x >= bounds.right && self.vx > 0.0)
                {
                    self.vx = -self.vx * BOUNCE_DAMPING;
                }
                if self.y <= bounds.top && self.vy < 0.0 {
                    self.vy = -self.vy * BOUNCE_DAMPING;
                }
                if self.vx.abs() > 1.0 {
                    self.facing = if self.vx > 0.0 { Facing::Right } else { Facing::Left };
                }
                // 벽·천장과 마찬가지로 방향 가드가 필요하다. 드래그는 경계 밖으로도
                // 따라가므로(Dock 위 등) 바닥보다 아래에서 놓을 수 있는데, 가드가 없으면
                // 위로 던져도 첫 틱에 "착지"로 삼켜지며 위로 순간이동한다
                if self.y >= bounds.floor_y && self.vy >= 0.0 {
                    self.y = bounds.floor_y;
                    self.vx = 0.0;
                    self.vy = 0.0;
                    self.enter(Behavior::Land, now_ms + LAND_MS);
                }
            }
            Behavior::Idle { .. } | Behavior::Sleep => {
                if now_ms >= self.behavior_until_ms {
                    self.pick_next(now_ms, bounds);
                }
            }
        }

        // 모니터가 바뀌거나 해상도가 달라지면 영역 밖에 남을 수 있다 — 항상 되돌린다
        self.clamp(bounds);
        self.snapshot()
    }

    /// 클릭 — 졸고 있어도 깨워서 놀라게 한다 (R5).
    ///
    /// 공중에서 찔리면 놀라 떨어진다. `Startled`는 지상 동작이라 그대로 넣으면
    /// 같은 step의 clamp가 펭귄을 바닥으로 순간이동시킨다.
    /// 빠따 — 클릭 한 번에 한 번 맞는다. **맞아도 날아가지 않는다.**
    /// 날려 보내는 건 드래그로 던졌을 때(`drag_end`)뿐이다.
    /// 연타하면 계속 맞는다.
    pub fn whack(&mut self, now_ms: u64, _bounds: Bounds) {
        self.last_stimulus_ms = now_ms;
        self.whack_seq += 1;
        // 제자리에서 맞는다 — 속도를 주지 않는다
        self.vx = 0.0;
        self.vy = 0.0;
        self.enter(Behavior::WindUp, now_ms + WINDUP_MS);
    }

    /// 킹받는 한마디를 띄운다. 문구는 웹뷰가 고른다.
    pub fn say(&mut self, now_ms: u64) {
        self.speech_seq += 1;
        // u64를 그대로 보내면 JS의 배정밀도에서 하위 비트가 잘려 대사 절반이
        // 영영 안 나온다 (2^63 근처 값은 2^11의 배수라 나머지가 짝수로 고정된다)
        let roll = self.next_u64() % 100_000;
        self.speech = Some(Speech {
            seq: self.speech_seq,
            roll,
        });
        self.speech_until_ms = now_ms + SPEECH_MS;
    }

    /// 싸가지 반응 하나를 고른다 — 직전과 같은 종류는 피한다.
    fn enter_sassy(&mut self, now_ms: u64) {
        let mut sassy = SASSY_KINDS[self.range((0, 4)) as usize];
        if Some(sassy) == self.last_sassy {
            let next =
                (SASSY_KINDS.iter().position(|k| *k == sassy).unwrap() + 1) % SASSY_KINDS.len();
            sassy = SASSY_KINDS[next];
        }
        self.last_sassy = Some(sassy);
        self.enter(Behavior::Sassy { sassy }, now_ms + SASSY_MS);
    }

    /// 드래그 시작 — 자율 이동을 멈춘다 (R6).
    pub fn drag_start(&mut self, now_ms: u64) {
        self.last_stimulus_ms = now_ms;
        self.vy = 0.0;
        // air를 여기서 세우지 않는다. 프론트는 클릭인지 드래그인지 알기 전에
        // 모든 pointerdown에서 drag_start를 부르므로, 여기서 띄워 버리면 땅에서
        // 클릭해도 반응 뒤에 헛낙하 + 착지 스쿼시가 붙는다. 실제로 들어 올렸다면
        // drag_end가 Thrown/Falling으로 들어가며 air를 스스로 세운다.
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

    /// 드래그 놓기 (R6, R12). 놓는 순간의 속도(논리 px/초)를 받아, 세게 던졌으면
    /// 그 속도로 포물선을 그리고 살짝 놓았으면 제자리에서 떨어진다.
    pub fn drag_end(&mut self, now_ms: u64, vx: f64, vy: f64) {
        self.last_stimulus_ms = now_ms;
        let (vx, vy) = clamp_throw(vx, vy);
        if (vx * vx + vy * vy).sqrt() >= THROW_MIN_SPEED {
            self.vx = vx;
            self.vy = vy;
            self.enter(Behavior::Thrown, now_ms);
        } else {
            self.vx = 0.0;
            self.vy = 0.0;
            self.enter(Behavior::Falling, now_ms);
        }
    }

    /// 헤엄 목적지를 영역 안에서 무작위로 고른다 (R11).
    fn enter_swim(&mut self, now_ms: u64, bounds: Bounds) {
        let width = (bounds.right - bounds.left).max(0.0);
        let height = (bounds.floor_y - bounds.top).max(0.0);
        let tx = bounds.left + self.fraction() * width;
        // 바닥에만 붙어 다니지 않게 위쪽을 조금 더 자주 고른다
        let ty = bounds.top + self.fraction().powf(1.4) * height;
        self.target = (tx, ty);
        // 목적지까지 걸릴 시간의 2배를 상한으로 둔다 — 경계 정산 등으로
        // 영영 도착하지 못해도 헤엄에 갇히지 않는다
        let dist = ((tx - self.x).powi(2) + (ty - self.y).powi(2)).sqrt();
        let budget_ms = ((dist / SWIM_SPEED) * 2_000.0) as u64 + 1_000;
        self.enter(Behavior::Swim, now_ms + budget_ms);
    }

    fn enter(&mut self, behavior: Behavior, until_ms: u64) {
        // 반응·드래그는 고도를 그대로 물려받고, 나머지는 동작이 곧 고도를 정한다.
        // 착지(Land)는 바닥에 닿은 시점이라 확실히 지상이다.
        match behavior {
            Behavior::Sassy { .. } | Behavior::Dragged | Behavior::WindUp | Behavior::Bonked => {}
            Behavior::Land => self.air = false,
            other => self.air = other.is_airborne(),
        }
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
    fn pick_next(&mut self, now_ms: u64, bounds: Bounds) {
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
        // 가끔 공중으로 떠서 화면 위쪽까지 돌아다닌다 (R11).
        // 바닥에서만 왔다갔다 하면 화면의 대부분을 쓰지 못한다.
        if bounds.floor_y - bounds.top > 1.0 && self.range((0, 99)) < SWIM_PERCENT {
            self.enter_swim(now_ms, bounds);
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
        self.x = self.x.clamp(bounds.left, bounds.right.max(bounds.left));
        if self.air {
            // 공중에서는 위아래 경계만 지킨다 — 바닥에 붙이면 헤엄이 성립하지 않고,
            // 공중에서 클릭했을 때 펭귄이 바닥으로 끌려간다
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

    const BOUNDS: Bounds = Bounds {
        left: 0.0,
        right: 1000.0,
        top: 0.0,
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
    fn 졸기_전까지는_움직이는_시간이_멈춰_있는_시간보다_길다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, BOUNDS);
        // 헤엄·낙하도 이동이다 — 이 테스트가 지키려는 것은 "가만히 있지 않는다"이지
        // "걷기라는 특정 동작을 많이 한다"가 아니다
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
    fn 들어_올렸다_놓으면_여전히_떨어진다() {
        // 위 수정이 드래그를 망가뜨리지 않았는지 반대편에서 고정한다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, BOUNDS);
        p.drag_end(1_100, 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Falling);
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 8_000 {
            t += 50;
            p.step(t, BOUNDS);
        }
        assert_eq!(p.behavior(), Behavior::Land);
    }

    #[test]
    fn 빠따를_맞아도_날아가지_않는다() {
        // 나는 건 드래그로 던졌을 때뿐이다 — 클릭으로 날아가면 안 된다
        let mut p = pet();
        p.step(1_000, BOUNDS);
        let before = p.snapshot();
        p.whack(1_000, BOUNDS);
        assert_eq!(p.behavior(), Behavior::WindUp, "방망이가 닿기 전엔 제자리다");

        let mut t = 1_000;
        for _ in 0..30 {
            t += 50;
            let s = p.step(t, BOUNDS);
            assert_eq!(s.x, before.x, "옆으로 밀리면 안 된다");
            assert_eq!(s.y, before.y, "떠오르면 안 된다");
            assert_ne!(s.behavior, Behavior::Thrown, "던져진 상태가 되면 안 된다");
        }
    }

    #[test]
    fn 맞으면_움찔했다가_약을_올린다() {
        let mut p = pet();
        p.step(1_000, BOUNDS);
        p.whack(1_000, BOUNDS);
        assert_eq!(p.step(1_000 + WINDUP_MS + 10, BOUNDS).behavior, Behavior::Bonked);
        let after = p.step(1_000 + WINDUP_MS + BONK_MS + 20, BOUNDS);
        assert!(
            matches!(after.behavior, Behavior::Sassy { .. }),
            "맞고 나면 약이 올라야 한다 (실제: {:?})",
            after.behavior
        );
    }

    #[test]
    fn 빠따는_한_번에_한_번씩_횟수가_는다() {
        // 웹뷰가 방망이를 몇 번 휘두를지 이 값으로 안다. 연타해도 매번 보여야 한다
        let mut p = pet();
        assert_eq!(p.snapshot().whack_seq, 0);
        for i in 1..=5u64 {
            p.whack(1_000 + i * 100, BOUNDS);
            assert_eq!(p.snapshot().whack_seq, i, "{i}번째 빠따가 안 세어졌다");
        }
    }

    #[test]
    fn 던져서_나는_중에_때리면_그_자리에서_맞고_마저_떨어진다() {
        // 때리는 것으로는 새 속도가 붙지 않는다 — 나는 건 던지기 전용이다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, BOUNDS);
        p.drag_end(1_100, 600.0, -400.0);
        assert_eq!(p.behavior(), Behavior::Thrown);

        // 아직 공중일 때 때린다
        let mut t = 1_100;
        for _ in 0..4 {
            t += 50;
            p.step(t, BOUNDS);
        }
        assert_eq!(p.behavior(), Behavior::Thrown, "아직 나는 중이어야 한다");
        assert!(p.snapshot().air, "공중 상태여야 한다");

        p.whack(t, BOUNDS);
        let hit_y = p.snapshot().y;
        t += WINDUP_MS + 10;
        let bonked = p.step(t, BOUNDS);
        assert_eq!(bonked.behavior, Behavior::Bonked);
        assert_eq!(bonked.y, hit_y, "맞는다고 솟아오르거나 떨어지면 안 된다");

        // 움찔이 끝나면 마저 떨어진다
        let after = p.step(t + BONK_MS + 20, BOUNDS);
        assert_eq!(after.behavior, Behavior::Falling, "공중이었으니 마저 떨어진다");
    }

    #[test]
    fn 빠따는_졸고_있어도_깨워서_날린다() {
        let mut p = pet();
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, BOUNDS);
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep);
        p.whack(t, BOUNDS);
        assert_eq!(p.behavior(), Behavior::WindUp);
        assert_eq!(p.step(t + WINDUP_MS + 10, BOUNDS).behavior, Behavior::Bonked);
    }

    #[test]
    fn 빠따를_맞는다고_말하지는_않는다() {
        // 말은 클릭이 아니라 시간에 맞춰 나온다 — 때릴 때마다 떠들면 시끄럽다
        let mut p = pet();
        p.whack(1_000, BOUNDS);
        assert!(p.snapshot().speech.is_none(), "클릭으로 말이 나오면 안 된다");
        p.whack(1_100, BOUNDS);
        p.whack(1_200, BOUNDS);
        assert!(p.snapshot().speech.is_none(), "연타해도 마찬가지다");
    }

    #[test]
    fn 대사_추첨값은_배정밀도에서_안전한_범위다() {
        // u64를 그대로 보내면 JS에서 하위 비트가 잘려 대사 절반이 영영 안 나온다
        let mut p = pet();
        for i in 0..200u64 {
            p.say(1_000 + i);
            let roll = p.snapshot().speech.unwrap().roll;
            assert!(roll < (1u64 << 53), "배정밀도로 정확히 표현돼야 한다: {roll}");
        }
    }

    #[test]
    fn 같은_대사가_연달아_나와도_새_발화로_구분된다() {
        // seq가 안 늘면 웹뷰가 말풍선을 다시 띄우지 못한다
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
        assert!(p.step(1_500, BOUNDS).speech.is_some(), "금방 사라지면 못 읽는다");
        assert!(
            p.step(1_000 + SPEECH_MS + 100, BOUNDS).speech.is_none(),
            "계속 떠 있으면 안 된다"
        );
    }

    #[test]
    fn 가만_둬도_가끔_한마디_한다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, BOUNDS);
        let spoke: std::collections::HashSet<u64> =
            seen.iter().filter_map(|s| s.speech.map(|v| v.seq)).collect();
        assert!(spoke.len() >= 2, "2분 동안 한마디도 안 하면 심심하다 (실제 {})", spoke.len());
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

        p.drag_end(1_200, 0.0, 0.0);
        let s = p.step(1_300, BOUNDS);
        assert_eq!(s.x, BOUNDS.right, "놓으면 영역 안으로 정산된다");
    }

    #[test]
    fn 드래그를_놓으면_낙하해_바닥에서_멈춘다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -400.0);
        p.step(1_100, BOUNDS);
        p.drag_end(1_200, 0.0, 0.0);
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
        let narrow = Bounds { left: 10.0, right: 10.0, top: 0.0, floor_y: 50.0 };
        let mut p = Pet::new(5, 0, narrow);
        let seen = drive(&mut p, 100, 40_000, 250, narrow);
        assert!(
            seen.iter().any(|s| !matches!(s.behavior, Behavior::Turn)),
            "회전 말고 다른 동작으로 넘어가야 한다"
        );
        assert!(seen.iter().all(|s| s.x == narrow.left));
    }

    #[test]
    fn 헤엄을_치면_바닥에서_떠오른다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, BOUNDS);
        assert!(
            seen.iter().any(|s| s.behavior == Behavior::Swim),
            "가끔은 공중으로 떠야 한다"
        );
        // 바닥보다 확실히 위쪽까지 올라간 적이 있어야 한다 (R11)
        let highest = seen.iter().map(|s| s.y).fold(f64::MAX, f64::min);
        assert!(
            highest < BOUNDS.floor_y - 50.0,
            "화면 위쪽을 쓰지 못했다 (최고점 {highest}, 바닥 {})",
            BOUNDS.floor_y
        );
    }

    #[test]
    fn 헤엄은_영역을_벗어나지_않는다() {
        let mut p = pet();
        for s in drive(&mut p, 100, 120_000, 100, BOUNDS) {
            assert!(s.x >= BOUNDS.left && s.x <= BOUNDS.right, "x가 벗어났다: {}", s.x);
            assert!(s.y >= BOUNDS.top && s.y <= BOUNDS.floor_y, "y가 벗어났다: {}", s.y);
        }
    }

    #[test]
    fn 올라갈_때와_내려갈_때의_세로_방향이_다르다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, BOUNDS);
        assert!(seen.iter().any(|s| s.vertical == Vertical::Up), "오르는 구간이 없다");
        assert!(seen.iter().any(|s| s.vertical == Vertical::Down), "내려가는 구간이 없다");
        // 지상 동작에서는 항상 Level이어야 CSS가 엉뚱한 기울기를 잡지 않는다
        for s in &seen {
            if !s.behavior.is_airborne() {
                assert_eq!(s.vertical, Vertical::Level, "지상인데 기울었다: {:?}", s.behavior);
            }
        }
    }

    #[test]
    fn 세게_던지면_포물선을_그린다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, BOUNDS);
        // 오른쪽 위로 세게 던진다
        p.drag_end(1_100, 700.0, -400.0);
        assert_eq!(p.behavior(), Behavior::Thrown);

        let start_x = p.snapshot().x;
        let mut ys = Vec::new();
        let mut t = 1_100;
        while p.behavior() == Behavior::Thrown && t < 12_000 {
            t += 50;
            ys.push(p.step(t, BOUNDS).y);
        }
        assert_eq!(p.behavior(), Behavior::Land, "결국 착지해야 한다");
        assert!(p.snapshot().x > start_x, "던진 방향으로 나아가야 한다");
        // 포물선 = 올라갔다 내려온다
        let peak = ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!(peak < ys[0], "위로 솟는 구간이 있어야 한다");
        assert!(*ys.last().unwrap() > peak, "다시 내려와야 한다");
    }

    #[test]
    fn 세게_던질수록_멀리_난다() {
        let throw = |vx: f64| {
            let mut p = pet();
            p.drag_start(1_000);
            p.drag_end(1_000, vx, -200.0);
            let start = p.snapshot().x;
            let mut t = 1_000;
            while p.behavior() == Behavior::Thrown && t < 12_000 {
                t += 50;
                p.step(t, BOUNDS);
            }
            p.snapshot().x - start
        };
        assert!(throw(900.0) > throw(350.0), "세기에 비례해 더 멀리 가야 한다");
    }

    #[test]
    fn 살짝_놓으면_던지지_않고_제자리에서_떨어진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, BOUNDS);
        let x = p.snapshot().x;
        p.drag_end(1_100, 20.0, 5.0);
        assert_eq!(p.behavior(), Behavior::Falling);

        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 12_000 {
            t += 50;
            p.step(t, BOUNDS);
        }
        assert!((p.snapshot().x - x).abs() < 1.0, "좌우로 날아가면 안 된다");
    }

    #[test]
    fn 바닥보다_아래에서_위로_던져도_삼켜지지_않는다() {
        // 드래그는 경계 밖으로도 따라가므로(Dock 위 등) 바닥보다 아래에서 놓을 수 있다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, 90.0); // 바닥보다 90px 아래로 끌어내림
        p.step(1_050, BOUNDS);
        p.drag_end(1_100, 700.0, -400.0); // 오른쪽 위로 세게
        assert_eq!(p.behavior(), Behavior::Thrown);

        let first = p.step(1_150, BOUNDS);
        assert_eq!(
            first.behavior,
            Behavior::Thrown,
            "위로 던졌는데 첫 틱에 착지로 삼켜졌다"
        );
        assert!(first.y > BOUNDS.floor_y - 1.0, "위로 순간이동하면 안 된다");
    }

    #[test]
    fn 던지기_속도는_상한을_넘지_않는다() {
        let mut p = pet();
        p.drag_start(1_000);
        // 비정상적으로 큰 속도가 들어와도 화면을 순간이동하지 않아야 한다
        p.drag_end(1_000, 500_000.0, -500_000.0);
        let first = p.step(1_050, BOUNDS);
        assert!(first.x <= BOUNDS.right && first.x >= BOUNDS.left);
        assert!(first.y >= BOUNDS.top && first.y <= BOUNDS.floor_y);
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
        let s = p.step(1_000, narrow);
        assert!(s.x <= narrow.right, "좁아진 영역 안으로 들어와야 한다");
        assert_eq!(s.y, narrow.floor_y, "바닥도 새 영역을 따른다");
    }
}
