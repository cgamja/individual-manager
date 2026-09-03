//! 펭귄이 할 수 있는 동작 목록과 동작 안의 국면.
//!
//! 모션 하나를 얹으려면 일곱 곳을 건드린다: 여기(`Behavior`),
//! `motion/*.rs`(매 틱 물리), `pick_next`(진입), 퇴장, `tuning.rs`(상수),
//! `src/pet/css/<도메인>.css`의 `pg--*`, `pet-css.test.ts`의 `ALL_BEHAVIORS`.

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
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

/// 볼링 한 판에서 **마리 하나가** 거쳐 가는 국면. 판 전체의 국면은 따로 있다
/// (`bowling::BoardPhase`) — 나눈 이유는 "전부 섰는가"를 물어볼 자리가
/// 필요해서다 (KTD8).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BowlingPhase {
    /// 자기 핀 자리로 걸어간다
    Gather,
    /// 자리에 떠서 공을 기다린다
    Ready,
    /// 안 맞은 채로 판이 끝났다. 흩어져 돌아간다
    Scatter,
}

/// 비치발리볼 한 판에서 **마리 하나가** 거쳐 가는 국면. 판 전체의 국면은 따로
/// 있다 (`volleyball::CourtPhase`) — 볼링과 같은 두 층 구조다.
///
/// **스스로 끝나는 것은 `Gather`·`Bump`·`Cheer`·`Sulk`뿐이다.** `Ready`와 `Chase`는
/// 판이 몰아 준다 — "다음 공이 어디로 오나"는 마리 혼자서는 답할 수 없다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VolleyPhase {
    /// 화면 가운데의 자기 자리로 **날아간다.** 유일하게 공중인 국면이다
    Gather,
    /// 코트에 서서 공을 본다
    Ready,
    /// 공이 떨어질 자리로 **뛴다** — 랠리 화면의 절반이 이 국면이다
    Chase,
    /// 공을 때린다. 뛰어오르는 그림은 CSS가 그리고 좌표는 안 바뀐다
    Bump,
    /// 이겼다 — 엉덩이를 흔든다 (`Sassy`의 그림을 CSS에서 재사용한다)
    Cheer,
    /// 졌다 — 등을 홱 돌린다. `Cheer`와 함께 이 동작의 귀결 국면이다
    Sulk,
}

/// 단체 야차 한 판에서 **마리 하나가** 거쳐 가는 국면. 판 전체의 국면은 따로
/// 있다 (`yacha::RingPhase`) — 볼링·비치발리볼과 같은 두 층 구조다.
///
/// **스스로 끝나는 것은 `Gather`·`Punch`·`Hurt`·`Win`뿐이다.** `Guard`는 다음
/// 라운드를 기다리고, `Down`은 판이 끝날 때까지 누워 있고, `Champ`는 세레모니가
/// 끝나야 나간다 — 셋 다 마리 혼자서는 언제 끝나는지 알 수 없다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum YachaPhase {
    /// 한가운데로 **날아온다**
    Gather,
    /// 상대에게 다가간다
    Hunt,
    /// 상대를 축으로 맴돈다
    Circle,
    /// 뒤로 뺀다
    Back,
    /// 막는다 — **실제로 막는다**: 가드 중인 놈을 치면 피격으로 안 친다
    Guard,
    /// 주먹을 뻗는다. **닿을 때만 친다** — 헛스윙은 화면에서 아무 일도 아니다
    Punch,
    /// 맞고 휘청인다. **넉백은 없다** — 밀려나는 게 아니다
    Hurt,
    /// 눈이 X자가 되어 쓰러져 있다. 판이 끝날 때까지 안 일어난다
    Down,
    /// 최후의 1인 — 양 날개를 번쩍 든다
    Win,
    /// 벨트를 차고 세레모니
    Champ,
}

/// 발작 한 판이 거쳐 가는 국면.
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
    Idle {
        idle: IdleKind,
    },
    /// 공중을 헤엄쳐 목적지로 이동한다 — 위아래로 다니는 수단 (R11)
    Swim,
    Sleep,
    /// 클릭에 대한 반응 — 놀라지 않고 싸가지 없게 군다 (R5)
    Sassy {
        sassy: SassyKind,
    },
    /// 빽빽거리기 — 짧은 시간에 여러 번 맞으면 몸을 부풀리고 날개를 퍼덕이며
    /// 정면으로 화낸다. 싸가지 다섯이 전부 "무시하는" 결이라, 대비를 주는 것이
    /// 이 동작의 존재 이유다. **소리는 내지 않는다** (PRD §5.5).
    Squawk,
    /// 안물 — 묻지 않았다며 조잘거리며 춤춘다. 몸이 좌우로 흔들리고 머리가
    /// 반대로 기운다. **소리가 이 동작의 목적이다** — 유일하게 음원 파일을 쓴다
    /// (PRD §9 Q9의 예외, `MOTIONS.md` 효과음 절). 버튼으로만 시작한다.
    DontAsk,
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
    Freakout {
        freakout: FreakoutPhase,
    },
    /// 얼음낚시 — 바닥에 앉아 구멍을 뚫고 드리운다. 30~60초로 이 앱에서
    /// 가장 긴 동작이고, **안에서 갈래가 갈리는 첫 동작**이다 (잡음/꽝).
    IceFishing {
        fishing: FishingPhase,
    },
    /// 볼링 — 핀이 되어 화면 중앙에 삼각형으로 뜨고, 공이 지나가면 튕겨
    /// 나간다(그때는 `Thrown`이 된다). **여러 마리가 하나의 사건에 함께
    /// 참여하는 첫 동작**이라 국면을 혼자 진행하지 않는다: `Gather`만 스스로
    /// 끝나고 나머지는 판(`Pets::bowling`)이 몰아 준다.
    Bowling {
        bowling: BowlingPhase,
    },
    /// 비치발리볼 — 화면 세로 중앙에 뜬 모래톱으로 모여 한 판 친다.
    /// **사용자가 아무것도 안 하는 첫 동작**이라, 이 동작의 성패는 물리가 아니라
    /// "20초 동안 보고 있기 지루한가"에 달렸다. 랠리는 판(`Pets::volleyball`)이
    /// 시드 난수로 만들고 마리는 몰리는 쪽이다.
    Volleyball {
        volley: VolleyPhase,
    },
    /// 단체 야차 — 복싱 장갑을 끼고 링에 모여 **제자리에서** 치고받는다.
    /// **서로 튕겨나가지 않는 것이 이 동작의 정의다** (R7): 핀볼이 부딪힘을
    /// 속도로 바꾸는 자리에서 야차는 피격을 **자세와 피격 수**로만 바꾼다.
    /// 많이 맞은 마리부터 쓰러지고 최후의 1인이 벨트를 받는다.
    Yacha {
        yacha: YachaPhase,
    },
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

    /// **말풍선을 막는 동작인가.** 집결형 한 판(볼링·비치발리볼·야차)이 도는
    /// 동안에는 대사를 내지 않는다 — 여러 마리가 화면 가운데에 모이는데 그 위로
    /// 말풍선까지 뜨면 판이 안 보인다 (2026-09-03 사용자: *"채팅이 너무 화면을
    /// 가리거든"*).
    ///
    /// **판정을 여기 하나로 모은 이유**는 이 파일의 머리말이 경고하는 그 성질
    /// 때문이다 — 모션 하나가 일곱 자리에 흩어지므로, 모드마다 제 자리에서
    /// 막게 두면 **새 모드를 얹을 때 빠뜨리는 것이 기본값**이 된다. 대사를 내는
    /// 쪽은 이것 하나만 묻는다.
    ///
    /// **기본은 "모르면 낸다"다.** 클릭 통과(`pose_of`)가 "모르는 동작은 접는다"로
    /// 간 것과 반대인데, 성격이 다르기 때문이다: 거기서는 접는 쪽이 안전하지만
    /// (창이 클릭을 먹는 것은 원래 동작이다), **말풍선은 이 앱의 재미 그 자체**라
    /// 새 동작이 조용해지는 것은 손해다. 막는 것이 예외이므로 예외만 적는다.
    ///
    /// **핀볼은 여기 없다** — 사용자가 명시적으로 제외했고, 애초에 핀볼은
    /// `Behavior`가 아니라 마리의 플래그라 목록에 낄 수도 없다.
    pub fn silences_speech(self) -> bool {
        matches!(
            self,
            Behavior::Bowling { .. } | Behavior::Volleyball { .. } | Behavior::Yacha { .. }
        )
    }

    /// 스스로 고도를 만드는 동작인가 (진입하면 공중 상태가 된다).
    pub fn is_airborne(self) -> bool {
        matches!(
            self,
            Behavior::Swim
                | Behavior::Falling
                | Behavior::Thrown
                | Behavior::Freakout {
                    freakout: FreakoutPhase::Dash
                }
                // 판이 바닥이 아니라 **화면 세로 중앙**에 서므로 핀은 떠 있다.
                | Behavior::Bowling { .. }
                // **비치발리볼도 전부 공중이다** — 판이 볼링과 같이 화면 세로
                // 중앙에 서고 모래도 네트도 거기 함께 뜬다. 하나라도
                // 지상으로 두면 `clamp`가 그 국면에서 펭귄을 바닥으로 끌어내려
                // 코트에서 떨어진다.
                | Behavior::Volleyball { .. }
                // **단체 야차도 전부 공중이다** — 링이 볼링·발리볼과 같은 화면
                // 세로 중앙에 선다. 쓰러진 마리(`Down`)까지 포함해야 한다:
                // 하나라도 빼면 그 국면의 마리만 링에서 바닥으로 끌려 내려간다.
                | Behavior::Yacha { .. }
        )
    }
}
