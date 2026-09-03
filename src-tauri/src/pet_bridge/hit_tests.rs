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
    assert!(!contains(hit_rect(AT.0, AT.1, 1.0), band, y), "되돌리지 않는다");
    assert!(contains(request_rect(AT.0, AT.1, 1.0), band, y), "요청하지도 않는다");
}

#[test]
fn 히트_박스는_펭귄을_따라_움직인다() {
    let (l0, t0, r0, b0) = hit_rect(0.0, 0.0, 1.0);
    let (l1, t1, r1, b1) = hit_rect(37.0, -19.0, 1.0);
    assert_eq!((l1 - l0, t1 - t0, r1 - r0, b1 - b0), (37.0, -19.0, 37.0, -19.0));
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
    assert!(통과(
        true,
        Pose::InBox,
        AT,
        Some(여백()),
        Some(여백())
    ));
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
    let 코앞 = (hl - PET_SIZE * PET_HIT_ARM_RATIO / 2.0, AT.1 + PET_SIZE / 2.0);
    assert!(
        !contains(hit_rect(AT.0, AT.1, 1.0), 코앞.0, 코앞.1),
        "아직 그림 밖이다"
    );
    assert!(!통과(
        true,
        Pose::InBox,
        AT,
        Some(코앞),
        Some(여백())
    ));
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
    assert!(통과(
        true,
        Pose::InBox,
        지나간_뒤,
        Some(커서),
        Some(커서)
    ));
    assert!(!통과(true, Pose::InBox, AT, Some(커서), Some(커서)));
}

#[test]
fn 요청_지점에서_한_마리_폭_넘게_움직이면_되돌린다() {
    let (ax, ay) = 여백();
    let 멀리 = (ax - PET_SIZE * PET_DRIFT_RATIO - 1.0, ay);
    assert!(!통과(
        true,
        Pose::InBox,
        AT,
        Some(멀리),
        Some((ax, ay))
    ));
}

#[test]
fn 기준점_근처의_잔떨림에는_안_풀린다() {
    let (ax, ay) = 여백();
    let 조금 = (ax + 3.0, ay - 2.0);
    assert!(통과(
        true,
        Pose::InBox,
        AT,
        Some(조금),
        Some((ax, ay))
    ));
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
