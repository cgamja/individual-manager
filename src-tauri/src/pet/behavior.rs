//! 펭귄이 할 수 있는 동작 목록과 동작 안의 국면.
//!
//! 모션 하나를 얹으려면 일곱 곳을 건드린다: 여기(`Behavior`),
//! `motion/*.rs`(매 틱 물리), `pick_next`(진입), 퇴장, `tuning.rs`(상수),
//! `pet.css`의 `pg--*`, `pet-css.test.ts`의 `ALL_BEHAVIORS`.

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