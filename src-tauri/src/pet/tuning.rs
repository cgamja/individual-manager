//! 속도·길이·확률·문턱 상수. 값을 바꾸려면 여기만 본다.
//!
//! `assert!`는 값 사이의 관계를 컴파일 시각에 묶는다.
//! `PET_SIZE`와 `BOWLING_BALL_SIZE`만 `pub` — 브릿지가 창 크기 계산에 쓴다.

/// 펭귄 한 변 (논리 px).
pub const PET_SIZE: f64 = 140.0;

// ── 이동 속도 (논리 px/초) ─────────────────────────────────────

/// 걷기.
pub(super) const WALK_SPEED: f64 = 42.0;
/// 헤엄.
pub(super) const SWIM_SPEED: f64 = 95.0;
/// 헤엄이 끝나고 날개 저어 내려앉기.
pub(super) const SWIM_DESCENT_SPEED: f64 = 210.0;
const _: () = assert!(SWIM_DESCENT_SPEED > SWIM_SPEED);
/// 슬라이딩 출발 속도 범위.
pub(super) const SLIDE_SPEED: (f64, f64) = (220.0, 340.0);
/// 굴러떨어질 때 벽 반대쪽으로 밀리는 속도.
pub(super) const TUMBLE_SPEED: f64 = 200.0;
/// 발작 중 돌진.
pub(super) const FREAKOUT_SPEED: f64 = 480.0;
const _: () = assert!(FREAKOUT_SPEED > SWIM_SPEED);

// ── 물리 ───────────────────────────────────────────────────────

/// 낙하 가속도 (논리 px/초²).
pub(super) const GRAVITY: f64 = 900.0;
/// 벽·천장에서 튈 때 남는 속도 비율.
pub(super) const BOUNCE_DAMPING: f64 = 0.5;
/// 바닥에서 통통 튈 때 가로 속도에 남는 비율.
pub(super) const FLOOR_BOUNCE_DAMPING: f64 = 0.45;
/// 이보다 느리면 통통 그만 튀고 선다.
pub(super) const BOUNCE_MIN_SPEED: f64 = 150.0;
/// 헤엄 목적지에 도착했다고 볼 거리.
pub(super) const ARRIVE_EPSILON: f64 = 6.0;
/// 한 step이 정산하는 최대 시간. 틱이 밀려도 순간이동하지 않는다.
pub(super) const MAX_STEP_MS: u64 = 250;

// ── 던지기 ─────────────────────────────────────────────────────

/// 던진 것으로 볼 최소 속도. 이보다 느리면 떨어뜨린 것이다.
pub(super) const THROW_MIN_SPEED: f64 = 260.0;
/// 던지기 속도 상한 — 초당 세계를 몇 번 가로지르는가.
///
/// **두 번의 실패 사이에 있다.** 옛 고정값 2600px/s는 1440px 화면을 0.55초에
/// 가로질러 눈이 못 따라갔고, 그걸 고치며 넣은 0.9는 1.11초라 이번엔 던진 것
/// 같지가 않았다(2026-09-02 사용자). 아래 두 `assert!`가 그 두 지점을 벽으로
/// 세운다 — 값을 만지는 다음 사람이 같은 자리로 되돌아가지 않게 한다.
pub(super) const THROW_MAX_WORLDS_PER_SEC: f64 = 1.4;
/// 상한 속도로 세계를 가로지르는 데 걸리는 시간(초)은 비율의 역수다 — 세계
/// 폭과 무관하다. 그게 이 상수를 비율로 둔 이유이기도 하다.
const THROW_CROSS_SEC: f64 = 1.0 / THROW_MAX_WORLDS_PER_SEC;
const _: () = assert!(THROW_CROSS_SEC > 0.6, "너무 빠르다 — 2600px/s 시절로 돌아간다");
const _: () = assert!(THROW_CROSS_SEC < 0.9, "너무 느리다 — 던진 것 같지 않다");
/// 세계 폭을 못 구했을 때 쓸 기준 폭.
pub(super) const FALLBACK_WORLD_WIDTH: f64 = 1_440.0;

// ── 착지 ───────────────────────────────────────────────────────

/// 이 속도 이상이면 철푸덕.
pub(super) const SPLAT_MIN_IMPACT: f64 = 700.0;
/// 이 속도 이상이면 널브러짐.
pub(super) const SPRAWL_MIN_IMPACT: f64 = 1_000.0;
const _: () = assert!(SPRAWL_MIN_IMPACT > SPLAT_MIN_IMPACT);
/// 착지 스쿼시 길이.
pub(super) const LAND_MS: u64 = 300;
/// 철푸덕 길이.
pub(super) const SPLAT_MS: u64 = 850;
/// 널브러짐 길이.
pub(super) const SPRAWL_MS: u64 = 1_700;
const _: () = assert!(SPLAT_MS > LAND_MS);
const _: () = assert!(SPRAWL_MS > SPLAT_MS);
/// 일어난 뒤 약을 올릴 확률(%).
pub(super) const SASSY_AFTER_LAND_PERCENT: u64 = 70;

// ── 스윙 넉백 ──────────────────────────────────────────────────
//
// 휘두른 방망이가 **앞에 있는 다른 마리**를 날린다. 판정은 커맨드 경로
// (`Pets::whack`)에 있고 난수를 쓰지 않는다.

/// 방망이가 닿는 거리 — **몸통 가운데끼리** 잰다.
///
/// **`PET_SIZE`보다 커야 한다.** 작으면 어깨를 맞대고 선 이웃조차 중심 거리가
/// 사거리 밖이라 한 마리도 안 날아간다 — 기능이 통째로 조용히 죽는다.
pub(super) const SWING_REACH: f64 = 200.0;
const _: () = assert!(SWING_REACH > PET_SIZE);

/// 방망이가 닿는 **위아래** 폭. 몸통보다 좁다 — 한 층 위를 헤엄쳐 지나가는
/// 마리까지 맞으면 "방망이"가 아니라 장판이다.
pub(super) const SWING_REACH_V: f64 = 100.0;
const _: () = assert!(SWING_REACH_V < PET_SIZE);

/// 맞은 이웃이 날아가는 속도 — 초당 세계를 몇 번 가로지르는가.
///
/// **손으로 던지기보다 느리다.** 조준해서 던진 것이 옆에서 스친 것보다 세야
/// "던졌다"와 "스쳤다"의 그림이 뒤집히지 않는다. 아래 `assert!`가 그 순서를
/// 붙든다 — 볼링 핀(1.5) > 던지기(1.4) > 스윙 넉백 > 핀볼 채(0.8) 순이다.
pub(super) const SWING_KNOCK_WORLDS_PER_SEC: f64 = 1.0;
const _: () = assert!(SWING_KNOCK_WORLDS_PER_SEC < THROW_MAX_WORLDS_PER_SEC);
/// 아래쪽 절반도 함께 붙든다 — 주장하는 관계마다 단언을 하나씩 다는 것이 이
/// 파일의 관행이고, 주석만 있고 단언이 없으면 다음 사람이 0.7로 내려도 조용하다.
const _: () = assert!(SWING_KNOCK_WORLDS_PER_SEC > PINBALL_HIT_WORLDS_PER_SEC);

/// 날아가는 각도 — 앞으로 1일 때 **위로** 얼마인가. 0이면 바닥을 기고, 1에
/// 가까우면 옆으로 안 가고 제자리에서 솟았다 떨어진다.
pub(super) const SWING_KNOCK_LIFT: f64 = 0.5;
const _: () = assert!(SWING_KNOCK_LIFT > 0.0);
const _: () = assert!(SWING_KNOCK_LIFT < 1.0);

// ── 핀볼 모드 ──────────────────────────────────────────────────

/// 모든 면이 범퍼일 때 남는 속도 비율.
pub(super) const PINBALL_DAMPING: f64 = 0.92;
/// 채로 후려칠 때의 속도 — 초당 세계를 몇 번 가로지르는가.
pub(super) const PINBALL_HIT_WORLDS_PER_SEC: f64 = 0.8;
const _: () = assert!(PINBALL_HIT_WORLDS_PER_SEC < THROW_MAX_WORLDS_PER_SEC);
const _: () = assert!(PINBALL_DAMPING < 1.0);
const _: () = assert!(PINBALL_DAMPING > BOUNCE_DAMPING);

/// 마리끼리 부딪히는 거리 — **몸통 중심 사이**다. 창은 한 변 `PET_SIZE`인 정사각형이지만
/// 펭귄 그림은 그보다 좁아, `PET_SIZE`를 그대로 쓰면 눈에 안 닿았는데 튕긴다. 반대로
/// 절반보다 작으면 거의 겹쳐야 튕겨 통과한 것처럼 보인다. 두 `assert!`가 그 사이에 묶는다.
pub(super) const PINBALL_COLLIDE_RADIUS: f64 = 104.0;
const _: () = assert!(PINBALL_COLLIDE_RADIUS < PET_SIZE);
const _: () = assert!(PINBALL_COLLIDE_RADIUS > PET_SIZE / 2.0);

/// 마리끼리 부딪힐 때 남는 속도 비율(반발 계수). **1보다 작아야 한다** — 1이면 여덟 마리가
/// 뒤엉킬 때 에너지가 줄지 않아 영영 안 멎고, 20Hz 틱도 영영 안 쉰다. 벽(`PINBALL_DAMPING`)
/// 보다 살짝 무른 이유는 벽이 판의 테두리라 랠리를 살려야 하고, 마리끼리는 매 번 두
/// 마리의 속도를 함께 흔들어 같은 계수로도 훨씬 시끄럽기 때문이다.
pub(super) const PINBALL_BUMP_DAMPING: f64 = 0.9;
const _: () = assert!(PINBALL_BUMP_DAMPING < 1.0);
/// 주석이 근거로 드는 관계를 그대로 묶는다. `> BOUNCE_DAMPING`(0.5)만으로는 0.99를
/// 넣어도 통과해 아무것도 못 막는다.
const _: () = assert!(PINBALL_BUMP_DAMPING < PINBALL_DAMPING);

/// 부딪힌 것으로 칠 **최소 상대 속도**.
///
/// 판정은 `vx`/`vy`가 아니라 **이번 틱에 지나온 거리**로 속도를 잰다 — 그 둘은 던져졌을
/// 때만 0이 아니라서, 틱 안에서 날아와 착지까지 끝낸 마리는 틱 끝에 속도가 0이고
/// 미끄러지는 마리는 처음부터 0이다. 대신 위치로 움직이는 평소 동작까지 전부 보이게
/// 되므로 문턱이 필요하다: **마주 걸어오는 두 마리(42×2)는 이 아래**라 스쳐도 안
/// 튕기고, 굴러떨어지기(200)·미끄러지기(220)·발작(480)은 위다.
pub(super) const PINBALL_BUMP_MIN_SPEED: f64 = 120.0;
const _: () = assert!(PINBALL_BUMP_MIN_SPEED > WALK_SPEED * 2.0);
const _: () = assert!(PINBALL_BUMP_MIN_SPEED < TUMBLE_SPEED);

/// 바닥에 선 마리가 맞았을 때 속도가 **위로 도는 비율**. 0이면 맞은 것이 안 보인다 —
/// 수평 속도만 받으면 다음 틱에 곧바로 다시 착지해 한 틱 미끄러지고 끝난다.
///
/// **속력을 더하는 값이 아니라 방향을 트는 값이다** (`Pet::bumped`). 세로만 얹으면
/// 속력이 √(1+비율²)배로 늘어 바닥 높이 충돌의 실효 반발 계수가 위 상수를 넘어선다.
pub(super) const PINBALL_BUMP_LIFT: f64 = 0.35;
const _: () = assert!(PINBALL_BUMP_LIFT > 0.0);
const _: () = assert!(PINBALL_BUMP_LIFT < 1.0);

// ── 동작 길이 (ms) ─────────────────────────────────────────────
//
// CSS 애니메이션 길이와 맞아야 한다 (`pet-css.test.ts`가 대조한다).

/// 제자리에서 도는 시간.
pub(super) const TURN_MS: u64 = 250;
/// 굴러떨어지는 시간.
pub(super) const TUMBLE_MS: u64 = 1_100;
const _: () = assert!(TUMBLE_MS > TURN_MS);
/// 미끄러지는 시간.
pub(super) const SLIDE_MS: u64 = 2_400;
const _: () = assert!(SLIDE_MS > TURN_MS);
/// 방망이 한 번 휘두르는 시간.
pub(super) const SWING_MS: u64 = 360;
/// 약 올리는 시간.
pub(super) const SASSY_MS: u64 = 900;
/// 빽빽거리는 시간.
pub(super) const SQUAWK_MS: u64 = 1_400;
const _: () = assert!(SQUAWK_MS > SASSY_MS);
/// 말풍선이 떠 있는 시간.
pub(super) const SPEECH_MS: u64 = 3_200;
/// 말과 말 사이 간격 범위.
pub(super) const TAUNT_GAP_MS: (u64, u64) = (7_000, 18_000);
/// 걷기 한 번의 길이 범위.
pub(super) const WALK_MS: (u64, u64) = (2_500, 6_000);
/// 유휴 한 번의 길이 범위.
pub(super) const IDLE_MS: (u64, u64) = (1_200, 3_200);
/// 졸기 한 번의 길이 범위.
pub(super) const SLEEP_MS: (u64, u64) = (12_000, 25_000);

// ── 다음 동작 추첨 ─────────────────────────────────────────────

/// 자극이 없으면 이만큼 뒤에 존다.
pub(super) const SLEEP_AFTER_MS: u64 = 300_000;
/// 걷기 뒤 또 걸을 확률(%). 나머지는 유휴.
pub(super) const WALK_AGAIN_PERCENT: u64 = 72;
/// 유휴 뒤 헤엄칠 확률(%).
pub(super) const SWIM_PERCENT: u64 = 30;
/// 걷기 뒤 미끄러질 확률(%). 유휴 뒤에는 안 나온다.
pub(super) const SLIDE_AFTER_WALK_PERCENT: u64 = 20;
/// 벽에 닿았을 때 돌아서지 않고 굴러떨어질 확률(%).
pub(super) const TUMBLE_AT_WALL_PERCENT: u64 = 30;
/// 얼음낚시를 시작할 확률(‰).
pub(super) const ICE_FISHING_PERMILLE: u64 = 7;
/// 발작이 터지는 확률 — 이 횟수에 한 번.
pub(super) const FREAKOUT_ONE_IN: u64 = 30_000;
/// 헤엄이 끝날 때 자유낙하로 빠지는 비율(%). 나머지는 내려앉는다.
pub(super) const SWIM_FREEFALL_PERCENT: u64 = 90;

// ── 빽빽거리기 ─────────────────────────────────────────────────

/// 이만큼 연달아 맞으면 터진다.
pub(super) const SQUAWK_WHACK_COUNT: u64 = 20;
/// 연타로 셀 최대 간격.
pub(super) const SQUAWK_GAP_MS: u64 = 2_000;

// ── 발작 ───────────────────────────────────────────────────────

/// 돌진하는 시간 범위.
pub(super) const FREAKOUT_MS: (u64, u64) = (2_000, 4_000);
const _: () = assert!(FREAKOUT_MS.1 >= FREAKOUT_MS.0);
/// 숨 고르는 시간.
pub(super) const FREAKOUT_PANT_MS: u64 = 700;
/// 한 번 튈 때 움직이는 거리 범위.
pub(super) const FREAKOUT_HOP: (f64, f64) = (100.0, 220.0);

// ── 얼음낚시 ───────────────────────────────────────────────────

/// 구멍 뚫는 시간.
pub(super) const FISHING_DIG_MS: u64 = 1_400;
/// 드리우고 기다리는 시간 범위.
pub(super) const FISHING_WAIT_MS: (u64, u64) = (4_000, 9_000);
/// 입질 시간.
pub(super) const FISHING_BITE_MS: u64 = 700;
/// 잡았을 때 보여주는 시간.
pub(super) const FISHING_CATCH_MS: u64 = 1_800;
/// 꽝일 때 시간.
pub(super) const FISHING_MISS_MS: u64 = 1_300;
/// 정리하고 일어나는 시간.
pub(super) const FISHING_PACK_MS: u64 = 700;
/// 한 판 전체 길이 범위. 판을 끝내는 것은 이 예산뿐이다.
pub(super) const FISHING_SESSION_MS: (u64, u64) = (30_000, 60_000);
const _: () = assert!(FISHING_SESSION_MS.0 > FISHING_DIG_MS + FISHING_WAIT_MS.1);
const _: () = assert!(FISHING_SESSION_MS.1 >= FISHING_SESSION_MS.0);
/// 채서 물고기가 딸려 나올 확률(%).
pub(super) const FISHING_CATCH_PERCENT: u64 = 40;

// ── 볼링 ───────────────────────────────────────────────────────
//
// 판 전체가 몇 초짜리 한 번이라 확률이 하나도 없다 — 시작은 버튼뿐이고
// 공 물리는 완전 결정적이다 (R12). 여기 값들은 전부 "보기에 볼링 같은가"로 정했다.

/// 공 지름 (논리 px). 펭귄만큼 공들이지 않는다 — 원 하나에 손가락 구멍 셋이다 (A6).
pub const BOWLING_BALL_SIZE: f64 = 64.0;
const _: () = assert!(BOWLING_BALL_SIZE < PET_SIZE);

/// 삼각 대형에서 **줄과 줄 사이**(가로) 간격. 펭귄 폭보다 좁아 살짝 겹친다 —
/// 벌려 놓으면 삼각형이 아니라 그냥 흩어져 뜬 펭귄들로 보인다.
pub(super) const BOWLING_ROW_GAP: f64 = 104.0;
const _: () = assert!(BOWLING_ROW_GAP < PET_SIZE);

/// 삼각 대형에서 **한 줄 안**(세로) 간격.
pub(super) const BOWLING_COL_GAP: f64 = 118.0;
const _: () = assert!(BOWLING_COL_GAP < PET_SIZE);

/// 오른쪽 끝에서 첫 핀까지 띄우는 거리. 공이 마지막 핀을 지나 빠져나갈 자리다.
pub(super) const BOWLING_PIN_MARGIN: f64 = 24.0;

/// 공 자리에서 삼각형 꼭짓점까지 **반드시** 남기는 길이. 대형이 넓어지면
/// 공까지 뻗는데, 그러면 굴리기 전에 이미 닿아 있다 (A5).
pub(super) const BOWLING_LANE_MIN: f64 = 240.0;

/// 핀 자리로 **날아가는** 속도. 판은 바닥이 아니라 화면 세로 중앙에 서므로
/// 걷는 게 아니라 헤엄쳐 간다. 헤엄보다 빠르다 — 평소 헤엄(95px/s)으로 가면
/// 다 서는 데 십 초가 넘어 판이 시작되기 전에 지친다.
pub(super) const BOWLING_GATHER_SPEED: f64 = 420.0;
const _: () = assert!(BOWLING_GATHER_SPEED > SWIM_SPEED);
const _: () = assert!(BOWLING_GATHER_SPEED < FREAKOUT_SPEED);

/// 맞은 핀이 튕겨 나가는 속도 — 초당 세계를 몇 번 가로지르는가. 던지기보다
/// 세다: 볼링공에 맞은 핀이 살살 밀려나면 맞은 것으로 안 보인다.
///
/// **던지기 상한을 올리면 이 값이 따라 올라간다** — 아래 `assert!`가 그 순서를
/// 붙들고 있다. 던지기만 올리면 손으로 던진 펭귄이 볼링공에 맞은 핀보다 빨라져
/// "맞았다"는 그림이 뒤집힌다.
pub(super) const BOWLING_KNOCK_WORLDS_PER_SEC: f64 = 1.5;
const _: () = assert!(BOWLING_KNOCK_WORLDS_PER_SEC > THROW_MAX_WORLDS_PER_SEC);

/// 튕겨 나간 핀이 아직 선 핀을 치는 거리. **연쇄가 이 값으로 산다.**
///
/// **대형의 이웃 거리보다 커야 한다.** 작으면 나란히 선 핀들이 서로 안 닿아
/// 공이 지나는 한 줄만 쓰러지고 끝난다 — 처음에 96으로 뒀다가 실제로 그랬다.
/// 아래 두 `assert!`가 대형 간격을 바꿀 때 이 값이 따라오도록 묶는다.
pub(super) const BOWLING_KNOCK_RADIUS: f64 = 126.0;
/// 옆줄의 대각선 이웃 — `sqrt(ROW_GAP² + (COL_GAP/2)²)`를 제곱으로 비교한다.
const _: () = assert!(
    BOWLING_KNOCK_RADIUS * BOWLING_KNOCK_RADIUS
        >= BOWLING_ROW_GAP * BOWLING_ROW_GAP
            + (BOWLING_COL_GAP / 2.0) * (BOWLING_COL_GAP / 2.0)
);
/// 같은 줄의 위아래 이웃.
const _: () = assert!(BOWLING_KNOCK_RADIUS >= BOWLING_COL_GAP);

/// 흩어지며 일어나는 시간. 얼음낚시의 `Pack`, 발작의 `Pant`와 같은 귀결 국면이다.
pub(super) const BOWLING_SCATTER_MS: u64 = 600;

/// 공이 멎고 펭귄들이 흩어지기까지의 뜸.
pub(super) const BOWLING_SETTLE_MS: u64 = 900;

/// 판이 어떤 이유로도 마리를 이보다 오래 붙들지 못한다. 판이 사라져도 펭귄이
/// 영원히 서 있지 않게 하는 안전장치다 (R11).
pub(super) const BOWLING_MAX_MS: u64 = 120_000;
const _: () = assert!(BOWLING_MAX_MS > BOWLING_SETTLE_MS);

/// 굴리기 속도 상한 — 초당 세계를 몇 번 가로지르는가. 던지기보다 느리다:
/// 공은 바닥을 구르지 날아가지 않는다.
pub(super) const BOWLING_MAX_WORLDS_PER_SEC: f64 = 0.75;
const _: () = assert!(BOWLING_MAX_WORLDS_PER_SEC < THROW_MAX_WORLDS_PER_SEC);

/// 굴러가는 공의 감속도 — **초당 세계 폭의 몇 배씩 속도가 줄어드는가.**
///
/// 둘을 함께 지킨다. (1) **비율이 아니라 감속도다** — 매 틱 비율로 줄이면
/// 속도가 0에 닿지 않아 20Hz 틱이 영영 안 쉰다. (2) **세계 폭에 비례한다** —
/// 고정값으로 두면 화면이 넓어질수록 공이 상대적으로 덜 굴러, 아무리 세게
/// 굴려도 끝 핀에 못 닿는다. 실제로 한 번 그렇게 짰고 테스트가 잡았다.
pub(super) const BOWLING_DECEL_WORLDS_PER_SEC2: f64 = 0.15;
const _: () = assert!(BOWLING_DECEL_WORLDS_PER_SEC2 > 0.0);

/// 최대 세기로 굴린 공이 세계를 몇 번 가로지르고 멎는가 — `v² / 2a`를 세계 폭
/// 단위로 쓴 값이다. 레인은 세계보다 펭귄 한 마리만큼 길므로 **1.2를 넘어야**
/// 최대 세기가 끝 핀을 지나 빠져나간다.
const BOWLING_ROLL_WORLDS: f64 =
    BOWLING_MAX_WORLDS_PER_SEC * BOWLING_MAX_WORLDS_PER_SEC / (2.0 * BOWLING_DECEL_WORLDS_PER_SEC2);
const _: () = assert!(BOWLING_ROLL_WORLDS > 1.2);

/// 이보다 느려지면 공이 멎는다. 감속도와 짝인 정지 문턱이다.
pub(super) const BOWLING_STOP_SPEED: f64 = 40.0;

/// 굴린 것으로 볼 최소 속도. 이보다 살살 놓으면 공은 그 자리에 남고 다시 집을 수 있다.
pub(super) const BOWLING_MIN_ROLL_SPEED: f64 = 120.0;
const _: () = assert!(BOWLING_MIN_ROLL_SPEED > BOWLING_STOP_SPEED);

/// 세계가 아주 좁을 때 속도 **상한**이 내려갈 수 있는 바닥. 던지기의
/// `THROW_MIN_SPEED`를 빌려 쓰지 않는다 — 빌리면 던지기를 튜닝할 때 볼링의
/// 최소 굴리기 속도가 조용히 따라 바뀐다. 볼링 상수는 전부 독립이다.
pub(super) const BOWLING_MIN_MAX_SPEED: f64 = 260.0;
const _: () = assert!(BOWLING_MIN_MAX_SPEED > BOWLING_MIN_ROLL_SPEED);

/// 공 중심이 펭귄 중심에서 이 거리 안에 들어오면 맞는다.
pub(super) const BOWLING_HIT_RADIUS: f64 = 52.0;

/// 펭귄 하나를 지나갈 때마다 잃는 속도 비율. **멈추지는 않는다** — 첫 펭귄에서
/// 멈추면 마릿수가 무의미해진다 (A2).
pub(super) const BOWLING_SPEED_LOSS_PER_PIN: f64 = 0.12;
const _: () = assert!(BOWLING_SPEED_LOSS_PER_PIN > 0.0);
const _: () = assert!(BOWLING_SPEED_LOSS_PER_PIN < 1.0);

// ── 비치발리볼 ─────────────────────────────────────────────────
//
// **관전형 한 판이다** — 사용자가 아무것도 안 한다. 그래서 실패 모드가 "물리가
// 틀렸다"가 아니라 "20초 동안 보고 있기 지루하다"이고, 아래 값의 절반은 그 답이다.
// 랠리는 판이 가진 시드 난수로 만들고(PRINCIPLE 3), 판을 끝내는 것은 예산뿐이다.

/// 비치볼 지름 (논리 px). 브릿지가 공 창 크기에 쓴다.
pub const VOLLEY_BALL_SIZE: f64 = 56.0;
const _: () = assert!(VOLLEY_BALL_SIZE < PET_SIZE);

/// 네트 그물의 높이 — **모래에 발을 딛는다.**
///
/// **따로 고르는 값이 아니라 `VOLLEY_NET_DROP`에서 나온다.** 그물 꼭대기가 펭귄
/// 좌상단보다 `NET_DROP` 아래이고 모래는 `PET_SIZE` 아래이므로, 그 사이가 곧
/// 그물 높이다. 둘을 따로 두고 `합 == PET_SIZE`를 단언하면 값 하나를 만질 때마다
/// **동기가 전혀 다른 두 상수**(하나는 "머리 아래", 하나는 "그물이 얼마나 커
/// 보이나")가 부동소수 등호로 묶여 컴파일이 깨진다.
pub(super) const VOLLEY_NET_HEIGHT: f64 = PET_SIZE - VOLLEY_NET_DROP;

/// 코트 창이 **발밑 선 아래로** 잡는 깊이. 판이 통째로 떠 있으므로 이건 화면
/// 바닥까지 내려가는 길이가 아니다.
///
/// **그려지는 모래톱은 이보다 얇다** — 가운데가 이만큼이고 가장자리로 갈수록
/// 얇아진다(단면이 아래로 볼록하다). 창은 가장 두꺼운 데를 기준으로 잡는다.
///
/// **모래는 화면 바닥이 아니라 판에 있다.** 한 번 화면 바닥에 그렸다가
/// 되돌렸다 — 판을 세로 중앙으로 올린 이유가 "펭귄·공·네트가 한 띠 안에서
/// 겹치지 않게"인데, 모래만 바닥에 남기면 **펭귄이 딛고 선 면이 사라져**
/// 코트가 허공에 뜬 그림이 된다. 볼링의 레인이 통째로 떠 있는 것과 같다.
pub(super) const VOLLEY_SAND_DEPTH: f64 = 80.0;

/// 모래톱이 코트 좌우로 더 뻗는 길이. 끝에 선 펭귄의 발밑도 모래여야 한다.
pub(super) const VOLLEY_SAND_MARGIN: f64 = PET_SIZE / 2.0;

/// 펭귄이 공을 치는 높이 — 펭귄 `y`(좌상단)보다 이만큼 **위**다.
pub(super) const VOLLEY_REACH: f64 = 40.0;

/// 네트 꼭대기가 펭귄 `y`(좌상단)보다 이만큼 **아래**다.
///
/// **머리 아래여야 한다.** `assets/penguin/body.tsx`의 머리는 viewBox에서 y 14~46이고
/// 140px 무대로 1.077배 늘어나므로 화면에서는 15~50이다. 10으로 뒀더니 네트
/// 윗줄이 얼굴을 가로질렀다. 55면 목 아래에 걸린다.
///
/// 값이 커진 만큼 **킬샷의 네트 여유도 함께 늘었다** — 아래 `VOLLEY_NET_CLEAR` 참고.
pub(super) const VOLLEY_NET_DROP: f64 = 55.0;

/// **타점이 네트 꼭대기보다 높다 — 이 한 줄이 네트 판정을 통째로 없앤다** (KTD6).
///
/// 판을 화면 세로 중앙으로 올리면서 근거가 **더 단순해졌다**: 예전에는 네트가
/// 모래에 서 있어서 "네트 높이 < 펭귄 키 + 손 높이"라는 비교가 필요했지만,
/// 이제 네트도 판에 함께 뜨고 타점은 펭귄 위, 네트 꼭대기는 펭귄 아래라
/// **부호만으로 성립한다.** 타점에서 출발해 타점으로 돌아오는 포물선은 전
/// 구간이 타점 이상이라 네트에 걸릴 수가 없다.
/// 그물이 남을 자리가 있어야 한다 — `NET_DROP`이 키만큼 커지면 높이가 0이 된다.
const _: () = assert!(VOLLEY_NET_DROP < PET_SIZE);

/// 한 틱이 정산하는 시간이 길수록 준음함수 오일러 적분이 공을 **아래로 치우치게**
/// 한다(`g·s·dt/2`). 정상 틱(50ms)에서 20px쯤이라 그만큼 미리 빼 둔다.
///
/// **밀린 틱(`MAX_STEP_MS` 250ms)까지 덮지는 않는다** — 그러면 100px을 빼야 해서
/// 킬샷이 코트 끝까지 밀린다. 대가는 **표본이 그물 안으로 최대 17px 들어가는
/// 것**이다(dt를 10~250ms로 훑어 잰 최악값, dt≈197ms에서 나온다. 85px 그물의
/// 5분의 1이다). 정상 틱 50ms에서는 50px이 남는다 — 20Hz가 네 배로 밀리는
/// 상황 자체가 드물고 그때는 판 전체가 끊겨 보인다.
pub(super) const VOLLEY_EULER_SLACK: f64 = 24.0;

/// 킬샷이 네트를 넘는지 계산할 때 쓰는 **실효 여유.**
///
/// 타점과 네트 꼭대기 사이(`REACH + NET_DROP`)에서 둘을 뺀다. 안 빼면 계산은
/// 통과하는데 **화면에서는 공이 그물을 뚫고 지나간다** — 실제로 그랬다:
/// 중심 여유 14px, 공 아래쪽은 네트 안으로 14px 들어갔다.
///
/// 1. **공은 점이 아니다** — 반지름 `VOLLEY_BALL_SIZE / 2`만큼 아래가 더 내려간다.
/// 2. **적분이 아래로 치우친다** — `VOLLEY_EULER_SLACK`.
pub(super) const VOLLEY_NET_CLEAR: f64 =
    VOLLEY_REACH + VOLLEY_NET_DROP - VOLLEY_BALL_SIZE / 2.0 - VOLLEY_EULER_SLACK;
const _: () = assert!(VOLLEY_NET_CLEAR > 0.0);

/// 네트 그물의 **반폭.** 네트는 선이 아니라 폭을 가지므로, 넘는지 따질 때는
/// 공이 가장 낮게 지나는 **먼 쪽 모서리**를 봐야 한다 — 가운데 선만 보면
/// 그 몇십 px 뒤에서 그물에 걸린다.
pub(super) const VOLLEY_NET_HALF_W: f64 = 48.0;
/// **런타임의 여백과 비교해야 한다.** `Court::new`가 좁은 화면에서 `gap`을
/// `half / 2`까지 줄이므로, 상수끼리만 견주면 그 축소를 놓친다. 아래 두 단언이
/// "가장 좁은 세계에서도 `gap`은 안 줄어든다 → 그러니 그물보다 넓다"를 잇는다.
const _: () = assert!(VOLLEY_NET_GAP * 4.0 <= VOLLEY_MIN_WORLD_WIDTH);
const _: () = assert!(VOLLEY_NET_HALF_W < VOLLEY_NET_GAP);

/// 네트에서 가장 가까운 자리까지 (몸통 가운데 기준). 네트에 딱 붙어 서면
/// 공을 넘기는 게 아니라 네트를 넘겨다보는 그림이 된다.
pub(super) const VOLLEY_NET_GAP: f64 = 110.0;

/// 네트에서 코트 끝까지.
pub(super) const VOLLEY_COURT_HALF: f64 = 460.0;
/// 한쪽에 최소한 펭귄 하나가 여유 있게 설 폭은 남아야 한다.
const _: () = assert!(VOLLEY_COURT_HALF > VOLLEY_NET_GAP + PET_SIZE);

/// 이보다 좁은 세계에서는 판을 열지 않는다 — 코트가 안 들어간다.
pub(super) const VOLLEY_MIN_WORLD_WIDTH: f64 = 2.0 * (VOLLEY_NET_GAP + PET_SIZE);

/// 세로로도 이만큼은 있어야 한다.
///
/// **판이 화면 세로 중앙으로 올라가면서 이 값이 두 배가 됐다.** 공이 뜰 수 있는
/// 높이가 세계 전체가 아니라 **판 위쪽 절반**뿐이기 때문이다. 좁으면
/// `flight_ms_for`의 천장 자르기가 체공 세 등급을 하나로 뭉개 **리듬이 갈리는
/// 갈래(KTD3-3)가 통째로 죽는다** — 아래 단언이 그 지점을 막는다.
pub(super) const VOLLEY_MIN_WORLD_HEIGHT: f64 = 420.0;

/// 판을 열 수 있는 최소 마릿수. **한 마리면 팀이 안 나온다** (R3).
pub(super) const VOLLEY_MIN_PETS: usize = 2;
const _: () = assert!(VOLLEY_MIN_PETS >= 2);

/// 자기 자리로 **날아가는** 속도. 볼링과 같은 이유로 헤엄보다 빠르다 — 평소
/// 헤엄으로 가면 다 서는 데만 십 초가 걸려 판이 시작하기 전에 지친다.
pub(super) const VOLLEY_GATHER_SPEED: f64 = 420.0;
const _: () = assert!(VOLLEY_GATHER_SPEED > SWIM_SPEED);
const _: () = assert!(VOLLEY_GATHER_SPEED < FREAKOUT_SPEED);

/// 받으러 뛰는 속도. 걷기보다 훨씬 빠르고 슬라이딩 상한보다 느리다 —
/// 배를 깔지 않고 두 발로 뛰는 그림이다.
pub(super) const VOLLEY_CHASE_SPEED: f64 = 300.0;
const _: () = assert!(VOLLEY_CHASE_SPEED > WALK_SPEED);
const _: () = assert!(VOLLEY_CHASE_SPEED < SLIDE_SPEED.1);

/// 공에만 쓰는 중력. **`GRAVITY`를 빌리지 않는다** — 빌리면 착지 튜닝이 랠리
/// 리듬을 조용히 바꾼다 (볼링 상수가 던지기를 안 빌린 것과 같은 규칙).
pub(super) const VOLLEY_GRAVITY: f64 = 1_200.0;

/// 체공 시간 세 등급 (ms) — **스파이크·평타·토스.**
///
/// **랠리에서 체감이 가장 큰 갈래다.** 포물선의 정점이 체공의 제곱에 비례하므로
/// 리듬과 그림이 함께 갈린다 — 같은 속도로 열두 번 오가는 것이 정확히 지루함이다.
pub(super) const VOLLEY_FLIGHT_MS: [u64; 3] = [550, 950, 1_500];
const _: () = assert!(VOLLEY_FLIGHT_MS[0] < VOLLEY_FLIGHT_MS[1]);
const _: () = assert!(VOLLEY_FLIGHT_MS[1] < VOLLEY_FLIGHT_MS[2]);

/// **가장 좁은 세계에서도 체공 세 등급이 갈려야 한다.** 판 위쪽 높이가
/// `MIN_WORLD_HEIGHT/2 - REACH`이고 정점이 `g·T²/8`이므로 천장이 정하는 상한은
/// `sqrt(8h/g)`다. 그게 평타보다 짧으면 세 등급이 둘로 뭉개진다.
const VOLLEY_MIN_HEADROOM: f64 = VOLLEY_MIN_WORLD_HEIGHT / 2.0 - VOLLEY_REACH;
const _: () = assert!(
    8.0 * VOLLEY_MIN_HEADROOM / VOLLEY_GRAVITY
        > (VOLLEY_FLIGHT_MS[1] as f64 / 1_000.0) * (VOLLEY_FLIGHT_MS[1] as f64 / 1_000.0)
);

/// 받을 마리가 목적지에 도착하고 남는 여유.
pub(super) const VOLLEY_ARRIVE_MARGIN_MS: u64 = 250;
const _: () = assert!(VOLLEY_ARRIVE_MARGIN_MS < VOLLEY_FLIGHT_MS[0]);

/// **가장 긴 체공으로 코트 반쪽을 가로지를 수 있어야 한다.** 못 하면 아무리 길게
/// 띄워도 받을 마리가 못 닿아 **랠리가 첫 왕복에서 끝난다** — 20초짜리 기능이
/// 1초짜리가 되는데 테스트도 로그도 깨끗하다.
const _: () = assert!(
    VOLLEY_CHASE_SPEED * ((VOLLEY_FLIGHT_MS[2] - VOLLEY_ARRIVE_MARGIN_MS) as f64 / 1_000.0)
        >= VOLLEY_COURT_HALF - VOLLEY_NET_GAP
);

/// 목적지를 **받는 팀에서 먼 쪽으로** 얼마나 끌어당기는가 (0~1).
///
/// **마릿수가 늘수록 `Chase`가 사라지는 것을 막는다.** 목적지를 균등하게만
/// 뽑으면 여덟 마리일 때 이웃 간격(~117px)이 사정거리(`VOLLEY_REACH_X` 70px)에
/// 가까워, 뽑힌 자리가 이미 누군가의 사정거리 안이라 아무도 안 뛴다 — 실측으로
/// 여덟 마리 한 판의 뛰는 시간이 2.5초까지 내려갔다. 랠리 화면의 절반을
/// 채워야 할 갈래가 정확히 최대 마릿수에서 죽는다.
///
/// 1.0으로 두지 않는다 — 그러면 매번 같은 빈자리를 노려 목적지가 예측 가능해진다.
pub(super) const VOLLEY_AWAY_BIAS: f64 = 0.75;
const _: () = assert!(VOLLEY_AWAY_BIAS > 0.0);
const _: () = assert!(VOLLEY_AWAY_BIAS < 1.0);

/// 공 중심이 받을 마리의 몸통 가운데에서 이 거리 안이면 닿는다.
pub(super) const VOLLEY_REACH_X: f64 = 70.0;
const _: () = assert!(VOLLEY_REACH_X < PET_SIZE);

/// 때리는 자세 길이.
pub(super) const VOLLEY_BUMP_MS: u64 = 380;

/// 서브 토스 — 자기 위로 띄웠다가 다시 받기까지. **서브는 국면이 아니라
/// "자기에게 보내는 왕복 0번"이라** 이 값이 곧 그 왕복의 체공이다.
pub(super) const VOLLEY_SERVE_MS: u64 = 800;
const _: () = assert!(VOLLEY_SERVE_MS > VOLLEY_BUMP_MS);
/// **서브만 천장 자르기를 안 거친다.** 랠리 왕복은 `flight_ms_for`가 정점을
/// 화면 안에 가두지만 서브는 이 값을 그대로 쓰므로, 관계를 여기서 묶는다 —
/// 안 묶으면 이 값을 1초로 올리는 순간 서브가 화면 위로 사라진다.
const VOLLEY_SERVE_APEX: f64 = VOLLEY_GRAVITY
    * (VOLLEY_SERVE_MS as f64 / 1_000.0)
    * (VOLLEY_SERVE_MS as f64 / 1_000.0)
    / 8.0;
const _: () = assert!(VOLLEY_SERVE_APEX < VOLLEY_MIN_WORLD_HEIGHT - VOLLEY_REACH);

/// 좋아하거나 약 오르는 시간. **`SASSY_MS`와 같아야 한다** — CSS가 싸가지 반응의
/// keyframe(`pg-butt-wiggle`·`pg-turn-away`)을 그대로 참조하므로, 어긋나면
/// 자세가 중간에 멈춘 채 끝난다.
pub(super) const VOLLEY_CHEER_MS: u64 = SASSY_MS;

/// 공이 모래에 닿고 코트가 걷히기까지의 뜸. **축하보다 길다** — 모래가 먼저
/// 사라지면 축하가 허공에서 끝난다.
pub(super) const VOLLEY_POINT_MS: u64 = VOLLEY_CHEER_MS + 200;
const _: () = assert!(VOLLEY_POINT_MS > VOLLEY_CHEER_MS);

/// 한 판의 예산 범위 — **"20초쯤"** (R9). 얼음낚시와 같은 꼴로, **판을 끝내는 것은
/// 이것뿐이다**: 예산이 지나면 다음 왕복이 킬샷이 되고 그 뒤로는 아무도 못 받는다.
/// "몇 번 오가면 끝"으로 하면 길이가 4초에서 30초까지 튄다.
pub(super) const VOLLEY_SESSION_MS: (u64, u64) = (18_000, 22_000);
const _: () = assert!(VOLLEY_SESSION_MS.1 >= VOLLEY_SESSION_MS.0);
/// 예산 안에 왕복이 열 번 넘게 들어가야 랠리로 보인다 — 가장 긴 체공만 나와도.
const _: () = assert!(VOLLEY_SESSION_MS.0 / VOLLEY_FLIGHT_MS[2] >= 10);

/// 판이 어떤 이유로도 마리를 이보다 오래 붙들지 못한다 (R14의 마지막 장치).
pub(super) const VOLLEY_MAX_MS: u64 = 60_000;
const _: () = assert!(VOLLEY_MAX_MS > VOLLEY_SESSION_MS.1 + VOLLEY_POINT_MS);

// ── 단체 야차 ──────────────────────────────────────────────────
//
// 야차(=야차클럽 유래의 맞장 은어)의 다인전은 battle royal이다. **경기장을
// 그리지 않고 대형도 없다** — 각자 상대를 골라 사방으로 붙었다 빠진다.
//
// **값은 전부 아티팩트에서 왔다** (2026-09-03 사용자 지시). 옮겨 적은 명세는
// `docs/plans/2026-09-03-027-feat-yacha-brawl-plan.md`의 "난투 명세" 절에 있다 —
// 여기 값과 그쪽이 어긋나면 그쪽이 이긴다.

/// 판을 열 수 있는 최소 마릿수. **한 마리면 때릴 상대가 없다** (R3).
/// **홀수 제약은 없다** — 팀이 없는 난투라 세 마리도 정상이다.
pub(super) const YACHA_MIN_PETS: usize = 2;
const _: () = assert!(YACHA_MIN_PETS >= 2);

/// 시뮬레이션 한 걸음 (ms). **틱(50ms)과 다르다** — 한 틱에 두세 걸음을 돌린다.
/// 걸음을 잘게 두는 이유는 주먹 판정(진행률 42%)이 틱 간격보다 촘촘해서다.
pub(super) const YACHA_DT_MS: u64 = 20;
const _: () = assert!(YACHA_DT_MS < MAX_STEP_MS);

/// 다니는 가로 반폭. **닿는 거리와 따로다** — 판을 넓히면 쫓아다니는 그림이
/// 늘고 주먹은 그대로라 붙어야만 맞는다.
pub(super) const YACHA_ARENA_HALF: f64 = 408.0;

/// 다니는 세로 범위 (발밑 기준). 위로 갈수록 멀고 작다.
pub(super) const YACHA_ARENA_Y: (f64, f64) = (-214.0, 96.0);
const _: () = assert!(YACHA_ARENA_Y.0 < YACHA_ARENA_Y.1);

/// 주먹이 닿는 거리. **판을 넓혀도 이건 안 건드린다.**
pub(super) const YACHA_REACH: f64 = 118.0;

/// 이보다 가까우면 "붙었다"로 친다.
pub(super) const YACHA_NEAR: f64 = 84.0;
const _: () = assert!(YACHA_NEAR < YACHA_REACH);

/// 이보다 겹치지 않는다. **밀려나는 게 아니라 비켜서는 것**이다.
pub(super) const YACHA_SEP: f64 = 46.0;
const _: () = assert!(YACHA_SEP < YACHA_NEAR);

/// 세로 1px을 가로보다 얼마나 멀게 세나 (원근).
pub(super) const YACHA_YW: f64 = 1.35;
const _: () = assert!(YACHA_YW > 1.0);

/// 걸음 (논리 px/ms). `hunt` 기준이고 맴돌기·빼기는 계수가 다르다.
/// **판이 넓어진 만큼 걸음도 크다** — 안 그러면 쫓아만 다니다 판이 끝난다.
pub(super) const YACHA_STEP_PER_MS: f64 = 0.124;

/// 깊이 배율의 기울기 — 아래(가까이)에 있을수록 크다. `1 + yOff / 이 값`.
pub(super) const YACHA_DEPTH_SPAN: f64 = 700.0;
const _: () = assert!(YACHA_DEPTH_SPAN > YACHA_ARENA_Y.1 - YACHA_ARENA_Y.0);

/// 주먹 한 번의 길이. **고정이다** — v5는 150~240에서 뽑아 놓고 진행도를 220으로
/// 나눠서 자세와 판정이 어긋나 있었다.
pub(super) const YACHA_SWING_MS: u64 = 180;
/// 주먹의 **판정 지점** (진행률). 뻗는 도중에 닿는다.
pub(super) const YACHA_SWING_HIT_AT: f64 = 0.42;
const _: () = assert!(YACHA_SWING_HIT_AT > 0.0 && YACHA_SWING_HIT_AT < 1.0);

/// 맞고 휘청이는 길이. **짧다** — 빨리 복귀해야 연타가 안 끊긴다.
pub(super) const YACHA_HURT_MS: u64 = 140;
const _: () = assert!(YACHA_HURT_MS < YACHA_SWING_MS);

/// 난투 예산. 아티팩트의 `brawl` 국면 길이 그대로다.
pub(super) const YACHA_BRAWL_MS: u64 = 14_000;
/// 예산 안에 주먹이 충분히 들어가야 "치고받는다"로 보인다.
const _: () = assert!(YACHA_BRAWL_MS / YACHA_SWING_MS >= 40);

/// 상태 고르기 확률 — **닿을 때만 친다** (헛스윙은 화면에서 아무 일도 아니다).
/// 멀 때: hunt / 나머지는 circle.
pub(super) const YACHA_P_HUNT: f64 = 0.78;
/// 붙었을 때 칠 확률. **방금 쳤으면 더 친다 — 이게 퍽퍽퍽 연타다.**
pub(super) const YACHA_P_SWING: (f64, f64) = (0.52, 0.64);
const _: () = assert!(YACHA_P_SWING.0 < YACHA_P_SWING.1);
/// 칠 확률 위로 얹히는 몫 — 가드, 그다음 빼기. 나머지가 맴돌기다.
pub(super) const YACHA_P_GUARD_ADD: f64 = 0.15;
pub(super) const YACHA_P_BACK_ADD: f64 = 0.29;
const _: () = assert!(YACHA_P_GUARD_ADD < YACHA_P_BACK_ADD);
const _: () = assert!(YACHA_P_SWING.1 + YACHA_P_BACK_ADD < 1.0);
/// 상대를 다시 고를 확률.
pub(super) const YACHA_P_RETARGET: f64 = 0.32;

/// 상태 지속 — 멀 때 / 붙었을 때 (`lo`, `범위`).
pub(super) const YACHA_HOLD_FAR_MS: (u64, u64) = (200, 320);
pub(super) const YACHA_HOLD_NEAR_MS: (u64, u64) = (140, 280);

/// 이만큼은 맞아야 쓰러진다 — 한 대도 안 맞고 넘어가는 그림을 막는다.
pub(super) const YACHA_MIN_HITS: u32 = 2;
const _: () = assert!(YACHA_MIN_HITS >= 1);

/// 최후의 1인이 양 날개를 드는 시간. **`SASSY_MS`와 같아야 한다** — CSS가
/// 싸가지 반응의 keyframe을 참조하므로 어긋나면 자세가 중간에 멈춘다.
pub(super) const YACHA_WIN_MS: u64 = SASSY_MS;

/// 링으로 **날아오는** 속도. 볼링·발리볼과 같은 값·같은 이유다.
pub(super) const YACHA_GATHER_SPEED: f64 = 420.0;
const _: () = assert!(YACHA_GATHER_SPEED > SWIM_SPEED);
const _: () = assert!(YACHA_GATHER_SPEED < FREAKOUT_SPEED);

/// 모이는 데 주는 시간. 아티팩트의 `gather` 국면이다.
pub(super) const YACHA_GATHER_MS: u64 = 2_500;

/// 미녀 펭귄이 **걸어오는** 속도. 다른 마리들이 날아온 것과 대비되어야 한다.
pub(super) const YACHA_QUEEN_SPEED: f64 = 220.0;
const _: () = assert!(YACHA_QUEEN_SPEED > WALK_SPEED);
const _: () = assert!(YACHA_QUEEN_SPEED < YACHA_GATHER_SPEED);

/// 미녀가 챔피언 옆 어디에 서나 (몸통 가운데 사이).
pub(super) const YACHA_QUEEN_STOP_GAP: f64 = 110.0;
const _: () = assert!(YACHA_QUEEN_STOP_GAP < PET_SIZE);

/// 미녀가 걸어 들어오는 데 주는 시간.
pub(super) const YACHA_QUEEN_MS: u64 = 2_000;
/// 벨트를 채우는 동안.
pub(super) const YACHA_BELT_MS: u64 = 1_200;
/// 세레모니.
pub(super) const YACHA_CEREMONY_MS: u64 = 2_500;
/// 미녀가 나가고 쓰러진 놈들이 일어나는 동안.
pub(super) const YACHA_EXIT_MS: u64 = 1_200;

/// 판이 어떤 이유로도 마리를 이보다 오래 붙들지 못한다 (종료 증명 ③).
pub(super) const YACHA_MAX_MS: u64 = 90_000;
const _: () = assert!(
    YACHA_MAX_MS
        > YACHA_GATHER_MS
            + YACHA_BRAWL_MS
            + YACHA_WIN_MS
            + YACHA_QUEEN_MS
            + YACHA_BELT_MS
            + YACHA_CEREMONY_MS
            + YACHA_EXIT_MS
);

/// 이보다 좁은 세계에서는 판을 열지 않는다 — 붙었다 빠질 자리가 안 나온다.
pub(super) const YACHA_MIN_WORLD_WIDTH: f64 = 2.0 * (YACHA_REACH + PET_SIZE);
/// 세로로도 이만큼은 있어야 한다.
pub(super) const YACHA_MIN_WORLD_HEIGHT: f64 = 360.0;
const _: () = assert!(YACHA_MIN_WORLD_HEIGHT > PET_SIZE);
