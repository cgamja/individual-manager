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
    ///
    /// 링버퍼에 최근 시각을 담아 "창 안에 N번"을 재는 방법도 있지만, 필드 둘이면
    /// 같은 체감을 내고 마릿수가 늘어도 메모리가 늘지 않는다.
    whack_run: u64,
    /// 마지막 빠따 시각. **`last_stimulus_ms`를 쓸 수 없다** — 그쪽은 드래그로도
    /// 갱신되므로 집었다 놓은 것이 연타로 세어진다.
    ///
    /// `None`은 "아직 한 번도 안 맞았다"이고 `Some(0)`은 "에폭 0에 맞았다"다.
    /// 지금은 두 경우의 **결과가 같다** — `whack_run`이 0에서 시작하므로 어느
    /// 갈래로 가든 첫 클릭은 1이 된다. 그래도 `Option`으로 두는 것은 이 등가가
    /// `whack_run`의 초깃값에 딸린 우연이라서다: 초깃값이 바뀌면 조용히 갈라진다.
    last_whack_ms: Option<u64>,
    /// 지금 빽빽거리는 판이 끝나는 시각. 0이면 빽빽거리는 중이 아니다.
    ///
    /// **`behavior`로는 판정할 수 없다.** 프론트는 클릭인지 드래그인지 알기 전에
    /// 모든 pointerdown에서 `drag_start`를 부르므로(그쪽 주석 참고), `whack`이
    /// 도착할 때 동작은 이미 `Dragged`다. 시각을 따로 들고 있어야 "빽빽거리는
    /// 중에 또 맞았다"를 알 수 있다.
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
    ///
    /// 목적지 y로 추론하면 안 된다 — `enter_swim`이 우연히 바닥 근처를 목적지로
    /// 뽑은 보통 헤엄까지 내려앉는 속도로 날아간다(실측 0.53%). 상태로 들고 있으면
    /// "예산이 끝나 집으로 돌린 순간"과 정확히 일치한다.
    swim_descending: bool,
    /// 이번 슬라이딩의 출발 속도 (논리 px/초). 진입할 때 한 번 뽑는다 —
    /// 길이는 고정이고 이 값이 거리를 정한다.
    slide_speed: f64,
    /// **핀볼 모드인가.** 켜면 착지 등급 판정을 우회하고 벽·천장·바닥이 전부
    /// 반사면이 된다 (`landing`, `PINBALL_DAMPING`).
    ///
    /// **`step`의 인자가 아니라 필드다.** 인자로 받으면 모든 모션 테스트의
    /// 호출부를 건드리는 대공사가 된다(`World` 교체 때 겪었고 그 후유증이
    /// `TODO.md`에 아직 열려 있다). 시드·`slide_speed`처럼 이미 있는 부류라
    /// 결정성도 그대로다.
    pinball: bool,
    /// 지금 하는 발작 한 판이 끝나는 시각.
    ///
    /// **빽빽거리기의 `squawk_until_ms`와 달리 무효화가 필요 없다.** 그쪽은
    /// `whack()`이 동작 밖에서 읽기 때문에 다른 동작으로 나갈 때 지워야 했지만,
    /// 이 값은 `Freakout` 팔 안에서만 읽으므로 새지 않는다.
    freakout_until_ms: u64,
    /// 지금 하는 얼음낚시 한 판이 끝나는 시각. **절대 시각 하나로 갖는다** —
    /// 국면마다 남은 시간을 빼 나가면 국면이 늘 때마다 계산이 갈라진다.
    fishing_until_ms: u64,
    rng: u64,
}

/// 이 세계에서 허용하는 던지기 최고 속도 (논리 px/초).
///
/// 폭이 유효하지 않으면 기준 폭으로 대체하고, 계산된 상한이 던지기 문턱보다 낮으면
/// 문턱까지 끌어올린다 — 그러지 않으면 좁은 세계에서 아무리 세게 던져도 던져지지 않는다.
fn throw_max_speed(world_width: f64) -> f64 {
    let width = if world_width > 0.0 {
        world_width
    } else {
        FALLBACK_WORLD_WIDTH
    };
    (width * THROW_MAX_WORLDS_PER_SEC).max(THROW_MIN_SPEED)
}

/// 던지기 속도를 방향은 유지한 채 세계 폭이 정한 상한으로 자른다.
fn clamp_throw(vx: f64, vy: f64, world_width: f64) -> (f64, f64) {
    let max = throw_max_speed(world_width);
    let speed = (vx * vx + vy * vy).sqrt();
    if speed <= max || speed == 0.0 {
        return (vx, vy);
    }
    let k = max / speed;
    (vx * k, vy * k)
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
    ///
    /// 시드는 `seed_base`와 id를 섞어 만든다 — 같은 시드를 받으면 두 마리가
    /// 똑같이 움직여 한 마리가 복제된 것처럼 보인다.
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

/// 바닥에 닿았을 때 무엇을 하는가.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Landing {
    /// 통통 — 반동 속도를 갖고 다시 떠오른다.
    Bounce(f64),
    /// 멈춰 선다 — 동작과 그 동작을 유지할 시간.
    Settle(Behavior, u64),
}

/// 바닥에 닿는 순간의 낙하 속도로 착지를 고른다.
///
/// 네 단계다: 아주 세게 박으면 **널브러지고**, 세게면 **철푸덕**, 어중간하면
/// **통통 튀고**, 거의 멈춘 채 닿으면 그냥 선다. 세기가 눈에 보여야
/// 높이 던진 보람이 착지에 남는다.
fn landing_of(impact_vy: f64) -> Landing {
    if impact_vy >= SPRAWL_MIN_IMPACT {
        Landing::Settle(Behavior::Sprawl, SPRAWL_MS)
    } else if impact_vy >= SPLAT_MIN_IMPACT {
        Landing::Settle(Behavior::Splat, SPLAT_MS)
    } else if impact_vy >= BOUNCE_MIN_SPEED {
        Landing::Bounce(-impact_vy * FLOOR_BOUNCE_DAMPING)
    } else {
        Landing::Settle(Behavior::Land, LAND_MS)
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
            // 저장된 설정은 브릿지가 만든 직후에 건다 — 기본은 꺼짐이다
            pinball: false,
            swim_descending: false,
            freakout_until_ms: 0,
            fishing_until_ms: 0,
            rng: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        };
        // 첫 한마디까지의 간격도 **뽑는다.** 고정값으로 두면 같은 순간에 태어난
        // 펭귄들이 다 같이 첫마디를 한다 — 여러 마리가 한목소리로 떠드는 꼴이다.
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

    /// 핀볼 모드를 켜고 끈다. 앱 전역 설정이라 브릿지가 전 마리에 건다.
    pub fn set_pinball(&mut self, on: bool) {
        self.pinball = on;
    }

    pub fn pinball(&self) -> bool {
        self.pinball
    }

    /// 바닥에 닿는 순간의 착지를 고른다.
    ///
    /// **`landing_of`를 지우지 않고 앞에서 가로챈다.** 핀볼은 착지 4단계를
    /// 지우는 모드가 아니라 **가려두는** 모드라, 끄면 문자 그대로 원래 함수가
    /// 돌아와야 한다.
    ///
    /// 핀볼에서는 철푸덕·널브러짐이 있던 속도 구간을 **통통이 흡수한다** —
    /// 새 분기가 아니라 문턱 둘이 사라지는 것이다. 아래 문턱
    /// (`BOUNCE_MIN_SPEED`)은 남긴다: 없애면 영원히 잔진동하며 다시는 걷지 않는다.
    ///
    /// **호출처 둘(`Falling`·`Thrown`)이 이 한 곳을 공유한다.** 벽 판정을
    /// `hit_wall` 한 곳에 모은 것과 같은 이유다 — 판정이 두 벌이 되면 한쪽만
    /// 고쳐지고 조용히 갈라진다.
    fn landing(&self, impact_vy: f64) -> Landing {
        if !self.pinball {
            return landing_of(impact_vy);
        }
        if impact_vy >= BOUNCE_MIN_SPEED {
            Landing::Bounce(-impact_vy * PINBALL_DAMPING)
        } else {
            Landing::Settle(Behavior::Land, LAND_MS)
        }
    }

    /// 벽·천장에서 튈 때 남는 속도 비율. 핀볼에서 벽은 범퍼다.
    fn wall_damping(&self) -> f64 {
        if self.pinball {
            PINBALL_DAMPING
        } else {
            BOUNCE_DAMPING
        }
    }

    /// 바닥에서 통통 튈 때 **가로** 속도에 남는 비율. 세로는 `landing`이 정한다.
    fn floor_vx_damping(&self) -> f64 {
        if self.pinball {
            PINBALL_DAMPING
        } else {
            FLOOR_BOUNCE_DAMPING
        }
    }

    /// 판정의 기준점 — **발밑 중앙**이다 (PRD §5.2).
    ///
    /// 창은 두 화면에 걸쳐도 되지만 규칙은 이 점 하나가 정한다. 좌상단을 쓰면
    /// 화면 오른쪽 끝에서 실제로 서 있는 화면과 판정 화면이 어긋난다.
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
        // 이번 틱의 판정은 **지금 발을 딛고 있는 화면**을 따른다. 이동한 뒤 화면이
        // 바뀌는 경우(경계 넘기)는 아직 다루지 않는다 — 화면이 하나뿐이라 같은 값이다.
        let bounds = self.bounds_in(world);
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
                    // 속도를 0으로 만들기 **전에** 착지 세기를 읽는다
                    match self.landing(self.vy) {
                        Landing::Bounce(vy) => self.vy = vy,
                        Landing::Settle(behavior, hold) => {
                            self.vy = 0.0;
                            self.enter(behavior, now_ms + hold);
                        }
                    }
                }
            }
            Behavior::Swing => {
                if now_ms >= self.behavior_until_ms {
                    if self.air {
                        // 공중에서 휘둘렀다면 이제 마저 떨어진다
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        // 한 번 휘두르고 나면 의기양양하게 약을 올린다
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
            // 반응이라 나가는 길이 `Sassy`와 같다 — 바로 옆에 둔다.
            // **`get_up`(70% 약올리기)을 쓰지 않는다**: 화를 다 낸 직후에 곧바로
            // 킹받게 굴면 방금 낸 화가 연기였던 것처럼 보인다.
            Behavior::Squawk => {
                if now_ms >= self.behavior_until_ms {
                    if self.air {
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
            Behavior::Land | Behavior::Splat | Behavior::Sprawl => {
                if now_ms >= self.behavior_until_ms {
                    self.get_up(now_ms);
                }
            }
            Behavior::Swim => {
                let (tx, ty) = self.target;
                let (dx, dy) = (tx - self.x, ty - self.y);
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
                    if self.y >= bounds.floor_y - ARRIVE_EPSILON {
                        // 바닥까지 내려왔다 — 통, 하고 닿는다
                        self.vy = 0.0;
                        self.enter(Behavior::Land, now_ms + LAND_MS);
                    } else if !self.swim_descending
                        && self.range((0, 99)) < SWIM_FREEFALL_PERCENT
                    {
                        // **대개는 그냥 떨어진다** (`SWIM_FREEFALL_PERCENT`).
                        // 높이 떠 있었으면 낙하 속도가 문턱을 넘어 철푸덕·널브러짐이
                        // 나온다 — 되돌린 결정이고 그게 의도다.
                        //
                        // **이미 내려앉는 중이면 추첨하지 않는다** (`&&`의 앞 항).
                        // 내려오는 도중에 예산이 만료돼 여기 다시 들어오는데, 그때
                        // 다시 뽑으면 난수를 태워 뒤 수열이 밀리고 절반쯤 내려온
                        // 펭귄이 갑자기 자유낙하로 샌다.
                        //
                        // 속도를 물려받지 않는다 — 헤엄은 `vy`를 쓰지 않으므로
                        // 남아 있는 값은 이번 낙하와 무관한 옛날 값이다.
                        self.vy = 0.0;
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        // 나머지는 날개를 저어 내려앉는다 — `MOTIONS.md`의 "내려앉음".
                        // 갈래가 하나 남아야 헤엄의 끝이 예측 가능한 지점이 아니게
                        // 된다 (PRINCIPLE 1).
                        self.target = (self.x, bounds.floor_y);
                        self.swim_descending = true;
                        // **상한도 미룬다.** 안 미루면 다음 틱에 또 이 분기로 들어와
                        // 목적지만 다시 찍고 영영 안 움직인다 (발작에서 겪었다).
                        let 남은 = (bounds.floor_y - self.y).max(0.0);
                        self.behavior_until_ms =
                            now_ms + ((남은 / SWIM_DESCENT_SPEED) * 2_000.0) as u64 + 1_000;
                    }
                } else {
                    let speed = if self.swim_descending {
                        SWIM_DESCENT_SPEED
                    } else {
                        SWIM_SPEED
                    };
                    let step = (speed * dt).min(dist);
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
                    self.vx = -self.vx * self.wall_damping();
                }
                if self.y <= bounds.top && self.vy < 0.0 {
                    self.vy = -self.vy * self.wall_damping();
                }
                if self.vx.abs() > 1.0 {
                    self.facing = if self.vx > 0.0 { Facing::Right } else { Facing::Left };
                }
                // 벽·천장과 마찬가지로 방향 가드가 필요하다. 드래그는 경계 밖으로도
                // 따라가므로(Dock 위 등) 바닥보다 아래에서 놓을 수 있는데, 가드가 없으면
                // 위로 던져도 첫 틱에 "착지"로 삼켜지며 위로 순간이동한다
                if self.y >= bounds.floor_y && self.vy >= 0.0 {
                    self.y = bounds.floor_y;
                    match self.landing(self.vy) {
                        Landing::Bounce(vy) => {
                            self.vy = vy;
                            // 통통 튀며 앞으로도 조금 밀린다 — 제자리에서만 튀면
                            // 던진 방향이 착지에서 끊긴다
                            self.vx *= self.floor_vx_damping();
                        }
                        Landing::Settle(behavior, hold) => {
                            self.vx = 0.0;
                            self.vy = 0.0;
                            self.enter(behavior, now_ms + hold);
                        }
                    }
                }
            }
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

        // 모니터가 바뀌거나 해상도가 달라지면 영역 밖에 남을 수 있다 — 항상 되돌린다
        self.clamp(bounds);
        self.snapshot()
    }

    /// 클릭 — 졸고 있어도 깨워서 놀라게 한다 (R5).
    ///
    /// 공중에서 찔리면 놀라 떨어진다. `Startled`는 지상 동작이라 그대로 넣으면
    /// 같은 step의 clamp가 펭귄을 바닥으로 순간이동시킨다.
    /// 빠따 — 클릭 한 번에 펭귄이 방망이를 한 번 휘두른다. **맞는 쪽이 아니라
    /// 휘두르는 쪽이다.** 제자리에서 휘두르므로 날아가지 않는다 — 날려 보내는 건
    /// 드래그로 던졌을 때(`drag_end`)뿐이다.
    ///
    /// **짧은 간격으로 `SQUAWK_WHACK_COUNT`번 이어지면 빽빽거린다.** 문턱을 넘은
    /// 그 클릭에서 스윙을 건너뛰고 곧바로 터뜨린다 — 스윙 뒤로 미루면 연타
    /// 중에는 매 클릭이 스윙을 다시 걸기 때문에 **연타를 멈춘 뒤에야** 터져서
    /// 자기 손짓과 연결이 안 된다.
    ///
    /// **핀볼 모드에서는 채다.** 펭귄이 공이 되고 커서가 채가 되어, 맞은
    /// 지점(`nx`/`ny`, 펭귄 기준 -0.5~0.5)의 반대 방향으로 날아간다.
    /// 커맨드를 둘로 나누지 않은 이유는 **빠따냐 채냐를 코어가 정해야**
    /// 하기 때문이다 — 프론트가 모드를 알면 설정이 웹뷰로 새어 나간다
    /// (PRINCIPLE 4).
    pub fn whack(&mut self, now_ms: u64, world: &World, nx: f64, ny: f64) {
        self.last_stimulus_ms = now_ms;
        if self.pinball {
            self.flip(now_ms, world, nx, ny);
            return;
        }
        self.whack_seq += 1;
        // 제자리에서 맞는다 — 속도를 주지 않는다
        self.vx = 0.0;
        self.vy = 0.0;

        // 이미 빽빽거리는 중이면 **스윙으로 끊지 않는다.** 매 클릭이 360ms
        // 스윙으로 화를 자르면 화가 보일 시간이 없다.
        //
        // **`behavior`를 보면 안 된다.** 프론트가 모든 pointerdown에서
        // `drag_start`를 부르므로 여기 도달할 때 동작은 이미 `Dragged`이고,
        // 그 검사는 실제 앱에서 한 번도 참이 되지 않는다.
        if now_ms < self.squawk_until_ms {
            self.last_whack_ms = Some(now_ms);
            // **판을 새로 연다.** 원래 종료 시각으로 되돌리면 안 된다 — 클릭
            // 한 번은 `drag_start` → `Dragged` 스냅샷을 웹뷰에 흘리므로 클래스가
            // `pg--squawk` → `pg--dragged` → `pg--squawk`로 오가고, 웹뷰는 그때
            // 1.4초짜리 애니메이션을 **처음부터 다시 재생한다.** 코어가 남은
            // 시간만 주면 부풀다 말고 끊겨, 연타할수록 영원히 부풀기만 하는
            // 펭귄이 된다 — 흡수가 막으려던 바로 그 그림이다.
            //
            // 새로 여는 쪽이 결도 맞는다: **때리는 동안 계속 화낸다.** 손을
            // 떼면 1.4초 뒤에 끝나므로 늘어나는 것은 사용자가 정한다.
            self.enter_squawk(now_ms);
            return;
        }

        self.whack_run = match self.last_whack_ms {
            Some(last) if now_ms.saturating_sub(last) <= SQUAWK_GAP_MS => self.whack_run + 1,
            _ => 1,
        };
        self.last_whack_ms = Some(now_ms);
        if self.whack_run >= SQUAWK_WHACK_COUNT {
            self.enter_squawk(now_ms);
            return;
        }
        // 치켜드는 단계를 두지 않는다 — 클릭하면 바로 휘두른다
        self.enter(Behavior::Swing, now_ms + SWING_MS);
    }

    /// 채로 후려친다 (핀볼 모드).
    ///
    /// **방향은 맞은 지점에서 펭귄 중심을 잇는 벡터다.** 아래를 치면 위로,
    /// 왼쪽을 치면 오른쪽으로 간다. 정확히 중심을 치면 길이가 0이라 그대로
    /// 나누면 NaN이 되고 펭귄이 좌표계 밖으로 사라지므로, 그때는 **바로 위**로 띄운다.
    ///
    /// **`whack_seq`를 올리지 않는다.** 그 값이 늘면 웹뷰가 방망이를 휘두르는데,
    /// 핀볼에서 방망이는 펭귄이 아니라 커서가 들고 있다.
    ///
    /// **연타 카운터도 세지 않는다.** 핀볼에서 스무 번 치는 것은 정상적인
    /// 랠리고, 거기서 제자리에 멈춰 빽빽대면 판이 끊긴다.
    fn flip(&mut self, now_ms: u64, world: &World, nx: f64, ny: f64) {
        let len = (nx * nx + ny * ny).sqrt();
        let (dx, dy) = if len > f64::EPSILON {
            (-nx / len, -ny / len)
        } else {
            (0.0, -1.0)
        };
        let speed = (world.width() * PINBALL_HIT_WORLDS_PER_SEC).max(THROW_MIN_SPEED);
        self.vx = dx * speed;
        self.vy = dy * speed;
        if self.vx.abs() > 1.0 {
            self.facing = if self.vx > 0.0 { Facing::Right } else { Facing::Left };
        }
        self.enter(Behavior::Thrown, now_ms);
    }

    /// 사용자가 시켜서 빽빽거린다 (설정 창의 "빽빽거리기").
    ///
    /// **공중을 허용한다.** 싸가지처럼 고도를 물려받는 반응이라 헤엄치다
    /// 빽빽대도 성립하고, 바닥으로 끌어내리면 헤엄치다 순간이동한다.
    /// 미끄러지기가 공중을 거절하는 것과 갈리는 지점이다 — 그쪽은 바닥과 닿아야
    /// 성립하는 이동이다.
    ///
    /// **이미 빽빽거리는 중이면 거절한다.** 재진입하면 코어는 길이를 늘리는데
    /// 웹뷰는 클래스가 그대로라 애니메이션을 되감지 않는다 (`start_slide`와 같다).
    pub fn start_squawk(&mut self, now_ms: u64) -> bool {
        if matches!(self.behavior, Behavior::Dragged | Behavior::Squawk) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_squawk(now_ms);
        true
    }

    /// 빽빽거리기 한 판을 시작한다. 연타와 "시켜보기"가 이 한 곳을 공유한다 —
    /// 예산과 카운터 초기화를 두 벌로 만들면 한쪽만 고쳐지고 조용히 갈라진다.
    fn enter_squawk(&mut self, now_ms: u64) {
        // 되돌리지 않으면 문턱을 넘은 뒤의 모든 클릭이 다시 문턱을 넘는다
        self.whack_run = 0;
        self.squawk_until_ms = now_ms + SQUAWK_MS;
        self.enter(Behavior::Squawk, self.squawk_until_ms);
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

    /// 넘어졌다 일어난 뒤 — 대체로 약을 올리고, 아니면 그냥 유휴로 간다.
    ///
    /// 착지(`Land`/`Splat`/`Sprawl`)와 굴러떨어지기가 이 출구를 **공유한다.**
    /// 세게 박고 일어난 뒤의 심리가 같으므로 갈래를 두 벌로 만들지 않는다.
    fn get_up(&mut self, now_ms: u64) {
        if self.range((0, 99)) < SASSY_AFTER_LAND_PERCENT {
            self.enter_sassy(now_ms);
        } else {
            self.enter_idle(now_ms);
        }
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
    ///
    /// 세계를 받는 이유는 **속도 상한이 세계의 가로 폭에 비례**하기 때문이다 — 좁은
    /// 화면에서 눈 깜짝할 새 가로지르지 않게, 넓어지면 같은 손짓이 더 멀리 가게.
    pub fn drag_end(&mut self, now_ms: u64, vx: f64, vy: f64, world: &World) {
        self.last_stimulus_ms = now_ms;
        let (vx, vy) = clamp_throw(vx, vy, world.width());
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
        self.swim_descending = false;
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
        // 빽빽거리기 예산은 다른 동작으로 나가는 순간 무효다. 안 그러면 판이
        // 끝나기 전에 던져진 펭귄이 다음 클릭에 날아가다 말고 되돌아온다.
        // **`Dragged`만 예외다** — 프론트가 모든 pointerdown에서 부르므로
        // 클릭 한 번에도 지나가고, 여기서 지우면 흡수가 다시 죽는다.
        if !matches!(behavior, Behavior::Squawk | Behavior::Dragged) {
            self.squawk_until_ms = 0;
        }
        // 반응·드래그는 고도를 그대로 물려받고, 나머지는 동작이 곧 고도를 정한다.
        // 착지(Land)는 바닥에 닿은 시점이라 확실히 지상이다.
        match behavior {
            // 고도를 그대로 물려받는 동작들. 낚시는 **시켜서** 공중에서 시작할 수
            // 있어서 여기 있다 — 저절로 나오는 낚시는 `pick_next`가 바닥에서만 부른다
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
        // **이유 없이 터진다.** 다른 모든 동작은 원인이 설명되는데, 설명이 안 되는
        // 것이 하나도 없으면 결국 예측 가능해진다 (PRINCIPLE 1).
        //
        // **얼음낚시보다 앞이다.** 뒤에 두면 앞 갈래 확률에 한 번 더 깎여 체감이
        // 계산과 어긋나고, 둘 다 뽑혔을 때는 더 드문 쪽이 이겨야 본 보람이 있다.
        // 졸기보다는 뒤다 — 5분 무자극이라는 훨씬 강한 조건이다.
        if !self.air && self.range((0, FREAKOUT_ONE_IN - 1)) == 0 {
            self.enter_freakout(now_ms);
            return;
        }
        // 아주 드물게 낚시를 한다 — 십 분에 한 번쯤 (MOTIONS "빈도 설계").
        // 짧은 동작만 빠르게 갈아 끼우면 펭귄이 안절부절못하는 것처럼 보인다.
        //
        // **졸기 뒤, 헤엄 앞**에 둔다: 졸기가 우선이어야 하고, 헤엄 뒤에 두면
        // 헤엄 확률 30%에 한 번 더 깎여 체감 빈도가 계산과 어긋난다.
        // 바닥 전용이다 — 공중에는 앉을 자리가 없다.
        if !self.air && self.range((0, 999)) < ICE_FISHING_PERMILLE {
            self.enter_ice_fishing(now_ms);
            return;
        }
        // 걷다 말고 배를 깔고 미끄러진다. **걷기 뒤에만** 나온다 — 서 있다가
        // 갑자기 눕는 건 준비 동작이 없다. 걸을 폭이 없으면 미끄러질 자리도 없다.
        if matches!(self.behavior, Behavior::Walk)
            && bounds.right > bounds.left
            && self.range((0, 99)) < SLIDE_AFTER_WALK_PERCENT
        {
            self.enter_slide(now_ms);
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
    use super::test_support::*;

    // ── 슬라이딩 ──────────────────────────────────────────────────

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
        // 시드가 다르면 시퀀스도 달라야 난수가 실제로 쓰이는 것이다
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
    fn 들어_올렸다_놓으면_여전히_떨어진다() {
        // 위 수정이 드래그를 망가뜨리지 않았는지 반대편에서 고정한다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        assert_eq!(p.behavior(), Behavior::Falling);
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 8_000 {
            t += 50;
            p.step(t, &world());
        }
        assert!(p.behavior().is_landing());
    }

    #[test]
    fn 휘둘러도_날아가지_않는다() {
        // 나는 건 드래그로 던졌을 때뿐이다 — 클릭으로 날아가면 안 된다
        let mut p = pet();
        p.step(1_000, &world());
        let before = p.snapshot();
        p.whack(1_000, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭하면 바로 휘두른다");

        let mut t = 1_000;
        for _ in 0..30 {
            t += 50;
            let s = p.step(t, &world());
            assert_eq!(s.x, before.x, "옆으로 밀리면 안 된다");
            assert_eq!(s.y, before.y, "떠오르면 안 된다");
            assert_ne!(s.behavior, Behavior::Thrown, "던져진 상태가 되면 안 된다");
        }
    }

    #[test]
    fn 휘두르고_나면_약을_올린다() {
        let mut p = pet();
        p.step(1_000, &world());
        p.whack(1_000, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
        let after = p.step(1_000 + SWING_MS + 20, &world());
        assert!(
            matches!(after.behavior, Behavior::Sassy { .. }),
            "휘두르고 나면 약이 올라야 한다 (실제: {:?})",
            after.behavior
        );
    }

    #[test]
    fn 빠따는_한_번에_한_번씩_횟수가_는다() {
        // 웹뷰가 방망이를 몇 번 휘두를지 이 값으로 안다. 연타해도 매번 보여야 한다
        let mut p = pet();
        assert_eq!(p.snapshot().whack_seq, 0);
        for i in 1..=5u64 {
            p.whack(1_000 + i * 100, &world(), 0.0, 0.0);
            assert_eq!(p.snapshot().whack_seq, i, "{i}번째 빠따가 안 세어졌다");
        }
    }

    #[test]
    fn 던져서_나는_중에_휘둘러도_그_자리에서_마저_떨어진다() {
        // 때리는 것으로는 새 속도가 붙지 않는다 — 나는 건 던지기 전용이다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 600.0, -400.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown);

        // 아직 공중일 때 때린다
        let mut t = 1_100;
        for _ in 0..4 {
            t += 50;
            p.step(t, &world());
        }
        assert_eq!(p.behavior(), Behavior::Thrown, "아직 나는 중이어야 한다");
        assert!(p.snapshot().air, "공중 상태여야 한다");

        p.whack(t, &world(), 0.0, 0.0);
        let hit_y = p.snapshot().y;
        assert_eq!(p.behavior(), Behavior::Swing);
        t += 50;
        let swinging = p.step(t, &world());
        assert_eq!(swinging.y, hit_y, "휘두른다고 솟아오르거나 떨어지면 안 된다");

        // 움찔이 끝나면 마저 떨어진다
        let after = p.step(t + SWING_MS + 20, &world());
        assert_eq!(after.behavior, Behavior::Falling, "공중이었으니 마저 떨어진다");
    }

    #[test]
    fn 빠따는_졸고_있어도_깨운다() {
        let mut p = pet();
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, &world());
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep);
        p.whack(t, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
    }

    #[test]
    fn 휘두른다고_말하지는_않는다() {
        // 말은 클릭이 아니라 시간에 맞춰 나온다 — 때릴 때마다 떠들면 시끄럽다
        let mut p = pet();
        p.whack(1_000, &world(), 0.0, 0.0);
        assert!(p.snapshot().speech.is_none(), "클릭으로 말이 나오면 안 된다");
        p.whack(1_100, &world(), 0.0, 0.0);
        p.whack(1_200, &world(), 0.0, 0.0);
        assert!(p.snapshot().speech.is_none(), "연타해도 마찬가지다");
    }

    // ── 빽빽거리기 ─────────────────────────────────────────────────

    /// 연타로 빽빽거리게 만든 펭귄과 터진 시각.
    fn 빽빽거리는_펭귄() -> (Pet, u64) {
        let mut p = pet();
        p.step(1_000, &world());
        let mut t = 1_000;
        for _ in 0..SQUAWK_WHACK_COUNT {
            t += 150;
            클릭(&mut p, t);
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "연타로 터져야 한다");
        (p, t)
    }

    #[test]
    fn 짧은_간격으로_스무_번_맞으면_빽빽거린다() {
        let mut p = pet();
        p.step(1_000, &world());
        let mut t = 1_000;
        for i in 1..=SQUAWK_WHACK_COUNT {
            t += 150;
            클릭(&mut p, t);
            if i < SQUAWK_WHACK_COUNT {
                assert_eq!(p.behavior(), Behavior::Swing, "{i}번째까지는 휘두른다");
            }
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "문턱을 넘은 클릭에서 터진다");
    }

    #[test]
    fn 띄엄띄엄_때리면_빽빽거리지_않는다() {
        // 한두 번 툭 치는 것으로 터지면 "연타에 대한 반응"이 아니게 된다
        let mut p = pet();
        let mut t = 1_000;
        for _ in 0..6 {
            t += SQUAWK_GAP_MS + 500;
            클릭(&mut p, t);
            assert_eq!(p.behavior(), Behavior::Swing, "간격이 벌어지면 그냥 휘두른다");
        }
    }

    #[test]
    fn 문턱_직전까지는_안_터지고_한_번_더_때리면_터진다() {
        // 경계가 `>`인지 `>=`인지 한 칸만 어긋나도 문턱이 통째로 밀린다.
        // **양쪽을 다 재야** 한다 — "아직 안 터진다"만 재면 문턱을 올리는 순간
        // 조용히 무의미해진다(실제로 4에서 20으로 올렸을 때 그렇게 됐다).
        let mut p = pet();
        let mut t = 300;
        for _ in 1..SQUAWK_WHACK_COUNT {
            클릭(&mut p, t);
            t += 100;
        }
        assert_eq!(p.behavior(), Behavior::Swing, "문턱 직전까지는 휘두른다");
        클릭(&mut p, t);
        assert_eq!(p.behavior(), Behavior::Squawk, "한 번 더 때리면 터진다");
    }

    #[test]
    fn 빽빽거리는_중에_맞아도_끊기지_않는다() {
        // 매 클릭이 360ms 스윙으로 화를 자르면 화가 보일 시간이 없다.
        // 대신 판이 새로 열리므로 **때리는 동안 계속 화낸다.**
        let (mut p, t) = 빽빽거리는_펭귄();
        클릭(&mut p, t + 200);
        assert_eq!(p.behavior(), Behavior::Squawk, "스윙으로 끊기면 안 된다");
        클릭(&mut p, t + 400);
        assert_eq!(p.behavior(), Behavior::Squawk);
        // 마지막 클릭에서 판이 새로 열렸으므로 거기서부터 한 판을 더 채운다 —
        // 웹뷰가 애니메이션을 되감는 것과 길이를 맞추기 위해서다
        let mid = p.step(t + 400 + SQUAWK_MS - 50, &world());
        assert_eq!(mid.behavior, Behavior::Squawk, "새 판이 아직 안 끝났다");
        let after = p.step(t + 400 + SQUAWK_MS + 20, &world());
        assert_ne!(after.behavior, Behavior::Squawk, "손을 떼면 제 시간에 끝난다");
    }

    #[test]
    fn 빽빽거리는_중에_맞은_것은_다음_연타로_세지_않는다() {
        // 세면 끝나자마자 한 번 더 터진다
        let (mut p, t) = 빽빽거리는_펭귄();
        for i in 1..=3 {
            클릭(&mut p, t + i * 100);
        }
        // 마지막 클릭에서 열린 판이 끝날 때까지 진행시킨다
        let end = t + 300 + SQUAWK_MS + 20;
        p.step(end, &world());
        클릭(&mut p, end + 40);
        assert_eq!(p.behavior(), Behavior::Swing, "카운터가 초기화돼야 한다");
    }

    #[test]
    fn 빽빽거리는_동안_제자리에_있다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        let before = p.snapshot();
        let mut now = t;
        for _ in 0..10 {
            now += 50;
            let s = p.step(now, &world());
            assert_eq!(s.x, before.x, "옆으로 움직이면 안 된다");
            assert_eq!(s.y, before.y, "떠오르거나 가라앉으면 안 된다");
        }
    }

    #[test]
    fn 빽빽거리기가_끝나면_유휴로_간다() {
        // 약을 올리며 나가지 않는다 — 화를 다 낸 뒤에 곧바로 킹받게 굴면
        // 방금 낸 화가 연기였던 것처럼 보인다
        let (mut p, t) = 빽빽거리는_펭귄();
        let after = p.step(t + SQUAWK_MS + 20, &world());
        assert!(
            matches!(after.behavior, Behavior::Idle { .. }),
            "유휴로 나가야 한다 (실제: {:?})",
            after.behavior
        );
    }

    #[test]
    fn 공중에서_빽빽거리면_끝나고_떨어진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        assert!(p.snapshot().air, "공중이어야 한다");

        let mut t = 1_100;
        for _ in 0..SQUAWK_WHACK_COUNT {
            t += 150;
            p.whack(t, &world(), 0.0, 0.0);
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "공중에서도 터진다");
        assert!(p.snapshot().air, "고도를 물려받아야 한다");

        let after = p.step(t + SQUAWK_MS + 20, &world());
        assert_eq!(after.behavior, Behavior::Falling, "공중이었으니 마저 떨어진다");
    }

    #[test]
    fn 빽빽거리다_던져지면_되돌아오지_않는다() {
        // 흡수를 시각으로 판정하므로, 예산이 남은 채 다른 동작으로 나갔다가
        // 클릭이 오면 날아가던 펭귄이 갑자기 빽빽거리며 멈출 수 있다
        let (mut p, t) = 빽빽거리는_펭귄();
        p.drag_start(t + 100);
        p.drag_by(120.0, -80.0);
        p.drag_end(t + 200, 900.0, -600.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown, "던져진 상태여야 한다");

        클릭(&mut p, t + 300);
        assert_ne!(p.behavior(), Behavior::Squawk, "예산은 나가는 순간 무효다");
    }

    #[test]
    fn 빽빽거리는_중에_들어_올릴_수_있다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        p.drag_start(t + 100);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 빽빽거리기는_제자리_동작이다() {
        assert!(!Behavior::Squawk.is_airborne(), "스스로 뜨지 않는다");
        assert!(!Behavior::Squawk.is_landing(), "바닥에 닿아서 생긴 게 아니다");
        // 1.4초짜리 동작이 500ms 느린 틱을 받으면 시작·종료가 눈에 띄게 밀린다
        assert!(Behavior::Squawk.moves_window(), "틱을 빠르게 유지해야 한다");
    }

    #[test]
    fn 시키면_바로_빽빽거린다() {
        let mut p = pet();
        p.step(1_000, &world());
        assert!(p.start_squawk(1_000));
        assert_eq!(p.behavior(), Behavior::Squawk);
    }

    #[test]
    fn 공중에서도_시키면_빽빽거린다() {
        // 싸가지처럼 고도를 물려받는 반응이라 헤엄치다 빽빽대도 성립한다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        assert!(p.start_squawk(1_150));
        assert_eq!(p.behavior(), Behavior::Squawk);
        assert!(p.snapshot().air, "바닥으로 끌어내리면 순간이동한다");
    }

    #[test]
    fn 들려_있거나_이미_빽빽거리면_시켜도_안_한다() {
        let mut p = pet();
        p.drag_start(1_000);
        assert!(!p.start_squawk(1_050), "손에 쥔 채로는 안 된다");

        let (mut q, t) = 빽빽거리는_펭귄();
        assert!(!q.start_squawk(t + 100), "재진입하면 웹뷰가 되감지 못한다");
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

    #[test]
    fn 드래그_중에는_자율_이동이_멈추고_주어진_위치를_따른다() {
        let mut p = pet();
        p.drag_start(1_000);
        let before = p.snapshot();

        // 자율 이동이 없어야 한다
        let s = p.step(2_000, &world());
        assert_eq!(s.x, before.x);
        assert_eq!(s.behavior, Behavior::Dragged);

        // 드래그 이동량은 그대로 반영된다
        p.drag_by(100.0, -200.0);
        let moved = p.step(2_100, &world());
        assert_eq!(moved.x, before.x + 100.0);
        assert_eq!(moved.y, before.y - 200.0);
    }

    #[test]
    fn 드래그는_영역_밖으로도_따라가고_놓을_때_정산한다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(5_000.0, -500.0);
        // 들고 있는 동안에는 clamp하지 않는다 — 사용자가 끄는 대로 간다
        assert_eq!(p.step(1_100, &world()).x, BOUNDS.left + 5_000.0);

        p.drag_end(1_200, 0.0, 0.0, &world());
        let s = p.step(1_300, &world());
        assert_eq!(s.x, BOUNDS.right, "놓으면 영역 안으로 정산된다");
    }

    #[test]
    fn 드래그를_놓으면_낙하해_바닥에서_멈춘다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -400.0);
        p.step(1_100, &world());
        p.drag_end(1_200, 0.0, 0.0, &world());
        assert_eq!(p.behavior(), Behavior::Falling);

        let mut t = 1_200;
        while p.behavior() == Behavior::Falling && t < 6_000 {
            t += 50;
            p.step(t, &world());
        }
        assert!(p.behavior().is_landing(), "바닥에 닿으면 착지한다");
        assert_eq!(p.snapshot().y, BOUNDS.floor_y);
    }

    #[test]
    fn 걸어다닐_폭이_없는_화면에서도_영원히_돌지_않는다() {
        // 작업 영역이 펭귄보다 좁으면 양쪽 경계가 겹쳐 매 step이 Turn이 된다
        let narrow = World::single(Bounds { left: 10.0, right: 10.0, top: 0.0, floor_y: 50.0 });
        let mut p = Pet::new(5, 0, &narrow);
        let seen = drive(&mut p, 100, 40_000, 250, &narrow);
        assert!(
            seen.iter().any(|s| !matches!(s.behavior, Behavior::Turn)),
            "회전 말고 다른 동작으로 넘어가야 한다"
        );
        assert!(seen.iter().all(|s| s.x == narrow.first().bounds.left));
    }

    #[test]
    fn 헤엄을_치면_바닥에서_떠오른다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
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
        for s in drive(&mut p, 100, 120_000, 100, &world()) {
            assert!(s.x >= BOUNDS.left && s.x <= BOUNDS.right, "x가 벗어났다: {}", s.x);
            assert!(s.y >= BOUNDS.top && s.y <= BOUNDS.floor_y, "y가 벗어났다: {}", s.y);
        }
    }

    #[test]
    fn 올라갈_때와_내려갈_때의_세로_방향이_다르다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
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
        p.step(1_050, &world());
        // 오른쪽 위로 세게 던진다
        p.drag_end(1_100, 700.0, -400.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown);

        let start_x = p.snapshot().x;
        let mut ys = Vec::new();
        let mut t = 1_100;
        while p.behavior() == Behavior::Thrown && t < 12_000 {
            t += 50;
            ys.push(p.step(t, &world()).y);
        }
        assert!(p.behavior().is_landing(), "결국 착지해야 한다");
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
            p.drag_end(1_000, vx, -200.0, &world());
            let start = p.snapshot().x;
            let mut t = 1_000;
            while p.behavior() == Behavior::Thrown && t < 12_000 {
                t += 50;
                p.step(t, &world());
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
        p.step(1_050, &world());
        let x = p.snapshot().x;
        p.drag_end(1_100, 20.0, 5.0, &world());
        assert_eq!(p.behavior(), Behavior::Falling);

        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 12_000 {
            t += 50;
            p.step(t, &world());
        }
        assert!((p.snapshot().x - x).abs() < 1.0, "좌우로 날아가면 안 된다");
    }

    #[test]
    fn 바닥보다_아래에서_위로_던져도_삼켜지지_않는다() {
        // 드래그는 경계 밖으로도 따라가므로(Dock 위 등) 바닥보다 아래에서 놓을 수 있다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, 90.0); // 바닥보다 90px 아래로 끌어내림
        p.step(1_050, &world());
        p.drag_end(1_100, 700.0, -400.0, &world()); // 오른쪽 위로 세게
        assert_eq!(p.behavior(), Behavior::Thrown);

        let first = p.step(1_150, &world());
        assert_eq!(
            first.behavior,
            Behavior::Thrown,
            "위로 던졌는데 첫 틱에 착지로 삼켜졌다"
        );
        assert!(first.y > BOUNDS.floor_y - 1.0, "위로 순간이동하면 안 된다");
    }

    /// 폭 1440 화면의 상한. KTD2의 비율(0.9)이 바뀌면 이 값도 함께 움직인다.
    fn 상한(width: f64) -> f64 {
        throw_max_speed(width)
    }

    // ---- 여러 마리 (Pets) ----

    fn pets_with_one() -> Pets {
        let mut pets = Pets::new();
        pets.add(1, 0, &world(), BOUNDS.left).expect("첫 마리는 들어간다");
        pets
    }

    // ---- 철푸덕 착지 ----

    /// 지정한 높이에서 떨어뜨려 착지 동작을 본다.
    fn 떨어뜨려_착지시킨다(drop_height: f64) -> Behavior {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -drop_height);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world()); // 살짝 놓는다 — 낙하만 시킨다
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        p.behavior()
    }

    #[test]
    fn 세게_떨어지면_철푸덕한다() {
        // 350px ≈ 착지 794px/s — 철푸덕 구간(700~1000)
        assert_eq!(떨어뜨려_착지시킨다(350.0), Behavior::Splat);
    }

    #[test]
    fn 아주_세게_떨어지면_널브러진다() {
        // 700px ≈ 착지 1122px/s — 널브러짐 구간(1000~)
        assert_eq!(떨어뜨려_착지시킨다(700.0), Behavior::Sprawl);
    }

    #[test]
    fn 살짝_떨어지면_그냥_선다() {
        // 거의 멈춘 채 닿으면 튀지도 퍼지지도 않는다
        assert_eq!(떨어뜨려_착지시킨다(5.0), Behavior::Land);
    }

    #[test]
    fn 어중간하게_떨어지면_통통_튄다() {
        // 바닥에 닿은 뒤 **다시 떠올라야** 통통이다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -60.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        let mut 닿았다 = false;
        let mut 다시_떠올랐다 = false;
        while p.behavior() == Behavior::Falling && t < 20_000 {
            t += 20;
            let s = p.step(t, &world());
            if s.y >= BOUNDS.floor_y {
                닿았다 = true;
            } else if 닿았다 {
                다시_떠올랐다 = true;
            }
        }
        assert!(닿았다 && 다시_떠올랐다, "바닥을 치고 다시 떠야 통통이다");
    }

    #[test]
    fn 통통은_몇_번_만에_멈춘다() {
        // 감쇠가 모자라면 공처럼 영원히 튄다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 12_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(p.behavior().is_landing(), "12초 안에 서야 한다 — {:?}", p.behavior());
    }

    #[test]
    fn 아래로_내리꽂으면_널브러진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -600.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 200.0, 900.0, &world()); // 아래로 세게
        let mut t = 1_100;
        while matches!(p.behavior(), Behavior::Thrown) && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert_eq!(p.behavior(), Behavior::Sprawl);
    }

    #[test]
    fn 던져서_세게_박아도_철푸덕한다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -600.0);
        p.step(1_050, &world());
        // 살짝 아래로 — 낙하 중 가속을 더해 철푸덕 구간에 들어온다
        p.drag_end(1_100, 300.0, 120.0, &world());
        let mut t = 1_100;
        while matches!(p.behavior(), Behavior::Thrown) && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(
            p.behavior().is_landing() && p.behavior() != Behavior::Land,
            "세게 박았으면 그냥 서면 안 된다 — {:?}",
            p.behavior()
        );
    }

    #[test]
    fn 철푸덕이_끝나면_평소_동작으로_돌아온다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -350.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() != Behavior::Splat && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        let 철푸덕_시작 = t;
        while p.behavior() == Behavior::Splat && t < 철푸덕_시작 + 10_000 {
            t += 20;
            p.step(t, &world());
        }
        assert_ne!(p.behavior(), Behavior::Splat, "영영 퍼져 있으면 안 된다");
        assert!(t - 철푸덕_시작 >= SPLAT_MS, "너무 빨리 일어난다");
    }

    #[test]
    fn 철푸덕_중에는_공중_상태가_아니다() {
        // 착지는 바닥에 닿은 시점이라 확실히 지상이다
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -350.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() != Behavior::Splat && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(!p.snapshot().air);
    }

    #[test]
    fn 같은_순간에_태어나도_첫마디_시각이_다르다() {
        // 시작할 때 저장된 마릿수만큼 한꺼번에 만든다. 첫 한마디까지가 고정값이면
        // 앱을 켤 때마다 전부 한목소리로 떠든다.
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
        // 같은 시각·같은 경계로 나란히 돌린다. 시드가 같으면 영원히 붙어 다닌다.
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
    fn 좁은_화면에서는_던지기_상한이_더_낮다() {
        let 좁은_곳 = 상한(1_440.0);
        let 넓은_곳 = 상한(2_880.0);
        assert!(
            (넓은_곳 - 좁은_곳 * 2.0).abs() < 1.0,
            "상한은 세계 폭에 비례해야 한다 — 좁은 곳 {좁은_곳}, 넓은 곳 {넓은_곳}"
        );
    }

    #[test]
    fn 상한_이하의_던지기는_속도가_그대로다() {
        let (vx, vy) = clamp_throw(400.0, -300.0, 1_440.0);
        assert!((vx - 400.0).abs() < f64::EPSILON);
        assert!((vy + 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn 상한은_방향을_유지한_채_속도만_줄인다() {
        let (vx, vy) = clamp_throw(30_000.0, -40_000.0, 1_440.0);
        let speed = (vx * vx + vy * vy).sqrt();
        assert!((speed - 상한(1_440.0)).abs() < 1.0, "상한까지 잘려야 한다");
        // 원래 비 3:-4가 보존된다
        assert!((vx / speed - 0.6).abs() < 1e-6);
        assert!((vy / speed + 0.8).abs() < 1e-6);
    }

    #[test]
    fn 화면_폭을_읽지_못하면_기본_폭으로_상한을_잡는다() {
        // 모니터 조회에 실패하면 브릿지가 폭 0인 납작한 경계를 준다. 그대로
        // 비례식에 넣으면 상한이 0이 되어 모든 던지기가 낙하로 바뀐다.
        let flat = World::single(Bounds {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            floor_y: 0.0,
        });
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_end(1_000, 900.0, -500.0, &flat);
        assert_eq!(p.behavior(), Behavior::Thrown, "던지기가 조용히 죽으면 안 된다");
    }

    #[test]
    fn 세계가_너무_좁아도_던지기_문턱_아래로_내려가지_않는다() {
        assert!(
            상한(100.0) >= THROW_MIN_SPEED,
            "상한이 최소 속도보다 낮으면 아무리 세게 던져도 던져지지 않는다"
        );
    }

    #[test]
    fn 던지기_문턱은_화면_폭이_달라져도_같다() {
        // 문턱은 "사용자가 튕겼는가"라는 손의 의도에 대한 것이라 세계와 무관하다 (KTD1)
        let 넓은_세계 = World::single(Bounds {
            left: 0.0,
            right: 4_000.0,
            ..BOUNDS
        });
        for w in [world(), 넓은_세계] {
            let mut p = pet();
            p.drag_start(1_000);
            p.drag_end(1_100, 20.0, 5.0, &w);
            assert_eq!(p.behavior(), Behavior::Falling, "살짝 놓으면 어디서든 떨어진다");
        }
    }

    #[test]
    fn 던지기_속도는_상한을_넘지_않는다() {
        let mut p = pet();
        p.drag_start(1_000);
        // 비정상적으로 큰 속도가 들어와도 화면을 순간이동하지 않아야 한다
        p.drag_end(1_000, 500_000.0, -500_000.0, &world());
        let first = p.step(1_050, &world());
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
        let s = p.step(1_000, &World::single(narrow));
        assert!(s.x <= narrow.right, "좁아진 영역 안으로 들어와야 한다");
        assert_eq!(s.y, narrow.floor_y, "바닥도 새 영역을 따른다");
    }

    // ── 세계(다중 화면 좌표계) ─────────────────────────────────────────

    #[test]
    fn 기준점은_펭귄_발밑_중앙이다() {
        let mut p = pet();
        p.x = 300.0;
        p.y = 400.0;
        assert_eq!(p.anchor(), (300.0 + PET_SIZE / 2.0, 400.0 + PET_SIZE));
    }

}





