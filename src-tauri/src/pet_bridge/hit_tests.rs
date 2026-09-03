use super::*;
use crate::pet::PET_SIZE;
use crate::pet_bridge::{PET_PAD_TOP, PET_PAD_X, PET_WINDOW_H, PET_WINDOW_W};

/// 펭귄이 놓인 자리. 값 자체는 아무래도 좋고 0이 아니기만 하면 된다 —
/// 0으로 두면 좌표를 더하는 걸 빼먹어도 통과한다.
const AT: (f64, f64) = (500.0, 300.0);

/// 그 펭귄의 창. 히트 상자가 여기 들어가는지 보는 데 쓴다.
fn window_rect((x, y): (f64, f64)) -> Rect {
    let l = x - PET_PAD_X;
    let t = y - PET_PAD_TOP;
    (l, t, l + PET_WINDOW_W, t + PET_WINDOW_H)
}

fn area((l, t, r, b): Rect) -> f64 {
    (r - l) * (b - t)
}

#[test]
fn 히트_박스는_창_안에_들어간다() {
    let (hl, ht, hr, hb) = hit_rect(AT.0, AT.1, 1.0);
    let (wl, wt, wr, wb) = window_rect(AT);
    assert!(hl >= wl, "왼쪽이 창 밖이다: {hl} < {wl}");
    assert!(ht >= wt, "위쪽이 창 밖이다: {ht} < {wt}");
    assert!(hr <= wr, "오른쪽이 창 밖이다: {hr} > {wr}");
    assert!(hb <= wb, "아래쪽이 창 밖이다: {hb} > {wb}");
}

#[test]
fn 히트_박스는_창_면적의_4분의_1보다_작다() {
    let ratio = area(hit_rect(AT.0, AT.1, 1.0)) / area(window_rect(AT));
    assert!(
        ratio < 0.25,
        "죽은 클릭 영역이 안 줄었다 — 창의 {:.1}%",
        ratio * 100.0
    );
}

#[test]
fn 펭귄_가운데는_히트_박스_안이다() {
    let (cx, cy) = (AT.0 + PET_SIZE / 2.0, AT.1 + PET_SIZE / 2.0);
    assert!(contains(hit_rect(AT.0, AT.1, 1.0), cx, cy));
}

#[test]
fn 방망이_여백은_히트_박스_밖이다() {
    // 창 왼쪽 끝에서 10px 안쪽 — 방망이가 휘둘러질 자리다.
    let x = AT.0 - PET_PAD_X + 10.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(!contains(hit_rect(AT.0, AT.1, 1.0), x, y));
    assert!(!contains(request_rect(AT.0, AT.1, 1.0), x, y));
}

#[test]
fn 말풍선_자리는_히트_박스_밖이다() {
    let x = AT.0 + PET_SIZE / 2.0;
    let y = AT.1 - PET_PAD_TOP + 10.0;
    assert!(!contains(hit_rect(AT.0, AT.1, 1.0), x, y));
    assert!(!contains(request_rect(AT.0, AT.1, 1.0), x, y));
}

#[test]
fn 무대_안이어도_실루엣_옆은_히트_박스_밖이다() {
    // 무대 왼쪽 끝 5px — `meet` 레터박스라 그림이 시작하지도 않은 자리다.
    let x = AT.0 + 5.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(
        !contains(hit_rect(AT.0, AT.1, 1.0), x, y),
        "무대 안이라고 다 펭귄은 아니다"
    );
}

#[test]
fn 요청_박스가_히트_박스를_포함한다() {
    let (hl, ht, hr, hb) = hit_rect(AT.0, AT.1, 1.0);
    let (rl, rt, rr, rb) = request_rect(AT.0, AT.1, 1.0);
    // **방향이 중요하다.** 웹뷰가 요청하는 조건(요청 박스 밖)이 Rust가 되돌리는
    // 조건(히트 박스 안)보다 바깥이어야 띠가 생긴다. 뒤집으면 그 사이에서
    // 요청과 되돌리기가 매 틱 번갈아 일어난다.
    assert!(rl < hl && rt < ht && rr > hr && rb > hb);
}

#[test]
fn 히스테리시스_띠_안은_어느_쪽도_아니다() {
    let (hl, _, _, _) = hit_rect(AT.0, AT.1, 1.0);
    let band = hl - PET_SIZE * PET_HIT_HYSTERESIS_RATIO / 2.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(
        !contains(hit_rect(AT.0, AT.1, 1.0), band, y),
        "되돌리지 않는다"
    );
    assert!(
        contains(request_rect(AT.0, AT.1, 1.0), band, y),
        "요청하지도 않는다"
    );
}

#[test]
fn 히트_박스는_펭귄을_따라_움직인다() {
    let (l0, t0, r0, b0) = hit_rect(0.0, 0.0, 1.0);
    let (l1, t1, r1, b1) = hit_rect(37.0, -19.0, 1.0);
    assert_eq!(
        (l1 - l0, t1 - t0, r1 - r0, b1 - b0),
        (37.0, -19.0, 37.0, -19.0)
    );
}

#[test]
fn 히트_박스는_펭귄_크기에_비례한다() {
    // 크기 % 조절이 `PET_SIZE`를 바꿔도 상자가 따라간다는 못이다. 픽셀 상수를
    // 하나라도 하드코딩하면 이 비례가 깨진다.
    let (l, t, r, b) = hit_rect(0.0, 0.0, 1.0);
    for (value, name) in [(l, "l"), (t, "t"), (r, "r"), (b, "b")] {
        let ratio = value / PET_SIZE;
        assert!(
            (0.0..=1.2).contains(&ratio),
            "{name}가 PET_SIZE 배수로 안 나온다: {ratio}"
        );
    }
    // 세로는 무대를 꽉 채운다 (viewBox의 긴 변이 세로라 `meet`이 세로를 맞춘다).
    assert!((b - t - PET_SIZE * (PET_HIT_B - PET_HIT_T) / PET_VIEWBOX_H).abs() < 1e-9);
}

#[test]
fn 반열린_구간이라_맞닿은_변은_밖이다() {
    let rect = (0.0, 0.0, 10.0, 10.0);
    assert!(contains(rect, 0.0, 0.0));
    assert!(!contains(rect, 10.0, 5.0));
    assert!(!contains(rect, 5.0, 10.0));
}

#[test]
fn 세_상자가_안에서_바깥_순서다() {
    // 히트 ⊂ 되돌리기 ⊂ 요청. 순서가 뒤집히면 경계에서 요청과 되돌리기가
    // 매 틱 번갈아 일어나거나, 그림에 닿고 나서야 되돌린다.
    let (hl, ht, hr, hb) = hit_rect(AT.0, AT.1, 1.0);
    let (vl, vt, vr, vb) = revert_rect(AT.0, AT.1, 1.0);
    let (ql, qt, qr, qb) = request_rect(AT.0, AT.1, 1.0);
    assert!(ql < vl && vl < hl);
    assert!(qt < vt && vt < ht);
    assert!(qr > vr && vr > hr);
    assert!(qb > vb && vb > hb);
}

// ── 통과 판정 ────────────────────────────────────────────────────

/// [`decide_click_through`]가 통과를 허락했나. 접는 이유(`Hold`/`Latch`)를
/// 가르는 검사는 아래 "거부의 갈래"에 따로 있다.
fn 통과(
    requested: bool,
    pose: Pose,
    pet: (f64, f64),
    cursor: Option<(f64, f64)>,
    anchor: Option<(f64, f64)>,
) -> bool {
    decide_click_through(requested, pose, pet, cursor, anchor, 1.0).through()
}

/// 되돌리기 상자 밖, 창 여백. 통과를 유지해야 하는 자리다.
fn 여백() -> (f64, f64) {
    (AT.0 - PET_PAD_X + 4.0, AT.1 + PET_SIZE / 2.0)
}

/// 펭귄 몸 위.
fn 몸() -> (f64, f64) {
    (AT.0 + PET_SIZE / 2.0, AT.1 + PET_SIZE / 2.0)
}

#[test]
fn 여백에_요청이_있고_커서가_그대로면_통과를_유지한다() {
    // 참으로 가는 유일한 길이다. 이게 깨지면 기능이 통째로 죽는다.
    assert!(통과(true, Pose::InBox, AT, Some(여백()), Some(여백())));
}

#[test]
fn 요청이_없으면_클릭을_먹는다() {
    assert!(!통과(false, Pose::InBox, AT, Some(여백()), None));
}

#[test]
fn 커서를_못_읽으면_클릭을_먹는다() {
    // 배율을 못 읽었을 때도 부르는 쪽이 `None`을 준다 — 같은 갈래다.
    assert!(!통과(true, Pose::InBox, AT, None, Some(여백())));
}

#[test]
fn 상자를_벗어나는_자세면_클릭을_먹는다() {
    // 들려 있기·슬라이딩·굴러떨어지기 따위. 그림이 상자 밖에 있는데 통과를
    // 걸면 그려진 펭귄을 눌렀는데 아래 앱이 받는다.
    assert!(!통과(
        true,
        Pose::OutOfBox,
        AT,
        Some(여백()),
        Some(여백())
    ));
}

#[test]
fn 커서가_그림_근처로_오면_닿기_전에_되돌린다() {
    // 그림에 닿은 뒤에 되돌리면 한 틱 + 세터 한 프레임 동안 클릭이 샌다.
    let (hl, _, _, _) = hit_rect(AT.0, AT.1, 1.0);
    let 코앞 = (
        hl - PET_SIZE * PET_HIT_ARM_RATIO / 2.0,
        AT.1 + PET_SIZE / 2.0,
    );
    assert!(
        !contains(hit_rect(AT.0, AT.1, 1.0), 코앞.0, 코앞.1),
        "아직 그림 밖이다"
    );
    assert!(!통과(true, Pose::InBox, AT, Some(코앞), Some(여백())));
}

#[test]
fn 커서가_펭귄_위면_되돌린다() {
    assert!(!통과(true, Pose::InBox, AT, Some(몸()), Some(여백())));
}

#[test]
fn 펭귄이_커서_밑으로_걸어오면_되돌린다() {
    // 커서는 가만히 있고 펭귄이 움직이는 경우다. 상자를 매 틱 다시 내지
    // 않으면 지나가는 동안 클릭이 아래 앱으로 샌다.
    let 커서 = 몸();
    let 지나간_뒤 = (AT.0 + 400.0, AT.1);
    assert!(통과(true, Pose::InBox, 지나간_뒤, Some(커서), Some(커서)));
    assert!(!통과(true, Pose::InBox, AT, Some(커서), Some(커서)));
}

#[test]
fn 요청_지점에서_한_마리_폭_넘게_움직이면_되돌린다() {
    let (ax, ay) = 여백();
    let 멀리 = (ax - PET_SIZE * PET_DRIFT_RATIO - 1.0, ay);
    assert!(!통과(true, Pose::InBox, AT, Some(멀리), Some((ax, ay))));
}

#[test]
fn 기준점_근처의_잔떨림에는_안_풀린다() {
    let (ax, ay) = 여백();
    let 조금 = (ax + 3.0, ay - 2.0);
    assert!(통과(true, Pose::InBox, AT, Some(조금), Some((ax, ay))));
}

// ── 거부의 갈래 ──────────────────────────────────────────────────

#[test]
fn 자세_때문에_접을_때는_요청을_남긴다() {
    // 지우면 슬라이딩이 끝난 뒤에도 요청이 없고, 웹뷰는 포인터가 움직일 때만
    // 그것도 스로틀에 걸려 재요청한다 — 커서가 멈춰 있으면 여백이 계속 클릭을
    // 먹는다(고치기 전 동작으로 퇴화).
    let v = decide_click_through(true, Pose::OutOfBox, AT, Some(여백()), None, 1.0);
    assert!(!v.through());
    assert!(!v.latches(), "일시적 사정인데 걸쇠가 걸렸다");
}

#[test]
fn 커서를_못_읽을_때도_요청을_남긴다() {
    let v = decide_click_through(true, Pose::InBox, AT, None, None, 1.0);
    assert!(!v.through());
    assert!(!v.latches());
}

#[test]
fn 위치_판정으로_되돌릴_때만_요청을_지운다() {
    // 이 둘은 배율에 기대는 판정이라, 다시 켜려면 웹뷰의 client 좌표를 거쳐야
    // 한다 — 걸쇠가 그걸 강제한다.
    let 몸_위 = decide_click_through(true, Pose::InBox, AT, Some(몸()), Some(여백()), 1.0);
    assert!(몸_위.latches(), "커서가 그림 근처인데 요청이 남는다");

    let (ax, ay) = 여백();
    let 멀리 = (ax - PET_SIZE * PET_DRIFT_RATIO - 1.0, ay);
    let 드리프트 = decide_click_through(true, Pose::InBox, AT, Some(멀리), Some((ax, ay)), 1.0);
    assert!(드리프트.latches(), "너무 멀어졌는데 요청이 남는다");
}

// ── 크기 배율 (플랜 026 U7) ────────────────────────────────────────────

/// 배율마다 도는 눈금. 슬라이더가 낼 수 있는 값의 양 끝과 가운데다.
const 배율들: [f64; 3] = [0.5, 1.0, 1.5];

/// 히트 상자는 **화면에 그려지는 크기**에 비례해야 한다. `PET_SIZE`를 그대로 쓰면
/// 60%에서 상자만 100% 자리에 남아 "몸통을 눌렀는데 반응이 없다"가 된다.
#[test]
fn 히트_상자가_그려진_크기에_비례한다() {
    let 기준 = hit_rect(AT.0, AT.1, 1.0);
    let (기준_w, 기준_h) = (기준.2 - 기준.0, 기준.3 - 기준.1);
    for s in 배율들 {
        let r = hit_rect(AT.0, AT.1, s);
        assert!(
            ((r.2 - r.0) - 기준_w * s).abs() < 1e-9,
            "{s} 배율에서 상자 폭이 안 따라왔다"
        );
        assert!(
            ((r.3 - r.1) - 기준_h * s).abs() < 1e-9,
            "{s} 배율에서 상자 높이가 안 따라왔다"
        );
    }
}

/// 코어 좌표(펭귄이 늘 `PET_SIZE`인 세계)가 화면 좌표로 옮겨져야 커서와 같은 자에
/// 놓인다. 안 옮기면 배율이 1이 아닐 때 상자가 통째로 엉뚱한 자리에 선다.
#[test]
fn 히트_상자가_코어_좌표를_화면으로_옮긴다() {
    for s in 배율들 {
        let r = hit_rect(AT.0, AT.1, s);
        let 창_왼쪽 = to_screen(AT.0 - PET_PAD_X, s);
        assert!(r.0 > 창_왼쪽, "{s}: 상자가 창 왼쪽 밖이다");
        // 상자 왼쪽은 무대 왼쪽(= 화면 좌표)에서 그림 여백만큼 안쪽이다.
        assert!(r.0 > to_screen(AT.0, s), "{s}: 상자가 무대보다 왼쪽이다");
        assert!(
            r.0 < to_screen(AT.0, s) + pet_render_px(s),
            "{s}: 무대를 넘었다"
        );
    }
}

/// 상자 셋의 포개짐(히트 ⊂ 되돌리기 ⊂ 요청)은 **어느 배율에서나** 성립해야 한다.
/// 여유를 절대 픽셀로 바꾸면 작은 배율에서 이 포개짐의 비율이 달라진다.
#[test]
fn 상자_셋의_포개짐이_배율을_안_탄다() {
    for s in 배율들 {
        let h = hit_rect(AT.0, AT.1, s);
        let rv = revert_rect(AT.0, AT.1, s);
        let rq = request_rect(AT.0, AT.1, s);
        assert!(
            rv.0 < h.0 && rv.1 < h.1 && rv.2 > h.2 && rv.3 > h.3,
            "{s}: 되돌리기가 안 감쌌다"
        );
        assert!(
            rq.0 < rv.0 && rq.1 < rv.1 && rq.2 > rv.2 && rq.3 > rv.3,
            "{s}: 요청이 안 감쌌다"
        );
        // 여유는 그려진 크기에 대한 **같은 비율**이어야 한다.
        let 여유 = h.0 - rv.0;
        assert!(
            (여유 - pet_render_px(s) * PET_HIT_ARM_RATIO).abs() < 1e-9,
            "{s}: 여유가 그려진 크기 비율이 아니다"
        );
    }
}

/// **여유를 비율로 두는 근거.** 상자를 좁히는 쪽(펭귄이 커서로 걸어오는 이동)도
/// 같은 배율로 줄므로 "여유 ÷ 한 틱 이동량"은 배율과 무관하다. 절대 픽셀로 바꾸면
/// 이 비가 배율마다 달라진다 — 그때 이 검사가 빨개진다.
#[test]
fn 여유가_한_틱_이동량에_대해_배율과_무관하다() {
    // 코어 기준 한 틱 이동량(걷기)은 배율과 무관하고, 화면 이동량은 배율에 비례한다.
    let 코어_한_틱 = 2.1; // WALK_SPEED(42 px/s) × TICK_MS(50ms)
    let 비 = |s: f64| (pet_render_px(s) * PET_HIT_ARM_RATIO) / to_screen(코어_한_틱, s);
    let 기준 = 비(1.0);
    for s in 배율들 {
        assert!(
            (비(s) - 기준).abs() < 1e-9,
            "{s}: 여유 대 이동량 비가 달라졌다"
        );
    }
    assert!(기준 > 3.0, "여유가 한 틱 이동량의 세 배도 안 된다: {기준}");
}

/// 드리프트 문도 그려진 크기를 따라야 한다 — 작아지면 **더 빨리** 걸린다.
/// 커서 속도는 배율을 안 타므로 이쪽이 조여지는 것이 맞는 방향이다.
#[test]
fn 드리프트_문이_작은_배율에서_더_빨리_걸린다() {
    let 여백_기준 = |s: f64| {
        (
            to_screen(AT.0 - PET_PAD_X + 4.0, s),
            to_screen(AT.1 + 70.0, s),
        )
    };
    let 멀어짐 = pet_render_px(1.0) * PET_DRIFT_RATIO * 0.8;
    // 100%에서는 아직 안 걸리는 거리가 50%에서는 걸린다.
    let (ax, ay) = 여백_기준(1.0);
    let v100 = decide_click_through(
        true,
        Pose::InBox,
        AT,
        Some((ax - 멀어짐, ay)),
        Some((ax, ay)),
        1.0,
    );
    let (bx, by) = 여백_기준(0.5);
    let v50 = decide_click_through(
        true,
        Pose::InBox,
        AT,
        Some((bx - 멀어짐, by)),
        Some((bx, by)),
        0.5,
    );
    assert!(!v100.latches(), "100%에서 벌써 걸렸다 — 표본이 잘못됐다");
    assert!(v50.latches(), "50%에서 드리프트가 안 걸렸다");
}
