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
pub(super) const THROW_MAX_WORLDS_PER_SEC: f64 = 0.9;
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

// ── 핀볼 모드 ──────────────────────────────────────────────────

/// 모든 면이 범퍼일 때 남는 속도 비율.
pub(super) const PINBALL_DAMPING: f64 = 0.92;
/// 채로 후려칠 때의 속도 — 초당 세계를 몇 번 가로지르는가.
pub(super) const PINBALL_HIT_WORLDS_PER_SEC: f64 = 0.8;
const _: () = assert!(PINBALL_HIT_WORLDS_PER_SEC < THROW_MAX_WORLDS_PER_SEC);
const _: () = assert!(PINBALL_DAMPING < 1.0);
const _: () = assert!(PINBALL_DAMPING > BOUNCE_DAMPING);

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

/// 핀 사이 간격. **펭귄 폭보다 좁아 살짝 겹친다** — 벌려 놓으면 한 줄이 아니라
/// 그냥 흩어져 선 펭귄들로 보인다.
pub(super) const BOWLING_PIN_GAP: f64 = 96.0;
const _: () = assert!(BOWLING_PIN_GAP < PET_SIZE);

/// 오른쪽 끝에서 첫 핀까지 띄우는 거리. 공이 마지막 핀을 지나 빠져나갈 자리다.
pub(super) const BOWLING_PIN_MARGIN: f64 = 24.0;

/// 공 자리에서 가장 왼쪽 핀까지 **반드시** 남기는 길이. 여덟 마리가 좁은 화면에
/// 서면 핀 줄이 공까지 뻗는데, 그러면 굴리기 전에 이미 닿아 있다 (A5).
pub(super) const BOWLING_LANE_MIN: f64 = 240.0;

/// 핀 자리로 걸어가는 속도. **걷기보다 빠르다** — 평소 걷기(42px/s)로 가면
/// 화면을 가로지르는 데 30초가 걸려 판이 시작되기 전에 지친다.
pub(super) const BOWLING_GATHER_SPEED: f64 = 380.0;
const _: () = assert!(BOWLING_GATHER_SPEED > WALK_SPEED);
const _: () = assert!(BOWLING_GATHER_SPEED < FREAKOUT_SPEED);

/// 공중에 있던 마리가 판에 합류하며 내려오는 속도. 순간이동하면 R2를 어긴다.
pub(super) const BOWLING_DESCENT_SPEED: f64 = 300.0;

// 맞은 펭귄이 도는 **주기**는 여기 없다. 도는 것을 멈추는 것은 시간이 아니라
// 판이라 대응하는 국면 길이가 없고, 상수만 두면 Rust에서 아무도 안 쓴다.
// 반복 애니메이션의 주기는 CSS가 혼자 정한다 (`pg-bowling-spin`).

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

/// 공 중심이 펭귄 중심에서 이 거리 안에 들어오면 맞는다.
pub(super) const BOWLING_HIT_RADIUS: f64 = 52.0;

/// 펭귄 하나를 지나갈 때마다 잃는 속도 비율. **멈추지는 않는다** — 첫 펭귄에서
/// 멈추면 마릿수가 무의미해진다 (A2).
pub(super) const BOWLING_SPEED_LOSS_PER_PIN: f64 = 0.12;
const _: () = assert!(BOWLING_SPEED_LOSS_PER_PIN > 0.0);
const _: () = assert!(BOWLING_SPEED_LOSS_PER_PIN < 1.0);
