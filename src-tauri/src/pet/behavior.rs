//! 모션 카탈로그 — [`Behavior`]와 그 안에서 갈리는 국면 enum들.
//!
//! "이 앱에 어떤 동작이 있나"의 답이 여기 하나다. 각 배리언트의 문서 주석이
//! **왜 그 동작이 있는지**를 적는다 (`MOTIONS.md`가 더 긴 판이다).
//!
//! # 모션 하나는 일곱 자리에 흩어져 있다
//!
//! 이 파일은 그중 **첫 자리**일 뿐이다. 새 모션을 얹거나 있는 모션을 고칠 때
//! 빠뜨리기 쉬운 나머지를 여기 적어 둔다 — `Slide`(슬라이딩)를 예로 들면:
//!
//! | # | 자리 | `Slide`의 경우 |
//! |---|---|---|
//! | 1 | 이 파일의 [`Behavior`] 배리언트 | `Behavior::Slide` |
//! | 2 | `step.rs`의 `match` 팔 — 매 틱 물리 | 남은 시간 비율로 감속 |
//! | 3 | 진입 규칙 | `pick_next`에서 걷기 뒤 `SLIDE_AFTER_WALK_PERCENT` |
//! | 4 | 퇴장 규칙 | `hit_wall` 공유 → `enter_idle` |
//! | 5 | `tuning.rs`의 상수 | `SLIDE_MS`·`SLIDE_SPEED`·`SLIDE_AFTER_WALK_PERCENT` |
//! | 6 | 웹뷰 — `src/lib/pet.ts`의 `behaviorClass` → `pet.css` | `pg--slide` + `@keyframes` |
//! | 7 | `src/pet/pet-css.test.ts`의 `ALL_BEHAVIORS` | 대조 목록에 한 줄 |
//!
//! 여섯째와 일곱째를 빠뜨려도 **Rust는 아무 말도 하지 않는다** — 펭귄이 그
//! 동작 동안 반응 없이 서 있을 뿐이라 눈으로만 잡힌다. 코어와 CSS의 길이가
//! 어긋나는 것도 마찬가지다(자세가 튀거나, 이미 일어나 걷는데 아직 누워 있다).
//! `pet-css.test.ts`가 그 조용한 실패를 막으려고 소스를 문자열로 대조한다.
//!
//! 사용자가 모션을 직접 만들게 하려면 **이 일곱을 전부 데이터로 바꿔야 한다.**
//! 지금은 Rust 제어 흐름이라 그럴 수 없고, 그래서 확장점을 미리 파 두지
//! 않았다 — 모양이 미결 질문(서술자냐 코드냐, CSS는 누가 소유하나,
//! PRINCIPLE 3의 결정성이 사용자 저작물에서 어떻게 살아남나)에 달려 있다.

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    // 아래 둘은 `pub(super)`다 — 상태머신(부모 모듈)이 부르는데, 부모는 자식의
    // 비공개 항목을 볼 수 없다. 타입을 자식 모듈로 내리면 따라오는 기계적
    // 결과이지 배치가 틀렸다는 신호가 아니다. `pet` 밖으로는 여전히 안 나간다.
    pub(super) fn flipped(self) -> Self {
        match self {
            Facing::Left => Facing::Right,
            Facing::Right => Facing::Left,
        }
    }

    /// x축 진행 부호.
    pub(super) fn sign(self) -> f64 {
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

/// 얼음낚시 한 판이 거쳐 가는 국면.
///
/// 국면을 코어가 갖는 이유는 **잡았나 꽝인가가 "무슨 동작"이기 때문**이다
/// (PRINCIPLE 4). 웹뷰가 뽑으면 같은 시드가 같은 결과를 내지 않는다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FishingPhase {
    /// 얼음에 구멍을 뚫는다
    Dig,
    /// 드리우고 기다린다 — 이 국면만 길고 반복이다
    Wait,
    /// 찌가 까딱한다. 홱 챈다
    Bite,
    /// 물고기가 딸려 나왔다. 자랑하고 그 판을 접는다
    Catch,
    /// 꽝. 시무룩하게 다시 드리운다
    Miss,
    /// 예산이 다 됐다. 낚싯대를 접고 일어난다 — 모든 판이 이 국면으로 끝난다
    Pack,
}

/// 발작 한 판이 거쳐 가는 국면.
///
/// **얼음낚시와 같은 구조다.** 국면을 둘로 나누면 두 문제가 한 번에 풀린다 —
/// 판 길이가 2~4초 난수라 CSS 길이 대조를 쓸 수 없는데 `Dash`는 무한 반복이라
/// 대조 대상이 아니고, 떨던 자세에서 곧장 유휴로 가면 `.pg-all`에 걸린 변형이
/// 한 프레임에 사라져 펭귄이 튀는 것을 `Pant`가 막는다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FreakoutPhase {
    /// 사방으로 마구 튄다
    Dash,
    /// 바닥에서 숨을 고른다 — 모든 판이 이 국면으로 끝난다
    Pant,
}

pub(super) const SASSY_KINDS: [SassyKind; 5] = [
    SassyKind::TurnAway,
    SassyKind::HeadFlick,
    SassyKind::WingFlick,
    SassyKind::EyeRoll,
    SassyKind::ButtWiggle,
];

pub(super) const IDLE_KINDS: [IdleKind; 4] = [
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
    /// 빽빽거리기 — 짧은 시간에 여러 번 맞으면 몸을 부풀리고 날개를 퍼덕이며
    /// 정면으로 화낸다. 싸가지 다섯이 전부 "무시하는" 결이라, 대비를 주는 것이
    /// 이 동작의 존재 이유다. **소리는 내지 않는다** (PRD §5.5).
    Squawk,
    /// 방망이를 휘두르는 중. **제자리에서** 휘두른다 — 나는 건 드래그로 던졌을 때뿐이다
    Swing,
    /// 사용자가 집어 든 상태 — 자율 이동을 하지 않는다 (R6)
    Dragged,
    Falling,
    /// 던져져 포물선을 그리는 중 — 좌우 속도를 갖는다는 점이 Falling과 다르다 (R12)
    Thrown,
    /// 착지 스쿼시 — 살짝 떨어졌을 때
    Land,
    /// 철푸덕 — 세게 떨어져 배를 깔고 납작하게 퍼진다. 높이와 무관하게 착지가
    /// 하나뿐이면 세게 던진 보람이 착지에서 사라진다.
    Splat,
    /// 널브러짐 — 아주 세게 박아 바닥에 아예 뻗는다. 한참 있다 겨우 일어난다.
    Sprawl,
    /// 굴러떨어지기 — 벽에 박고 반동으로 데굴 굴러 나자빠진다.
    /// 착지 4단계와 달리 **바닥이 아니라 벽**에서 생긴다.
    Tumble,
    /// 슬라이딩 — 배를 깔고 미끄러진다. 걷기보다 빠르고 멀리 가며,
    /// 멈출 때 바로 서지 않고 주르륵 밀린다. 지상 이동에 완급을 준다.
    Slide,
    /// 발작 — 며칠에 한 번 이유 없이 터지는 광란. 사방으로 마구 튀다가 바닥으로
    /// 돌아와 숨을 고르고 **아무 일 없었다는 듯** 평소로 돌아간다.
    /// 원인이 없는 것이 이 동작의 정의다 — 원인이 있으면 화(`Squawk`)다.
    Freakout { freakout: FreakoutPhase },
    /// 얼음낚시 — 바닥에 앉아 구멍을 뚫고 드리운다. 30~60초로 이 앱에서
    /// 가장 긴 동작이고, **안에서 갈래가 갈리는 첫 동작**이다 (잡음/꽝).
    IceFishing { fishing: FishingPhase },
}

impl Behavior {
    /// 창을 실제로 옮겨야 하는 동작인가. 졸기는 아니다 (R10).
    pub fn moves_window(self) -> bool {
        !matches!(self, Behavior::Sleep)
    }

    /// 바닥에 닿은 직후의 동작인가. 세기에 따라 스쿼시(`Land`)와 철푸덕(`Splat`)으로
    /// 갈리지만, "착지했다"를 묻는 쪽에서는 둘을 구분할 필요가 없다.
    pub fn is_landing(self) -> bool {
        matches!(self, Behavior::Land | Behavior::Splat | Behavior::Sprawl)
    }

    /// 스스로 고도를 만드는 동작인가 (진입하면 공중 상태가 된다).
    pub fn is_airborne(self) -> bool {
        matches!(
            self,
            Behavior::Swim
                | Behavior::Falling
                | Behavior::Thrown
                // 사방으로 튀려면 떠 있어야 한다. 숨 고르기는 바닥이므로 빠진다 —
                // 그러면 `enter()`의 기본 갈래가 국면마다 옳게 동작한다.
                | Behavior::Freakout { freakout: FreakoutPhase::Dash }
        )
    }
}