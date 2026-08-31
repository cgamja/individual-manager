//! 펭귄 코어 — Tauri 무의존 순수 상태머신.
//!
//! 시간은 epoch ms로, 놀 수 있는 영역은 [`World`](화면마다 자기 [`Bounds`]를 갖는
//! [`Screen`]의 목록)로 주입받는다. 난수도 코어가
//! 소유한 시드 PRNG라 같은 시드 + 같은 타임스탬프열은 항상 같은 동작 시퀀스를 낳는다 —
//! 그래야 "스스로 움직이는" 동작을 테스트로 고정할 수 있다 (KTD1).

use serde::Serialize;

/// 펭귄 자체가 차지하는 한 변 (논리 px).
///
/// 브릿지가 아니라 **코어가 소유한다** — 어느 화면 위에 서 있는지를 정하는
/// 기준점이 이 값에서 나오기 때문이다. 남의 모듈에서 가져와 자기 판정에 쓰면
/// 그 모듈을 고칠 때 판정이 조용히 따라 바뀐다.
pub const PET_SIZE: f64 = 140.0;

/// 걷는 속도 (논리 px/초).
const WALK_SPEED: f64 = 42.0;
/// 헤엄치는 속도 (논리 px/초). 걷기보다 빨라야 "떠서 이동한다"는 느낌이 난다.
const SWIM_SPEED: f64 = 95.0;
/// 낙하 가속도 (논리 px/초²).
const GRAVITY: f64 = 900.0;
/// 벽·천장에 부딪혔을 때 남는 속도 비율. 1.0이면 영원히 튕긴다.
const BOUNCE_DAMPING: f64 = 0.5;
/// 던진 것으로 볼 최소 속도 (논리 px/초). 이보다 느리면 그냥 떨어뜨린 것이다.
/// **세계 크기와 무관하게 고정한다** — 이 문턱은 "사용자가 튕겼는가"라는 손의
/// 의도에 대한 것이고, 화면이 넓다고 같은 손짓이 떨어뜨림으로 바뀌면 이상하다 (KTD1).
const THROW_MIN_SPEED: f64 = 260.0;
/// 던지기 속도 상한 — 초당 **세계를 몇 번 가로지르는가**. 절대 px/s로 두면 좁은
/// 화면에서 눈 깜짝할 새 가로지르고 넓은 화면에서는 답답하다. 취향 상수라 여기서만 고친다.
///
/// 여기서 "세계"는 화면 폭이 아니라 **펭귄이 실제로 다닐 수 있는 가로 범위**다
/// ([`Bounds`]는 창 크기를 이미 빼 놓았다). 그래야 이 비율이 "1초에 세계를 0.9번
/// 가로지른다"는 뜻으로 문자 그대로 읽힌다.
const THROW_MAX_WORLDS_PER_SEC: f64 = 0.9;
/// 세계 폭을 못 구했을 때 쓸 기준 폭. 모니터 조회에 실패하면 브릿지가 폭 0인 납작한
/// 경계를 주는데, 그대로 비례식에 넣으면 상한이 0이 되어 **모든 던지기가 조용히
/// 낙하로 바뀐다**. 그런 죽음을 만들지 않으려고 둔다.
const FALLBACK_WORLD_WIDTH: f64 = 1_440.0;
/// 헤엄 목적지에 도착했다고 볼 거리.
const ARRIVE_EPSILON: f64 = 6.0;
/// 한 step이 정산하는 최대 시간. 시스템 슬립 등으로 틱이 밀렸을 때
/// 펭귄이 화면을 가로질러 순간이동하지 않게 잘라낸다.
const MAX_STEP_MS: u64 = 250;

const TURN_MS: u64 = 250;
/// 벽에 닿았을 때 **돌아서지 않고 그대로 박아 굴러 넘어질** 확률(%).
///
/// 벽 도달 자체가 흔하지 않아서(걷기가 2.5~6초 단발이고 속도는 `WALK_SPEED`)
/// 이보다 낮으면 평생 못 보고, 높이면 벽이 곧 넘어지는 곳이 되어 `Turn`이 사라진다.
/// 취향 상수라 여기서만 고친다.
const TUMBLE_AT_WALL_PERCENT: u64 = 30;
/// 굴러 넘어지는 데 걸리는 시간. 도는 것보다 확실히 길어야 갈래가 갈래로 읽힌다.
const TUMBLE_MS: u64 = 1_100;
/// 벽에 박은 반동으로 굴러가기 시작하는 속도 (논리 px/초). 걷기보다 빨라야
/// "튕겨서 굴렀다"로 보인다. 남은 시간에 비례해 0까지 줄어드므로 실제 이동
/// 거리는 이 값의 절반 × `TUMBLE_MS`, 대략 자기 몸 크기 남짓이다.
const TUMBLE_SPEED: f64 = 200.0;
/// 도는 것보다 짧으면 "박고 나자빠졌다"가 "잠깐 비틀했다"로 읽힌다.
const _: () = assert!(TUMBLE_MS > TURN_MS);
/// 싸가지 반응의 길이. 한 박자 확실히 보여야 킹받는다.
const SASSY_MS: u64 = 900;
/// 킹받는 한마디가 떠 있는 시간.
const SPEECH_MS: u64 = 3_200;
/// 한마디와 다음 한마디 사이 간격. 클릭과 무관하게 알아서 떠든다.
const TAUNT_GAP_MS: (u64, u64) = (7_000, 18_000);
/// 방망이를 휘두르는 시간.
const SWING_MS: u64 = 360;

/// 빽빽거리는 시간. 싸가지보다 **확실히 길어야** 한 박자 큰 반응으로 읽힌다 —
/// 뒤집히면 정면으로 화내는 쪽이 무시하는 쪽보다 짧아지는 꼴이라 컴파일 타임에 막는다.
const SQUAWK_MS: u64 = 1_400;
const _: () = assert!(SQUAWK_MS > SASSY_MS);
/// 이만큼 연달아 맞으면 빽빽거린다.
///
/// 취향 상수라 여기서만 고친다. 낮추면 한두 번 툭 치는 것으로 터져 "연타에 대한
/// 반응"이 아니게 되고, 높이면 평생 못 본다.
const SQUAWK_WHACK_COUNT: u64 = 4;
/// 연타로 인정하는 클릭 사이 최대 간격. 스윙 하나가 360ms라 자연스러운 연타
/// 간격은 200~400ms다 — 이 값은 넉넉한 쪽으로 잡았다.
const SQUAWK_GAP_MS: u64 = 800;

/// 착지 후 약이 올라 한마디 할 확률(%).
const SASSY_AFTER_LAND_PERCENT: u64 = 70;
const LAND_MS: u64 = 300;
/// 철푸덕은 착지보다 **길다** — 퍼진 상태를 한 박자 보여줘야 철푸덕이다.
const SPLAT_MS: u64 = 850;
/// 널브러짐은 철푸덕보다 **더** 길다. 배를 깔고 퍼진 것과 아예 뻗은 것은 다르다.
const SPRAWL_MS: u64 = 1_700;
/// 착지 강도가 커질수록 오래 누워 있어야 강도가 읽힌다. 뒤집히면 세게 박을수록
/// 빨리 일어나는 꼴이 되므로 컴파일 타임에 막는다.
const _: () = assert!(SPLAT_MS > LAND_MS);
const _: () = assert!(SPRAWL_MS > SPLAT_MS);

/// 이 속도(논리 px/초) 이상으로 바닥에 닿으면 철푸덕한다.
///
/// 중력이 900px/s²이므로 대략 **270px 넘게 떨어졌을 때**다. 제자리에서 놓거나
/// 한 뼘 떨어진 것으로 배를 깔면 착지가 매번 요란해진다.
const SPLAT_MIN_IMPACT: f64 = 700.0;
/// 이 속도 이상이면 아예 **널브러진다**. 대략 550px 넘게 떨어졌거나 아래로
/// 내리꽂았을 때다 — 화면 위쪽에서 떨어뜨리면 닿는 값이어야 의미가 있다.
const SPRAWL_MIN_IMPACT: f64 = 1_000.0;
const _: () = assert!(SPRAWL_MIN_IMPACT > SPLAT_MIN_IMPACT);

/// 이 속도보다 느리게 닿으면 튀지 않고 그대로 선다. 없으면 0에 수렴할 때까지
/// 영원히 잔진동한다.
const BOUNCE_MIN_SPEED: f64 = 150.0;
/// 바닥에서 튈 때 남는 속도 비율. 벽(`BOUNCE_DAMPING`)보다 많이 죽인다 —
/// 바닥은 몇 번 통통거리다 서야지, 오래 튀면 공처럼 보인다.
const FLOOR_BOUNCE_DAMPING: f64 = 0.45;
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

/// 미끄러지는 한 판의 길이. **고정이다** — 거리는 출발 속도로 흔든다.
///
/// 길이를 뽑으면 CSS 애니메이션 길이를 이 값과 맞출 수 없다
/// (`pet-css.test.ts`의 "동작 길이 동기화"가 대조하는 것이 고정 상수다).
const SLIDE_MS: u64 = 2_400;

/// 배를 깔고 출발하는 속도 범위 (논리 px/초). 여기서 뽑아 감속한다 —
/// 거리는 `속도 × 길이 / 2`라 매번 다르다 (264~408px).
///
/// **하한은 취향이 아니라 계산이다.** 가장 느리게 출발해도 가장 오래 걸은 것
/// (`WALK_SPEED × WALK_MS.1` = 252px)보다 멀리 가야 "걷기보다 멀리"가 조건 없이
/// 참이 된다. 180으로 뒀더니 216px이라 긴 걷기에 졌다.
const SLIDE_SPEED: (f64, f64) = (220.0, 340.0);

/// **걷기가** 끝났을 때 미끄러질 확률(%).
///
/// 유휴 뒤에는 나오지 않는다 — 서 있다가 갑자기 배를 깔면 준비 동작이 없다.
/// 걷던 관성이 있어야 미끄러지는 것으로 읽힌다.
const SLIDE_AFTER_WALK_PERCENT: u64 = 20;

// 회전이 다 돌기도 전에 미끄러짐이 끝나면 자세가 튄다
const _: () = assert!(SLIDE_MS > TURN_MS);

/// 걷기·유휴가 끝났을 때 얼음낚시를 시작할 확률 (천분율).
///
/// **백분율이 아니라 천분율인 이유**: 걷기·유휴 **한 사이클**이 평균 4초쯤이라
/// 십 분에 한 번은 대략 0.7%다 (아래 `FISHING_SESSION_MS`의 "한 판"과는 다른
/// 단위다 — 그쪽은 낚시 세션 전체다). `range((0, 99))`로는 최소가 1%라
/// 이 등급을 표현할 수 없다.
/// 자주 나오면 "가끔 보여서 반가운" 동작이 아니라 기본 동작이 된다.
const ICE_FISHING_PERMILLE: u64 = 7;

/// 구멍을 뚫는 시간.
const FISHING_DIG_MS: u64 = 1_400;
/// 드리우고 입질을 기다리는 시간. 이 앱에서 가장 긴 정적 구간이다.
const FISHING_WAIT_MS: (u64, u64) = (4_000, 9_000);
/// 찌가 까딱해서 홱 채는 시간.
const FISHING_BITE_MS: u64 = 700;
/// 잡은 물고기를 들어 자랑하는 시간.
const FISHING_CATCH_MS: u64 = 1_800;
/// 꽝을 보고 시무룩해 있는 시간.
const FISHING_MISS_MS: u64 = 1_300;
/// 예산이 다 돼서 낚싯대를 접고 일어나는 시간.
///
/// **이 국면이 없으면 앉은 자세에서 선 자세로 한 프레임 만에 튄다.** 유휴는
/// `.pg-all`을 건드리지 않으므로 눌림이 그 순간 사라진다. 모든 판이 여기로
/// 끝나므로 매번 보인다.
const FISHING_PACK_MS: u64 = 700;
/// 한 판의 예산. 이 시간이 지나면 다음 드리우기 대신 일어난다.
const FISHING_SESSION_MS: (u64, u64) = (30_000, 60_000);
/// 채서 물고기가 딸려 나올 확률. 나머지는 꽝이다.
const FISHING_CATCH_PERCENT: u64 = 40;

// 예산이 첫 드리우기보다 짧으면 구멍만 뚫고 끝나는 판이 생긴다
const _: () = assert!(FISHING_SESSION_MS.0 > FISHING_DIG_MS + FISHING_WAIT_MS.1);
const _: () = assert!(FISHING_SESSION_MS.1 >= FISHING_SESSION_MS.0);

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

/// 화면 식별자. 브릿지가 화면의 기하(위치·크기)에서 만든다 — macOS의
/// `Monitor::name()`은 모델 번호라 같은 모델 두 대를 구분하지 못하고, 고유값인
/// `CGDirectDisplayID`는 Tauri가 노출하지 않는다 (플랜 KTD2).
pub type ScreenId = u64;

/// 화면 하나. 배치가 바뀌면 `id`도 바뀌는데, 그건 버그가 아니라 "배치가 바뀌었다"는
/// 신호로 쓴다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Screen {
    pub id: ScreenId,
    pub bounds: Bounds,
}

impl Screen {
    /// 기준점(펭귄 발밑 중앙)이 이 화면 위에 있다고 볼 범위 — `(x0, x1, y0, y1)`.
    ///
    /// [`Bounds`]는 **펭귄 좌상단**의 범위라 기준점 기준으로는 그만큼 밀려 있다.
    /// 창 크기를 이미 빼 놓은 값이므로, 화면 둘이 실제로 맞닿아 있어도 이 범위
    /// 사이에는 **창 하나만큼 빈틈이 남는다.** 화면이 하나뿐인 지금은 드러나지
    /// 않지만, 경계를 넘나들게 만들 때는 그 빈틈을 메워야 한다.
    fn anchor_area(self) -> (f64, f64, f64, f64) {
        (
            self.bounds.left + PET_SIZE / 2.0,
            self.bounds.right + PET_SIZE / 2.0,
            self.bounds.top + PET_SIZE,
            self.bounds.floor_y + PET_SIZE,
        )
    }

    /// 기준점이 이 화면의 가로 범위에서 얼마나 벗어났나. 안이면 0이다.
    fn horizontal_distance(self, ax: f64) -> f64 {
        let (x0, x1, _, _) = self.anchor_area();
        (x0 - ax).max(ax - x1).max(0.0)
    }

    /// 기준점에서 이 화면까지의 거리. 화면 위에 있으면 0이다.
    ///
    /// 안일 때 **정확히** `0.0`이 나오는 것이 [`World::screen_at`]의 포함 판정을
    /// 떠받친다 — `dx`/`dy`가 각각 `max(0.0)`을 거치므로 부동소수 오차가 낄 자리가
    /// 없다. 여기에 클램프되지 않은 항을 더하면 그 성질이 깨진다.
    fn anchor_distance(self, ax: f64, ay: f64) -> f64 {
        let (_, _, y0, y1) = self.anchor_area();
        let dx = self.horizontal_distance(ax);
        let dy = (y0 - ay).max(ay - y1).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}

/// 펭귄이 노는 세계 — 연결된 화면 전부 (PRD §5.2).
///
/// **불변식: 비어 있지 않다.** 화면이 하나도 없는 세계에는 펭귄이 있을 자리가 없고,
/// 그 상태를 표현할 수 있게 두면 모든 판정에 `Option`이 번진다. 그래서 생성자에서
/// 한 번만 막는다.
#[derive(Clone, PartialEq, Debug)]
pub struct World {
    screens: Vec<Screen>,
}

impl World {
    /// 화면 목록에서 세계를 만든다. 비어 있으면 `None`.
    pub fn new(screens: Vec<Screen>) -> Option<Self> {
        if screens.is_empty() {
            None
        } else {
            Some(World { screens })
        }
    }

    /// 화면 하나짜리 세계. 모니터를 못 읽었을 때와 테스트에서 쓴다.
    pub fn single(bounds: Bounds) -> Self {
        World {
            screens: vec![Screen { id: 0, bounds }],
        }
    }

    /// 목록의 첫 화면. 비어 있지 않다는 불변식 덕에 항상 있다.
    pub fn first(&self) -> Screen {
        self.screens[0]
    }

    /// 기준점이 놓인 화면. 어느 화면에도 없으면 `None`.
    pub fn screen_at(&self, ax: f64, ay: f64) -> Option<Screen> {
        self.screens
            .iter()
            .copied()
            .find(|s| s.anchor_distance(ax, ay) == 0.0)
    }

    /// 기준점에서 가장 가까운 화면. 세계가 비어 있지 않으므로 항상 있다.
    pub fn nearest(&self, ax: f64, ay: f64) -> Screen {
        let mut best = self.screens[0];
        let mut best_d = best.anchor_distance(ax, ay);
        for s in &self.screens[1..] {
            let d = s.anchor_distance(ax, ay);
            if d < best_d {
                best = *s;
                best_d = d;
            }
        }
        best
    }

    /// 펭귄 x좌표가 놓일 화면. 새 펭귄을 어디에 만들지 정할 때 쓴다.
    /// 세로는 아직 정해지지 않았으므로 가로만 본다.
    pub fn screen_for_x(&self, x: f64) -> Screen {
        let ax = x + PET_SIZE / 2.0;
        let mut best = self.screens[0];
        let mut best_d = f64::INFINITY;
        for s in &self.screens {
            let d = s.horizontal_distance(ax);
            if d < best_d {
                best = *s;
                best_d = d;
            }
        }
        best
    }

    /// 세계 전체의 가로 폭 — 던지기 상한이 여기에 비례한다 (KTD7).
    /// 화면이 하나면 그 화면의 이동 폭과 같다.
    pub fn width(&self) -> f64 {
        let left = self
            .screens
            .iter()
            .map(|s| s.bounds.left)
            .fold(f64::INFINITY, f64::min);
        let right = self
            .screens
            .iter()
            .map(|s| s.bounds.right)
            .fold(f64::NEG_INFINITY, f64::max);
        (right - left).max(0.0)
    }
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
    /// 간격이 짧은 클릭이 지금까지 몇 번 이어졌는가. 문턱을 넘으면 빽빽거린다.
    ///
    /// 링버퍼에 최근 시각을 담아 "창 안에 N번"을 재는 방법도 있지만, 필드 둘이면
    /// 같은 체감을 내고 마릿수가 늘어도 메모리가 늘지 않는다.
    whack_run: u64,
    /// 마지막 빠따 시각. **`last_stimulus_ms`를 쓸 수 없다** — 그쪽은 드래그로도
    /// 갱신되므로 집었다 놓은 것이 연타로 세어진다.
    ///
    /// `Option`인 것이 중요하다. 0으로 초기화하면 에폭 초반 타임스탬프를 쓰는
    /// 테스트에서 `300 - 0 <= SQUAWK_GAP_MS`가 참이 되어 **첫 클릭이 이미 두
    /// 번째 연타로 세어진다.** 실제 앱에서는 안 드러나고 테스트에서만 터진다.
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
    /// 이번 슬라이딩의 출발 속도 (논리 px/초). 진입할 때 한 번 뽑는다 —
    /// 길이는 고정이고 이 값이 거리를 정한다.
    slide_speed: f64,
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
                    match landing_of(self.vy) {
                        Landing::Bounce(vy) => self.vy = vy,
                        Landing::Settle(behavior, hold) => {
                            self.vy = 0.0;
                            self.enter(behavior, now_ms + hold);
                        }
                    }
                }
            }
            Behavior::Walk => {
                self.x += self.facing.sign() * WALK_SPEED * dt;
                self.after_ground_move(now_ms, bounds);
            }
            Behavior::Turn => {
                if now_ms >= self.behavior_until_ms {
                    self.facing = self.facing.flipped();
                    let until = now_ms + self.range(WALK_MS);
                    self.enter(Behavior::Walk, until);
                }
            }
            Behavior::Slide => {
                // 감속은 남은 시간 비율로 한다. 마찰 상수를 두면 정지 판정이 따로
                // 필요해지고 그게 틀리면 영원히 미끄러지는데, 이 방식은 끝나는
                // 순간 속도가 정확히 0이라 그 상태를 표현할 수 없다 (굴러떨어지기와 같다).
                let remaining =
                    self.behavior_until_ms.saturating_sub(now_ms) as f64 / SLIDE_MS as f64;
                self.x += self.facing.sign() * self.slide_speed * remaining * dt;
                self.after_ground_move(now_ms, bounds);
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
            Behavior::Tumble => {
                // 남은 시간에 비례해 감속한다. 마찰 상수를 두면 정지 판정이 따로
                // 필요해지고 그게 틀리면 영원히 미끄러지는데, 이 방식은 동작이
                // 끝나는 순간 속도가 정확히 0이라 그 상태를 표현할 수 없다.
                let remaining =
                    self.behavior_until_ms.saturating_sub(now_ms) as f64 / TUMBLE_MS as f64;
                self.x += self.facing.sign() * TUMBLE_SPEED * remaining * dt;
                if now_ms >= self.behavior_until_ms {
                    self.get_up(now_ms);
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
                    match landing_of(self.vy) {
                        Landing::Bounce(vy) => {
                            self.vy = vy;
                            // 통통 튀며 앞으로도 조금 밀린다 — 제자리에서만 튀면
                            // 던진 방향이 착지에서 끊긴다
                            self.vx *= FLOOR_BOUNCE_DAMPING;
                        }
                        Landing::Settle(behavior, hold) => {
                            self.vx = 0.0;
                            self.vy = 0.0;
                            self.enter(behavior, now_ms + hold);
                        }
                    }
                }
            }
            Behavior::IceFishing { fishing } => {
                // 위치를 건드리지 않는다 — 앉은 자리에서 한다 (R5).
                // **국면 도중에 자르지 않는다**: 예산 확인은 드리우기로
                // 들어가는 순간에만 한다. 중간에 끊으면 낚싯대를 든 채
                // 사라지거나 채는 동작이 반쯤에서 잘린다.
                if now_ms >= self.behavior_until_ms {
                    match fishing {
                        FishingPhase::Dig => self.enter_fishing_wait(now_ms),
                        FishingPhase::Wait => self.enter_fishing(
                            FishingPhase::Bite,
                            now_ms + FISHING_BITE_MS,
                        ),
                        FishingPhase::Bite => {
                            let (phase, hold) =
                                if self.range((0, 99)) < FISHING_CATCH_PERCENT {
                                    (FishingPhase::Catch, FISHING_CATCH_MS)
                                } else {
                                    (FishingPhase::Miss, FISHING_MISS_MS)
                                };
                            self.enter_fishing(phase, now_ms + hold);
                        }
                        // **잡아도 판이 끝나지 않는다.** 잡을 때마다 끝내면 판
                        // 길이가 40% 확률에 좌우돼 중앙값이 20초 아래로 내려간다
                        // — 졸기(12~25초)보다 짧아져서 "가장 긴 동작"이라는
                        // 존재 이유가 사라진다. 예산 하나만 판 길이를 정한다.
                        FishingPhase::Miss | FishingPhase::Catch => {
                            self.enter_fishing_wait(now_ms)
                        }
                        FishingPhase::Pack => {
                            if self.air {
                                // 허공에서 낚시했으면 이제 마저 떨어진다
                                self.enter(Behavior::Falling, now_ms);
                            } else {
                                self.enter_idle(now_ms);
                            }
                        }
                    }
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
    /// 빠따 — 클릭 한 번에 펭귄이 방망이를 한 번 휘두른다. **맞는 쪽이 아니라
    /// 휘두르는 쪽이다.** 제자리에서 휘두르므로 날아가지 않는다 — 날려 보내는 건
    /// 드래그로 던졌을 때(`drag_end`)뿐이다.
    ///
    /// **짧은 간격으로 `SQUAWK_WHACK_COUNT`번 이어지면 빽빽거린다.** 문턱을 넘은
    /// 그 클릭에서 스윙을 건너뛰고 곧바로 터뜨린다 — 스윙 뒤로 미루면 연타
    /// 중에는 매 클릭이 스윙을 다시 걸기 때문에 **연타를 멈춘 뒤에야** 터져서
    /// 자기 손짓과 연결이 안 된다.
    pub fn whack(&mut self, now_ms: u64, _world: &World) {
        self.last_stimulus_ms = now_ms;
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

    /// 사용자가 시켜서 낚시를 시작한다 (설정 창의 "얼음낚시").
    ///
    /// **고도를 그대로 물려받는다.** 헤엄치는 중에 시키면 그 높이에 그대로 앉아
    /// 허공에서 낚시한다 — 얼음도 물도 없는 데서 낚싯대를 드리우는 게 이 앱의
    /// 결에 맞는다 (PRINCIPLE 1). 바닥으로 끌어내리면 헤엄치다 순간이동한다.
    ///
    /// **들려 있을 때만 거절한다.** 손에 쥔 채로 낚시를 시작하면 놓는 순간
    /// 낙하와 낚시가 겹친다. 시작했으면 참을 돌려주므로, 부르는 쪽이 "왜 아무
    /// 일도 없나"를 설명할 수 있다.
    ///
    /// 자극 시각을 갱신한다 — 시켜 놓고 5분 뒤에 조는 건 이상하다.
    pub fn start_fishing(&mut self, now_ms: u64) -> bool {
        // 이미 낚시 중이면 거절한다 — 이유는 `start_slide`와 같다
        if matches!(
            self.behavior,
            Behavior::Dragged | Behavior::IceFishing { .. }
        ) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_ice_fishing(now_ms);
        true
    }

    /// 사용자가 시켜서 미끄러진다 (설정 창의 "슬라이딩").
    ///
    /// **낚시와 달리 바닥에서만 먹는다.** 낚시는 허공에 앉는 게 더 웃겼지만
    /// 미끄러지는 것은 **바닥과 닿아야** 성립한다 — 공중에서 배를 깔면 그냥
    /// 헤엄이다. 들려 있을 때도 거절한다.
    ///
    /// 걸을 폭이 없는 화면에서는 미끄러질 자리도 없다 — 그 판정은 세계를 아는
    /// 쪽(`step`)이 하므로 여기서는 보지 않고, 진입해도 첫 틱에 정리된다.
    pub fn start_slide(&mut self, now_ms: u64) -> bool {
        // **이미 미끄러지는 중이면 거절한다.** 여기서 다시 진입하면 코어는 길이를
        // 늘리는데 웹뷰는 클래스가 그대로라 애니메이션을 되감지 않는다 — 누운
        // 그림이 끝나고도 펭귄이 선 채로 최대 2.4초를 더 미끄러진다.
        // `shouldRestart`가 "같은 한 번짜리 클래스가 연달아 오지 않는다"에 기대고
        // 있으므로, 그 전제를 깨지 않는 쪽을 고른다.
        if self.air || matches!(self.behavior, Behavior::Dragged | Behavior::Slide) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_slide(now_ms);
        true
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

    /// 지상 이동 한 틱을 마무리한다 — 경계에 닿았으면 벽 반응, 시간이 다 됐으면 다음 동작.
    ///
    /// **걷기와 슬라이딩이 이 함수를 공유한다.** `hit_wall`만 나눠 쓰고 그 앞의
    /// 분기 사슬을 복사해 두면, 경계 처리를 고칠 때(F2의 화면 넘기가 그렇다)
    /// 한쪽만 고쳐지고 조용히 갈라진다. 실제로 복사해 뒀다가 리뷰에서 잡혔다.
    fn after_ground_move(&mut self, now_ms: u64, bounds: Bounds) {
        if bounds.right <= bounds.left {
            // 걸어다닐 폭이 없는 화면(펭귄보다 좁은 작업 영역)에서는 양쪽 경계가
            // 겹쳐 매 step마다 Turn으로 들어가 영원히 제자리에서 돈다.
            // 그럴 때는 회전을 건너뛰고 평소처럼 유휴로 넘어가게 둔다.
            self.x = bounds.left;
            if now_ms >= self.behavior_until_ms {
                self.pick_next(now_ms, bounds);
            }
        } else if self.x <= bounds.left {
            self.x = bounds.left;
            self.hit_wall(now_ms);
        } else if self.x >= bounds.right {
            self.x = bounds.right;
            self.hit_wall(now_ms);
        } else if now_ms >= self.behavior_until_ms {
            self.pick_next(now_ms, bounds);
        }
    }

    /// 벽에 닿았을 때 — 얌전히 돌아서거나, 그대로 박고 굴러 넘어진다.
    ///
    /// 좌우 경계가 이 함수를 **공유한다.** "벽에 닿았다"를 판정하는 코드가 두 곳이
    /// 되면 한쪽만 고쳐지고 조용히 갈라진다.
    fn hit_wall(&mut self, now_ms: u64) {
        if self.range((0, 99)) < TUMBLE_AT_WALL_PERCENT {
            // 반동으로 벽 반대쪽으로 굴러간다. 방향을 **여기서** 뒤집는 이유는
            // 진행 방향과 `facing`이 어긋나면 웹뷰가 회전을 반대로 그리기
            // 때문이다. `Turn`은 끝날 때 뒤집지만 최종 결과는 같다.
            self.facing = self.facing.flipped();
            self.enter(Behavior::Tumble, now_ms + TUMBLE_MS);
        } else {
            self.enter(Behavior::Turn, now_ms + TURN_MS);
        }
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

    /// 미끄러지기 시작한다. **출발 속도를 여기서 한 번 뽑는다** — 길이는 고정이고
    /// 이 값이 거리를 정하므로, 매 틱 뽑으면 감속이 들쭉날쭉해진다.
    fn enter_slide(&mut self, now_ms: u64) {
        let (lo, hi) = SLIDE_SPEED;
        self.slide_speed = lo + self.fraction() * (hi - lo);
        self.enter(Behavior::Slide, now_ms + SLIDE_MS);
    }

    /// 얼음낚시 한 판을 시작한다. 구멍 뚫기부터다.
    ///
    /// 예산을 **여기서 한 번만** 뽑아 절대 시각으로 들고 있는다 — 국면이
    /// 몇 바퀴를 돌든 한 판의 길이는 이 값 하나가 정한다.
    fn enter_ice_fishing(&mut self, now_ms: u64) {
        self.fishing_until_ms = now_ms + self.range(FISHING_SESSION_MS);
        self.enter_fishing(FishingPhase::Dig, now_ms + FISHING_DIG_MS);
    }

    /// 드리우기로 들어가거나, 예산이 다 됐으면 일어난다.
    ///
    /// **구멍을 다 뚫었을 때·잡았을 때·꽝을 봤을 때가 이 함수를 공유한다.** 예산을
    /// 보는 코드가 두 벌이 되면 한쪽만 고쳐지고 조용히 갈라진다 (`hit_wall`과 같은 이유).
    /// 판이 끝나는 길도 여기 하나뿐이므로 **모든 판은 `Pack`을 거친다.**
    ///
    /// 나가는 길이 `get_up`이 아닌 것은 의도다 — 넘어졌다 일어난 뒤와 달리,
    /// 30초 얌전히 앉아 있다가 갑자기 약을 올리는 건 결이 다르다.
    fn enter_fishing_wait(&mut self, now_ms: u64) {
        if now_ms >= self.fishing_until_ms {
            self.enter_fishing(FishingPhase::Pack, now_ms + FISHING_PACK_MS);
            return;
        }
        let until = now_ms + self.range(FISHING_WAIT_MS);
        self.enter_fishing(FishingPhase::Wait, until);
    }

    fn enter_fishing(&mut self, fishing: FishingPhase, until_ms: u64) {
        self.enter(Behavior::IceFishing { fishing }, until_ms);
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

    const BOUNDS: Bounds = Bounds {
        left: 0.0,
        right: 1000.0,
        top: 0.0,
        floor_y: 800.0,
    };

    /// 화면 하나짜리 세계. 대부분의 테스트는 지금까지처럼 한 화면만 본다.
    fn world() -> World {
        World::single(BOUNDS)
    }

    fn pet() -> Pet {
        Pet::new(42, 0, &world())
    }

    /// `from`부터 `to`까지 `dt` 간격으로 진행시키며 스냅샷을 모은다.
    fn drive(pet: &mut Pet, from: u64, to: u64, dt: u64, world: &World) -> Vec<Snapshot> {
        let mut out = Vec::new();
        let mut t = from;
        while t <= to {
            out.push(pet.step(t, world));
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

        let after = p.step(1_000, &world());
        // 1초에 WALK_SPEED만큼 — MAX_STEP_MS(250ms)로 잘리므로 그 몫만 이동한다
        assert!(after.x > before.x, "오른쪽을 보면 x가 커져야 한다");
    }

    #[test]
    fn 왼쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
        let mut p = Pet::new(7, 0, &world());
        p.facing = Facing::Left;
        p.x = 5.0;

        let s = p.step(200, &world());
        assert_eq!(s.x, BOUNDS.left, "경계를 넘어가면 안 된다");
        assert_eq!(s.behavior, Behavior::Turn);
    }

    #[test]
    fn 오른쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
        let mut p = Pet::new(7, 0, &world());
        p.x = BOUNDS.right - 3.0;

        let s = p.step(200, &world());
        assert_eq!(s.x, BOUNDS.right);
        assert_eq!(s.behavior, Behavior::Turn);
    }

    #[test]
    fn 방향_전환이_끝나면_반대_방향으로_걷는다() {
        let mut p = Pet::new(7, 0, &world());
        p.x = BOUNDS.right - 3.0;
        p.step(200, &world());
        assert_eq!(p.snapshot().facing, Facing::Right);

        let s = p.step(200 + TURN_MS + 10, &world());
        assert_eq!(s.facing, Facing::Left, "전환이 끝나면 방향이 뒤집힌다");
        assert_eq!(s.behavior, Behavior::Walk);
    }

    /// 오른쪽 벽에 붙여 놓고 한 틱 진행시켜 벽 반응 하나를 본다.
    fn 벽_반응(seed: u64) -> Behavior {
        let mut p = Pet::new(seed, 0, &world());
        p.x = BOUNDS.right - 3.0;
        p.step(200, &world()).behavior
    }

    /// 벽에서 굴러떨어지는 시드 하나를 찾는다.
    ///
    /// 시드를 상수로 박아 두지 않는 이유는 **확률 갈래이기 때문**이다. 굴림은
    /// 코어가 소유한 PRNG를 쓰므로, 앞에서 난수를 한 번 더 뽑는 변경만 들어가도
    /// 박아 둔 시드가 반대 갈래로 넘어가 테스트가 통째로 무너진다.
    fn 굴러떨어지는_시드() -> u64 {
        (1u64..10_000)
            .find(|s| 벽_반응(*s) == Behavior::Tumble)
            .expect("굴러떨어지는 시드가 하나도 없다")
    }

    /// 오른쪽 벽에서 굴러떨어지기 시작한 펭귄. 여러 테스트가 같은 자세로 시작한다.
    fn 굴러떨어지는_펭귄() -> Pet {
        let mut p = Pet::new(굴러떨어지는_시드(), 0, &world());
        p.x = BOUNDS.right - 3.0;
        p.step(200, &world());
        p
    }

    #[test]
    fn 벽에_닿으면_굴러떨어지거나_돌아선다() {
        let 반응: Vec<Behavior> = (1u64..200).map(벽_반응).collect();
        assert!(
            반응.contains(&Behavior::Tumble),
            "굴러떨어지는 경우가 하나도 없다"
        );
        assert!(
            반응.contains(&Behavior::Turn),
            "돌아서는 경우가 하나도 없다 — 벽이 곧 넘어지는 곳이 됐다"
        );
        assert!(
            반응
                .iter()
                .all(|b| matches!(b, Behavior::Tumble | Behavior::Turn)),
            "벽 반응은 이 둘뿐이다"
        );
    }

    #[test]
    fn 굴러떨어지기는_벽_반대_방향으로_이동한다() {
        let mut p = 굴러떨어지는_펭귄();
        let s = p.snapshot();
        assert_eq!(s.behavior, Behavior::Tumble);
        assert_eq!(s.facing, Facing::Left, "벽에서 멀어지는 쪽을 본다");

        let after = p.step(300, &world()).x;
        assert!(after < s.x, "오른쪽 벽에서 굴렀으면 x가 작아진다");
    }

    #[test]
    fn 굴러떨어지는_동안_속도가_줄어든다() {
        let mut p = 굴러떨어지는_펭귄();
        let mut 이동량 = Vec::new();
        let mut prev = p.snapshot().x;
        let mut t = 250;
        while t < 200 + TUMBLE_MS {
            let x = p.step(t, &world()).x;
            이동량.push(prev - x);
            prev = x;
            t += 50;
        }
        assert!(
            이동량.first().unwrap() > 이동량.last().unwrap(),
            "뒤로 갈수록 덜 움직여야 구르다 멈추는 것으로 읽힌다: {이동량:?}"
        );
    }

    #[test]
    fn 굴러떨어지기가_끝나면_멈춘다() {
        let mut p = 굴러떨어지는_펭귄();
        drive(&mut p, 250, 200 + TUMBLE_MS, 50, &world());
        let 끝난_뒤 = p.step(200 + TUMBLE_MS + 10, &world()).x;
        let 더_뒤 = p.step(200 + TUMBLE_MS + 200, &world()).x;
        assert_eq!(끝난_뒤, 더_뒤, "굴러떨어지기가 끝나면 더 움직이지 않는다");
    }

    #[test]
    fn 굴러떨어지고_나면_방향이_뒤집혀_있다() {
        let mut p = Pet::new(굴러떨어지는_시드(), 0, &world());
        p.x = BOUNDS.right - 3.0;
        assert_eq!(p.snapshot().facing, Facing::Right);

        let s = p.step(200, &world());
        assert_eq!(s.facing, Facing::Left, "Turn을 탔을 때와 최종 결과가 같다");
    }

    #[test]
    fn 굴러떨어지기_뒤에는_약을_올리거나_유휴로_간다() {
        // 착지(Land/Splat/Sprawl)와 출구를 공유한다 — 같은 규칙이 두 벌이 되지 않게
        let mut 나온_동작 = Vec::new();
        for seed in 1u64..300 {
            let mut p = Pet::new(seed, 0, &world());
            p.x = BOUNDS.right - 3.0;
            if p.step(200, &world()).behavior != Behavior::Tumble {
                continue;
            }
            나온_동작.push(p.step(200 + TUMBLE_MS + 10, &world()).behavior);
        }
        assert!(!나온_동작.is_empty(), "굴러떨어지는 시드가 하나도 없다");
        assert!(
            나온_동작
                .iter()
                .all(|b| matches!(b, Behavior::Sassy { .. } | Behavior::Idle { .. })),
            "{나온_동작:?}"
        );
    }

    #[test]
    fn 굴러떨어지기는_지상_동작이다() {
        assert!(!Behavior::Tumble.is_airborne());
        assert!(
            !Behavior::Tumble.is_landing(),
            "바닥에 닿아서 생긴 게 아니다"
        );
        assert!(Behavior::Tumble.moves_window(), "제자리 애니메이션이 아니다");

        let s = 굴러떨어지는_펭귄().snapshot();
        assert!(!s.air);
        assert_eq!(s.y, BOUNDS.floor_y, "바닥에 붙어 굴러간다");
    }

    #[test]
    fn 굴러떨어지는_중에_클릭하면_방망이를_휘두른다() {
        let mut p = 굴러떨어지는_펭귄();
        p.whack(300, &world());
        assert_eq!(p.behavior(), Behavior::Swing);
    }

    #[test]
    fn 굴러떨어지는_중에_들어_올릴_수_있다() {
        let mut p = 굴러떨어지는_펭귄();
        p.drag_start(300);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 걸을_폭이_없는_화면에서는_굴러떨어지지_않는다() {
        // 양쪽 경계가 겹치는 화면에서는 벽 판정이 매 step 참이 된다. 여기서
        // 굴림을 돌리면 영원히 구르며 제자리를 맴돈다.
        let narrow = World::single(Bounds {
            left: 10.0,
            right: 10.0,
            top: 0.0,
            floor_y: 50.0,
        });
        for seed in 1u64..200 {
            let mut p = Pet::new(seed, 0, &narrow);
            let seen: Vec<Behavior> = drive(&mut p, 100, 5_000, 100, &narrow)
                .iter()
                .map(|s| s.behavior)
                .collect();
            assert!(!seen.contains(&Behavior::Tumble), "시드 {seed}");
        }
    }

    // ── 얼음낚시 ──────────────────────────────────────────────────
    //
    // 이 앱에서 가장 긴 동작이고, **안에서 갈래가 갈리는 첫 동작**이다
    // (잡음/꽝). 그래서 "무슨 국면을 거쳤는가"를 통째로 뽑아 놓고 규칙을 건다 —
    // 국면마다 펭귄을 따로 만들면 갈래가 늘 때마다 준비 코드가 갈라진다.

    /// 얼음낚시 한 판을 처음부터 끝까지 돌린다.
    ///
    /// 거쳐 간 국면(연속 중복은 접는다), 끝난 뒤의 동작, 끝난 시각을 돌려준다.
    fn 낚시_한_판(seed: u64) -> (Vec<FishingPhase>, Behavior, u64) {
        let w = world();
        let mut p = Pet::new(seed, 0, &w);
        p.enter_ice_fishing(0);
        let mut 국면 = Vec::new();
        let mut t = 0;
        loop {
            match p.step(t, &w).behavior {
                Behavior::IceFishing { fishing } => {
                    if 국면.last() != Some(&fishing) {
                        국면.push(fishing);
                    }
                }
                other => return (국면, other, t),
            }
            t += 50;
            assert!(t < 300_000, "시드 {seed}: 낚시가 끝나지 않는다");
        }
    }

    /// 30분치를 돌려 스냅샷을 모은다. 얼음낚시는 십 분에 한 번쯤이라
    /// 짧게 돌리면 한 번도 안 나온다.
    fn 삼십분(seed: u64) -> Vec<Snapshot> {
        let w = world();
        let mut p = Pet::new(seed, 0, &w);
        drive(&mut p, 100, 30 * 60_000, 100, &w)
    }

    #[test]
    fn 가끔_얼음낚시를_한다() {
        let 나온_시드: Vec<u64> = (1u64..7)
            .filter(|s| {
                삼십분(*s)
                    .iter()
                    .any(|s| matches!(s.behavior, Behavior::IceFishing { .. }))
            })
            .collect();
        assert!(
            !나온_시드.is_empty(),
            "30분을 돌려도 얼음낚시가 한 번도 안 나온다"
        );
    }

    #[test]
    fn 얼음낚시는_드물다() {
        // 자주 나오면 "가끔 보여서 반가운" 동작이 아니라 기본 동작이 된다
        for seed in 1u64..7 {
            let 전체 = 삼십분(seed);
            let 낚시 = 전체
                .iter()
                .filter(|s| matches!(s.behavior, Behavior::IceFishing { .. }))
                .count();
            assert!(
                낚시 * 100 < 전체.len() * 30,
                "시드 {seed}: 30분 중 {낚시}/{} 이 낚시다",
                전체.len()
            );
        }
    }

    #[test]
    fn 얼음낚시는_구멍_뚫기부터_시작한다() {
        let (국면, _, _) = 낚시_한_판(42);
        assert_eq!(국면.first(), Some(&FishingPhase::Dig));
    }

    #[test]
    fn 구멍을_뚫고_나면_드리운다() {
        let (국면, _, _) = 낚시_한_판(42);
        assert_eq!(국면.get(1), Some(&FishingPhase::Wait), "{국면:?}");
    }

    #[test]
    fn 입질_뒤에는_잡거나_꽝이다() {
        for seed in 1u64..40 {
            let (국면, _, _) = 낚시_한_판(seed);
            for (i, phase) in 국면.iter().enumerate() {
                if *phase != FishingPhase::Bite {
                    continue;
                }
                let 다음 = 국면.get(i + 1);
                assert!(
                    matches!(
                        다음,
                        Some(FishingPhase::Catch) | Some(FishingPhase::Miss)
                    ),
                    "시드 {seed}: 입질 뒤가 {다음:?}다 — {국면:?}"
                );
            }
        }
    }

    #[test]
    fn 꽝이면_다시_드리운다() {
        let mut 봤다 = false;
        for seed in 1u64..40 {
            let (국면, _, _) = 낚시_한_판(seed);
            for (i, phase) in 국면.iter().enumerate() {
                if *phase != FishingPhase::Miss {
                    continue;
                }
                let 다음 = 국면.get(i + 1);
                if 다음 == Some(&FishingPhase::Wait) {
                    봤다 = true;
                }
                // 예산이 다 됐으면 다시 드리우지 않고 접는다
                assert!(
                    matches!(다음, Some(FishingPhase::Wait) | Some(FishingPhase::Pack)),
                    "시드 {seed}: 꽝 뒤가 {다음:?}다 — {국면:?}"
                );
            }
        }
        assert!(봤다, "꽝 뒤에 다시 드리우는 판이 하나도 없다");
    }

    #[test]
    fn 물고기를_잡아도_예산이_남으면_다시_드리운다() {
        // 잡을 때마다 판을 끝내면 길이가 40% 확률에 좌우돼 중앙값이 20초 아래로
        // 내려간다 — 졸기보다 짧아지면 "가장 긴 동작"이 아니다
        let mut 다시_드리운_적 = false;
        for seed in 1u64..40 {
            let (국면, _, _) = 낚시_한_판(seed);
            for (i, phase) in 국면.iter().enumerate() {
                if *phase != FishingPhase::Catch {
                    continue;
                }
                let 다음 = 국면.get(i + 1);
                if 다음 == Some(&FishingPhase::Wait) {
                    다시_드리운_적 = true;
                }
                assert!(
                    matches!(다음, Some(FishingPhase::Wait) | Some(FishingPhase::Pack)),
                    "시드 {seed}: 잡은 뒤가 {다음:?}다 — {국면:?}"
                );
            }
        }
        assert!(다시_드리운_적, "잡고 나서 다시 드리우는 판이 하나도 없다");
    }

    #[test]
    fn 모든_판은_낚싯대를_접고_끝난다() {
        // 앉은 자세에서 곧장 유휴로 가면 눌림이 한 프레임 만에 사라져 튄다
        for seed in 1u64..40 {
            let (국면, _, _) = 낚시_한_판(seed);
            assert_eq!(
                국면.last(),
                Some(&FishingPhase::Pack),
                "시드 {seed}: {국면:?}"
            );
        }
    }

    #[test]
    fn 얼음낚시_한_판은_예산_안에_끝난다() {
        // 국면 도중에 자르지 않으므로 상한은 예산 + 마지막 한 바퀴다.
        // 무한히 도는 판을 잡는 게 이 테스트의 목적이다.
        // 판을 끝내는 것은 **예산 하나뿐**이므로 하한이 예산의 하한이다.
        // 잡았다고 끝나던 때는 이 단언이 성립하지 않았고, 실제 중앙값이
        // 18.6초까지 내려가 있었다 (리뷰 실측).
        let 상한 = FISHING_SESSION_MS.1
            + FISHING_WAIT_MS.1
            + FISHING_BITE_MS
            + FISHING_MISS_MS.max(FISHING_CATCH_MS)
            + FISHING_PACK_MS;
        for seed in 1u64..40 {
            let (국면, _, 끝) = 낚시_한_판(seed);
            assert!(
                끝 >= FISHING_SESSION_MS.0,
                "시드 {seed}: {끝}ms 만에 끝났다 — 예산보다 짧다 — {국면:?}"
            );
            assert!(끝 <= 상한, "시드 {seed}: {끝}ms 나 걸렸다 — {국면:?}");
        }
    }

    #[test]
    fn 얼음낚시_중에는_위치가_변하지_않는다() {
        let w = world();
        let mut p = Pet::new(42, 0, &w);
        p.x = 400.0;
        p.enter_ice_fishing(0);
        let (시작_x, 시작_y) = (p.x, p.y);
        let mut t = 0;
        while let Behavior::IceFishing { .. } = p.step(t, &w).behavior {
            assert_eq!((p.x, p.y), (시작_x, 시작_y), "{t}ms 에서 움직였다");
            t += 50;
        }
    }

    #[test]
    fn 얼음낚시가_끝나면_유휴로_간다() {
        // 넘어졌다 일어난 뒤(get_up)와 출구를 공유하지 않는다 — 30초 얌전히
        // 앉아 있다 갑자기 약을 올리는 건 결이 다르다
        for seed in 1u64..40 {
            let (국면, 뒤, _) = 낚시_한_판(seed);
            assert!(
                matches!(뒤, Behavior::Idle { .. }),
                "시드 {seed}: 낚시 뒤가 {뒤:?}다 — {국면:?}"
            );
        }
    }

    #[test]
    fn 얼음낚시_중에_클릭하면_방망이를_휘두른다() {
        let mut p = pet();
        p.enter_ice_fishing(0);
        p.whack(300, &world());
        assert_eq!(p.behavior(), Behavior::Swing);
    }

    #[test]
    fn 얼음낚시_중에_들어_올릴_수_있다() {
        let mut p = pet();
        p.enter_ice_fishing(0);
        p.drag_start(300);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 얼음낚시는_지상_동작이다() {
        let 낚시 = Behavior::IceFishing {
            fishing: FishingPhase::Wait,
        };
        assert!(!낚시.is_airborne());
        assert!(!낚시.is_landing(), "바닥에 닿아서 생긴 게 아니다");
        // 창은 제자리지만 틱은 빠르게 유지해야 한다 — 느려지면 0.7초짜리
        // 입질 국면이 최대 0.5초 늦게 도착한다
        assert!(낚시.moves_window(), "틱이 느려지면 국면이 늦게 도착한다");

        let mut p = pet();
        p.enter_ice_fishing(0);
        let s = p.step(50, &world());
        assert!(!s.air);
        assert_eq!(s.y, BOUNDS.floor_y, "바닥에 앉는다");
    }

    #[test]
    fn 시키면_바로_낚시를_시작한다() {
        let mut p = pet();
        assert!(p.start_fishing(1_000));
        assert_eq!(
            p.behavior(),
            Behavior::IceFishing {
                fishing: FishingPhase::Dig
            }
        );
    }

    #[test]
    fn 시키면_바로_미끄러진다() {
        let mut p = pet();
        p.x = 400.0;
        assert!(p.start_slide(1_000));
        assert_eq!(p.behavior(), Behavior::Slide);
        let 뒤 = p.step(1_200, &world());
        assert_ne!(뒤.x, 400.0, "시켰는데 제자리다");
    }

    #[test]
    fn 공중이거나_들려_있으면_시켜도_미끄러지지_않는다() {
        // 미끄러지는 것은 바닥과 닿아야 성립한다 — 공중에서 배를 깔면 그냥 헤엄이다
        let mut 헤엄 = pet();
        헤엄.air = true;
        assert!(!헤엄.start_slide(1_000));
        assert_ne!(헤엄.behavior(), Behavior::Slide);

        let mut 들림 = pet();
        들림.drag_start(1_000);
        assert!(!들림.start_slide(1_100));
        assert_eq!(들림.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 이미_하는_중이면_다시_시켜도_받지_않는다() {
        // 다시 진입하면 코어는 길이를 늘리는데 웹뷰는 클래스가 그대로라
        // 애니메이션을 되감지 않는다 — 그림과 상태가 어긋난다
        let mut 낚시 = pet();
        assert!(낚시.start_fishing(1_000));
        assert!(!낚시.start_fishing(1_200), "낚시 중에 또 받았다");

        let mut 슬라이딩 = pet();
        슬라이딩.x = 400.0;
        assert!(슬라이딩.start_slide(1_000));
        assert!(!슬라이딩.start_slide(1_200), "미끄러지는 중에 또 받았다");
    }

    #[test]
    fn 들려_있으면_시켜도_낚시하지_않는다() {
        // 손에 쥔 채로 시작하면 놓는 순간 낙하와 낚시가 겹친다
        let mut 들림 = pet();
        들림.drag_start(1_000);
        assert!(!들림.start_fishing(1_100));
        assert_eq!(들림.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 공중에서_시키면_허공에서_낚시한다() {
        // 바닥으로 끌어내리면 헤엄치다 순간이동한다
        let w = world();
        let mut p = pet();
        p.air = true;
        p.y = 300.0;
        assert!(p.start_fishing(1_000));

        let s = p.step(1_050, &w);
        assert!(matches!(s.behavior, Behavior::IceFishing { .. }));
        assert!(s.air, "고도를 잃으면 안 된다");
        assert_eq!(s.y, 300.0, "그 높이에 그대로 앉는다");
    }

    #[test]
    fn 허공에서_낚시가_끝나면_떨어진다() {
        // 유휴로 바로 가면 clamp가 바닥으로 순간이동시킨다
        let w = world();
        let mut p = pet();
        p.air = true;
        p.y = 300.0;
        assert!(p.start_fishing(0));
        let 동작: Vec<Behavior> = drive(&mut p, 0, 120_000, 50, &w)
            .iter()
            .map(|s| s.behavior)
            .collect();
        let 낚시_뒤 = 동작
            .iter()
            .skip_while(|b| matches!(b, Behavior::IceFishing { .. }))
            .next()
            .copied();
        assert_eq!(낚시_뒤, Some(Behavior::Falling), "{:?}", &동작[..5]);
    }

    #[test]
    fn 시켜서_시작한_낚시_중에는_졸지_않는다() {
        // 자극 시각을 갱신하지 않으면 시켜 놓고 조는 판이 생긴다
        let w = world();
        let mut p = Pet::new(42, 0, &w);
        let 시작 = SLEEP_AFTER_MS + 10_000;
        assert!(p.start_fishing(시작));
        let 동작: Vec<Behavior> = drive(&mut p, 시작, 시작 + 90_000, 100, &w)
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert!(!동작.contains(&Behavior::Sleep), "낚시하다 졸았다");
    }

    #[test]
    fn 공중에서는_얼음낚시를_시작하지_않는다() {
        // 앉을 자리가 없다. 지금은 pick_next가 지상에서만 불리지만,
        // 그 전제가 깨져도 낚시가 공중에서 시작되면 안 된다
        for seed in 1u64..200 {
            let mut p = Pet::new(seed, 0, &world());
            p.air = true;
            for i in 0..40u64 {
                p.pick_next(i * 100, BOUNDS);
                assert!(
                    !matches!(p.behavior, Behavior::IceFishing { .. }),
                    "시드 {seed}: 공중에서 낚시를 시작했다"
                );
                p.air = true;
            }
        }
    }

    // ── 슬라이딩 ──────────────────────────────────────────────────

    /// 걷기가 끝나는 순간의 갈래 하나를 본다. 걷기 시간이 다 되도록 몰아 놓고
    /// 한 틱 더 진행시킨다.
    fn 걷기_뒤(seed: u64) -> Behavior {
        let w = world();
        let mut p = Pet::new(seed, 0, &w);
        // 벽에 닿아 hit_wall로 새지 않게 가운데에서 출발시킨다
        p.x = 400.0;
        let mut t = 50;
        while t < 20_000 {
            let b = p.step(t, &w).behavior;
            if b != Behavior::Walk {
                return b;
            }
            t += 50;
        }
        panic!("시드 {seed}: 걷기가 끝나지 않는다");
    }

    /// 미끄러지기 시작한 펭귄과 시작 시각.
    fn 미끄러지는_펭귄() -> (Pet, u64) {
        let w = world();
        for seed in 1u64..500 {
            let mut p = Pet::new(seed, 0, &w);
            p.x = 400.0;
            let mut t = 50;
            while t < 20_000 {
                if p.step(t, &w).behavior == Behavior::Slide {
                    return (p, t);
                }
                t += 50;
            }
        }
        panic!("미끄러지는 시드가 하나도 없다");
    }

    #[test]
    fn 걷기가_끝나면_가끔_미끄러진다() {
        let 갈래: Vec<Behavior> = (1u64..120).map(걷기_뒤).collect();
        assert!(
            갈래.contains(&Behavior::Slide),
            "미끄러지는 경우가 하나도 없다"
        );
        assert!(
            갈래.iter().any(|b| matches!(b, Behavior::Idle { .. })),
            "쉬는 경우가 사라졌다 — 걷고 나면 늘 미끄러진다"
        );
    }

    #[test]
    fn 유휴가_끝났을_때는_미끄러지지_않는다() {
        // 서 있다가 갑자기 배를 깔면 준비 동작이 없다. 걷던 관성이 있어야
        // 미끄러지는 것으로 읽힌다
        for seed in 1u64..300 {
            let mut p = Pet::new(seed, 0, &world());
            p.behavior = Behavior::Idle { idle: IdleKind::Shake };
            p.pick_next(1_000, BOUNDS);
            assert_ne!(p.behavior, Behavior::Slide, "시드 {seed}");
        }
    }

    #[test]
    fn 미끄러지는_동안_진행_방향으로_이동한다() {
        let (mut p, t) = 미끄러지는_펭귄();
        let 시작 = p.snapshot();
        let 뒤 = p.step(t + 200, &world());
        let 나아간_거리 = (뒤.x - 시작.x) * 시작.facing.sign();
        assert!(나아간_거리 > 0.0, "{나아간_거리}");
    }

    #[test]
    fn 슬라이딩은_걷기보다_빠르다() {
        let (mut p, t) = 미끄러지는_펭귄();
        let 시작_x = p.snapshot().x;
        let facing = p.snapshot().facing;
        let 뒤 = p.step(t + 200, &world());
        let 미끄러진_거리 = (뒤.x - 시작_x).abs();
        let 걸었을_거리 = WALK_SPEED * 0.2;
        assert!(
            미끄러진_거리 > 걸었을_거리,
            "{미끄러진_거리} vs 걷기 {걸었을_거리}"
        );
        let _ = facing;
    }

    #[test]
    fn 미끄러지는_동안_속도가_줄어든다() {
        let (mut p, t0) = 미끄러지는_펭귄();
        let mut 이동량 = Vec::new();
        let mut prev = p.snapshot().x;
        let sign = p.snapshot().facing.sign();
        let mut t = t0 + 50;
        while t < t0 + SLIDE_MS {
            let x = p.step(t, &world()).x;
            이동량.push((x - prev) * sign);
            prev = x;
            t += 50;
        }
        assert!(
            이동량.first().unwrap() > 이동량.last().unwrap(),
            "뒤로 갈수록 덜 움직여야 주르륵 멈추는 것으로 읽힌다: {이동량:?}"
        );
    }

    #[test]
    fn 슬라이딩이_끝나면_멈춘다() {
        let (mut p, t0) = 미끄러지는_펭귄();
        drive(&mut p, t0 + 50, t0 + SLIDE_MS, 50, &world());
        let 끝난_뒤 = p.step(t0 + SLIDE_MS + 10, &world()).x;
        let 더_뒤 = p.step(t0 + SLIDE_MS + 60, &world()).x;
        assert!(
            (끝난_뒤 - 더_뒤).abs() < 0.001 || p.behavior() != Behavior::Slide,
            "끝났는데도 미끄러진다"
        );
    }

    #[test]
    fn 미끄러진_거리는_매번_다르다() {
        // 길이는 고정이고 출발 속도를 뽑는다 — 길이를 뽑으면 CSS와 맞출 수 없다
        let w = world();
        let mut 거리 = Vec::new();
        for seed in 1u64..400 {
            let mut p = Pet::new(seed, 0, &w);
            p.x = 400.0;
            let mut t = 50;
            while t < 20_000 {
                if p.step(t, &w).behavior == Behavior::Slide {
                    let 시작 = p.snapshot().x;
                    let sign = p.snapshot().facing.sign();
                    drive(&mut p, t + 50, t + SLIDE_MS, 50, &w);
                    거리.push(((p.snapshot().x - 시작) * sign * 10.0).round() as i64);
                    break;
                }
                t += 50;
            }
            if 거리.len() >= 8 {
                break;
            }
        }
        assert!(거리.len() >= 3, "표본이 모자라다: {거리:?}");
        assert!(
            거리.iter().collect::<std::collections::HashSet<_>>().len() > 1,
            "거리가 늘 같다: {거리:?}"
        );
    }

    #[test]
    fn 슬라이딩이_걷기보다_멀리_간다() {
        // 가장 느리게 출발해도 가장 오래 걷는 것보다 멀리 간다
        let 최소_거리 = SLIDE_SPEED.0 * (SLIDE_MS as f64 / 1000.0) / 2.0;
        let 걷기_최대 = WALK_SPEED * (WALK_MS.1 as f64 / 1000.0);
        assert!(최소_거리 > 걷기_최대, "{최소_거리} vs {걷기_최대}");
    }

    #[test]
    fn 미끄러지다_벽에_닿으면_돌아서거나_굴러떨어진다() {
        // 벽 판정이 걷기와 두 벌이 되면 한쪽만 고쳐지고 조용히 갈라진다
        let w = world();
        let (mut p, _) = 미끄러지는_펭귄();
        // 진행 방향 벽 바로 앞에 갖다 놓는다
        let 벽 = if p.snapshot().facing == Facing::Right {
            BOUNDS.right
        } else {
            BOUNDS.left
        };
        p.x = 벽 - p.snapshot().facing.sign() * 2.0;
        let s = p.step(p.last_step_ms + 100, &w);
        assert!(
            matches!(s.behavior, Behavior::Turn | Behavior::Tumble),
            "{:?}",
            s.behavior
        );
        assert!(s.x >= BOUNDS.left && s.x <= BOUNDS.right, "경계를 넘었다");
    }

    #[test]
    fn 걸을_폭이_없는_화면에서는_미끄러지지_않는다() {
        let narrow = World::single(Bounds {
            left: 10.0,
            right: 10.0,
            top: 0.0,
            floor_y: 50.0,
        });
        for seed in 1u64..200 {
            let mut p = Pet::new(seed, 0, &narrow);
            let seen: Vec<Behavior> = drive(&mut p, 100, 20_000, 100, &narrow)
                .iter()
                .map(|s| s.behavior)
                .collect();
            assert!(!seen.contains(&Behavior::Slide), "시드 {seed}");
        }
    }

    #[test]
    fn 슬라이딩은_지상_동작이다() {
        assert!(!Behavior::Slide.is_airborne());
        assert!(!Behavior::Slide.is_landing());
        assert!(Behavior::Slide.moves_window(), "창이 따라 움직여야 한다");
        let (p, _) = 미끄러지는_펭귄();
        assert!(!p.snapshot().air);
        assert_eq!(p.snapshot().y, BOUNDS.floor_y);
    }

    #[test]
    fn 미끄러지는_중에_클릭하면_방망이를_휘두른다() {
        let (mut p, t) = 미끄러지는_펭귄();
        p.whack(t + 100, &world());
        assert_eq!(p.behavior(), Behavior::Swing);
    }

    #[test]
    fn 미끄러지는_중에_들어_올릴_수_있다() {
        let (mut p, t) = 미끄러지는_펭귄();
        p.drag_start(t + 100);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

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
        p.whack(1_000, &world());
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
        p.whack(1_000, &world());
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
            p.whack(1_000 + i * 100, &world());
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

        p.whack(t, &world());
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
        p.whack(t, &world());
        assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
    }

    #[test]
    fn 휘두른다고_말하지는_않는다() {
        // 말은 클릭이 아니라 시간에 맞춰 나온다 — 때릴 때마다 떠들면 시끄럽다
        let mut p = pet();
        p.whack(1_000, &world());
        assert!(p.snapshot().speech.is_none(), "클릭으로 말이 나오면 안 된다");
        p.whack(1_100, &world());
        p.whack(1_200, &world());
        assert!(p.snapshot().speech.is_none(), "연타해도 마찬가지다");
    }

    // ── 빽빽거리기 ─────────────────────────────────────────────────

    /// **실제 클릭 한 번**을 흉내 낸다. 프론트는 클릭인지 드래그인지 알기 전에
    /// 모든 pointerdown에서 `drag_start`를 부르므로, `whack`만 부르면 실제로는
    /// 지나지 않는 경로를 테스트하게 된다.
    fn 클릭(p: &mut Pet, now_ms: u64) {
        p.drag_start(now_ms);
        p.whack(now_ms, &world());
    }

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
    fn 짧은_간격으로_네_번_맞으면_빽빽거린다() {
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
    fn 첫_클릭이_연타로_세어지지_않는다() {
        // `last_whack_ms`를 0으로 초기화하면 에폭 초반 타임스탬프에서
        // `300 - 0 <= GAP`이 참이 되어 첫 클릭이 이미 두 번째로 세어진다
        let mut p = pet();
        클릭(&mut p, 300);
        클릭(&mut p, 400);
        클릭(&mut p, 500);
        assert_eq!(p.behavior(), Behavior::Swing, "아직 세 번이다");
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
            p.whack(t, &world());
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

    /// 왼쪽 화면과, 그 오른쪽에 떨어져 놓인 화면. 사이에 빈 공간이 있다.
    fn 두_화면() -> World {
        World::new(vec![
            Screen {
                id: 1,
                bounds: Bounds { left: 0.0, right: 1_000.0, top: 0.0, floor_y: 800.0 },
            },
            Screen {
                id: 2,
                bounds: Bounds { left: 2_000.0, right: 3_000.0, top: 100.0, floor_y: 900.0 },
            },
        ])
        .expect("화면이 둘이면 세계가 만들어진다")
    }

    #[test]
    fn 빈_화면_목록으로는_세계를_만들_수_없다() {
        assert!(World::new(vec![]).is_none(), "펭귄이 있을 자리가 없다");
    }

    #[test]
    fn 기준점은_펭귄_발밑_중앙이다() {
        let mut p = pet();
        p.x = 300.0;
        p.y = 400.0;
        assert_eq!(p.anchor(), (300.0 + PET_SIZE / 2.0, 400.0 + PET_SIZE));
    }

    #[test]
    fn 발밑이_속한_화면을_찾는다() {
        let w = 두_화면();
        // 왼쪽 화면 한복판에 선 펭귄
        let left = (500.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.screen_at(left.0, left.1).map(|s| s.id), Some(1));
        // 오른쪽 화면 한복판에 선 펭귄
        let right = (2_500.0 + PET_SIZE / 2.0, 900.0 + PET_SIZE);
        assert_eq!(w.screen_at(right.0, right.1).map(|s| s.id), Some(2));
    }

    #[test]
    fn 화면_사이_빈_공간에는_화면이_없다() {
        let w = 두_화면();
        let gap = (1_500.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert!(w.screen_at(gap.0, gap.1).is_none());
    }

    #[test]
    fn 발밑이_어느_화면에도_없으면_가장_가까운_화면을_준다() {
        let w = 두_화면();
        // 왼쪽 화면 바로 오른쪽 — 1번이 가깝다
        let near_left = (1_100.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.nearest(near_left.0, near_left.1).id, 1);
        // 오른쪽 화면 바로 왼쪽 — 2번이 가깝다
        let near_right = (1_900.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.nearest(near_right.0, near_right.1).id, 2);
    }

    #[test]
    fn 세계_폭은_화면_전체를_덮는다() {
        assert_eq!(두_화면().width(), 3_000.0);
        assert_eq!(
            World::single(BOUNDS).width(),
            BOUNDS.right - BOUNDS.left,
            "화면이 하나면 그 화면의 이동 폭과 같다"
        );
    }

    #[test]
    fn 화면이_하나면_동작_수열이_그대로다() {
        // 좌표계가 화면 목록(`World`)으로 바뀌어도 화면이 하나면 판정이 달라지지
        // 않는다는 증거다.
        //
        // "경계 안에 있는가"만 보면 안 된다 — 판정 사각형이 좁아지거나 밀려도,
        // 심지어 기준점 계산이 망가져도 통과한다. 화면이 하나뿐이면 어떤 기준점이든
        // 결국 같은 화면으로 떨어지기 때문이다. 그래서 **수열을 통째로 못박는다.**
        //
        // 값은 **확률 갈래가 하나 늘 때마다** 다시 뜬다. 갈래는 난수를 하나 더
        // 뽑고, 그러면 그 뒤가 통째로 밀린다. 지금까지 세 번 재기준화했다 —
        // 벽 굴림(`hit_wall`), 얼음낚시, 슬라이딩(둘 다 `pick_next`). 전부 의도한 변경이다.
        // **동작을 늘리지 않았는데 이 배열이 흔들리면 그건 의도하지 않은 변경이다.**
        let w = world();
        let mut p = Pet::new(42, 0, &w);
        let seq = drive(&mut p, 0, 60_000, 50, &w);
        assert_eq!(seq.len(), 1_201);

        // (인덱스, 동작, x, y)
        let golden = [
            (0_usize, "Turn", 0.0, 800.0),
            (97, "Swim", 206.4, 701.8),
            (194, "Swim", 502.8, 349.1),
            (291, "Sprawl", 643.4, 800.0),
            (388, "Walk", 716.9, 800.0),
            (485, "Walk", 960.1, 800.0),
            (582, "Idle { idle: LookAround }", 848.8, 800.0),
            (679, "Walk", 670.3, 800.0),
            (776, "Swim", 296.9, 567.4),
            (873, "Sassy { sassy: HeadFlick }", 121.6, 800.0),
            (970, "Idle { idle: Stretch }", 121.6, 800.0),
            (1067, "Walk", 77.5, 800.0),
            (1164, "Walk", 115.5, 800.0),
        ];
        for (i, behavior, x, y) in golden {
            let s = seq[i];
            assert_eq!(format!("{:?}", s.behavior), behavior, "{i}번째 동작");
            assert_eq!(format!("{:.1}", s.x), format!("{x:.1}"), "{i}번째 x");
            assert_eq!(format!("{:.1}", s.y), format!("{y:.1}"), "{i}번째 y");
        }

        // 사각형이 좁아지거나 밀리면 여기서 걸린다 — 펭귄은 실제로 양 끝과 바닥에
        // 닿는다. 이 시드는 60초 안에 오른쪽 끝까지 가지 않으므로 이 확인만 길게 돈다.
        let mut 오래 = Pet::new(42, 0, &w);
        let 긴_수열 = drive(&mut 오래, 0, 300_000, 50, &w);
        assert!(긴_수열.iter().any(|s| s.x == BOUNDS.left), "왼쪽 끝에 닿는다");
        assert!(긴_수열.iter().any(|s| s.x == BOUNDS.right), "오른쪽 끝에 닿는다");
        assert!(긴_수열.iter().any(|s| s.y == BOUNDS.floor_y), "바닥에 닿는다");
    }

    #[test]
    fn 화면_판정_범위는_기준점만큼_밀려_있다() {
        let w = world();
        // 좌상단이 left에 있는 펭귄의 **발밑**은 left + PET_SIZE/2, floor_y + PET_SIZE에 있다
        assert!(w
            .screen_at(BOUNDS.left + PET_SIZE / 2.0, BOUNDS.floor_y + PET_SIZE)
            .is_some());
        // 보정하지 않고 좌상단 좌표로 물으면 화면 위가 아니다.
        // 기준점 보정이 빠지면 이 단언이 먼저 깨진다.
        assert!(w.screen_at(BOUNDS.left, BOUNDS.top).is_none());
    }

    #[test]
    fn 발밑이_속한_화면의_바닥을_따른다() {
        let w = 두_화면();
        let mut p = Pet::new(7, 0, &w);
        p.x = 2_500.0;
        p.y = 900.0;
        let s = p.step(1_000, &w);
        assert_eq!(s.y, 900.0, "오른쪽 화면의 바닥을 따른다");
        assert_eq!(p.bounds_in(&w).floor_y, 900.0);
    }

    #[test]
    fn 정확히_같은_거리면_앞_화면이_이긴다() {
        let w = 두_화면();
        // 두 화면의 기준점 가로 범위는 [70,1070]과 [2070,3070] — 그 한가운데
        let mid = (1_070.0 + 2_070.0) / 2.0;
        assert_eq!(w.nearest(mid, 800.0 + PET_SIZE).id, 1, "동거리면 목록 앞이 이긴다");
    }

    #[test]
    fn 틈에_놓인_x는_가까운_화면으로_간다() {
        let w = 두_화면();
        assert_eq!(w.screen_for_x(1_100.0).id, 1, "왼쪽 화면에 더 가깝다");
        assert_eq!(w.screen_for_x(1_950.0).id, 2, "오른쪽 화면에 더 가깝다");
    }

    #[test]
    fn 폭이_0인_화면이_섞여도_세계_폭은_전체를_덮는다() {
        let w = World::new(vec![
            Screen {
                id: 1,
                bounds: Bounds { left: 0.0, right: 0.0, top: 0.0, floor_y: 800.0 },
            },
            Screen {
                id: 2,
                bounds: Bounds { left: 2_000.0, right: 3_000.0, top: 0.0, floor_y: 800.0 },
            },
        ])
        .expect("화면이 둘이면 세계가 만들어진다");
        assert_eq!(w.width(), 3_000.0);
    }

    #[test]
    fn 새_펭귄은_x가_속한_화면에서_시작한다() {
        let w = 두_화면();
        let mut pets = Pets::new();
        let id = pets.add(7, 0, &w, 2_500.0).expect("추가된다");
        let s = pets.get(id).expect("있다").snapshot();
        assert_eq!(s.y, 900.0, "오른쪽 화면의 바닥에서 시작한다");
    }
}




