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
    let (hl, ht, hr, hb) = hit_rect(AT.0, AT.1);
    let (wl, wt, wr, wb) = window_rect(AT);
    assert!(hl >= wl, "왼쪽이 창 밖이다: {hl} < {wl}");
    assert!(ht >= wt, "위쪽이 창 밖이다: {ht} < {wt}");
    assert!(hr <= wr, "오른쪽이 창 밖이다: {hr} > {wr}");
    assert!(hb <= wb, "아래쪽이 창 밖이다: {hb} > {wb}");
}

#[test]
fn 히트_박스는_창_면적의_4분의_1보다_작다() {
    let ratio = area(hit_rect(AT.0, AT.1)) / area(window_rect(AT));
    assert!(
        ratio < 0.25,
        "죽은 클릭 영역이 안 줄었다 — 창의 {:.1}%",
        ratio * 100.0
    );
}

#[test]
fn 펭귄_가운데는_히트_박스_안이다() {
    let (cx, cy) = (AT.0 + PET_SIZE / 2.0, AT.1 + PET_SIZE / 2.0);
    assert!(contains(hit_rect(AT.0, AT.1), cx, cy));
}

#[test]
fn 방망이_여백은_히트_박스_밖이다() {
    // 창 왼쪽 끝에서 10px 안쪽 — 방망이가 휘둘러질 자리다.
    let x = AT.0 - PET_PAD_X + 10.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(!contains(hit_rect(AT.0, AT.1), x, y));
    assert!(!contains(request_rect(AT.0, AT.1), x, y));
}

#[test]
fn 말풍선_자리는_히트_박스_밖이다() {
    let x = AT.0 + PET_SIZE / 2.0;
    let y = AT.1 - PET_PAD_TOP + 10.0;
    assert!(!contains(hit_rect(AT.0, AT.1), x, y));
    assert!(!contains(request_rect(AT.0, AT.1), x, y));
}

#[test]
fn 무대_안이어도_실루엣_옆은_히트_박스_밖이다() {
    // 무대 왼쪽 끝 5px — `meet` 레터박스라 그림이 시작하지도 않은 자리다.
    let x = AT.0 + 5.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(
        !contains(hit_rect(AT.0, AT.1), x, y),
        "무대 안이라고 다 펭귄은 아니다"
    );
}

#[test]
fn 요청_박스가_히트_박스를_포함한다() {
    let (hl, ht, hr, hb) = hit_rect(AT.0, AT.1);
    let (rl, rt, rr, rb) = request_rect(AT.0, AT.1);
    // **방향이 중요하다.** 웹뷰가 요청하는 조건(요청 박스 밖)이 Rust가 되돌리는
    // 조건(히트 박스 안)보다 바깥이어야 띠가 생긴다. 뒤집으면 그 사이에서
    // 요청과 되돌리기가 매 틱 번갈아 일어난다.
    assert!(rl < hl && rt < ht && rr > hr && rb > hb);
}

#[test]
fn 히스테리시스_띠_안은_어느_쪽도_아니다() {
    let (hl, _, _, _) = hit_rect(AT.0, AT.1);
    let band = hl - PET_SIZE * PET_HIT_HYSTERESIS_RATIO / 2.0;
    let y = AT.1 + PET_SIZE / 2.0;
    assert!(!contains(hit_rect(AT.0, AT.1), band, y), "되돌리지 않는다");
    assert!(contains(request_rect(AT.0, AT.1), band, y), "요청하지도 않는다");
}

#[test]
fn 히트_박스는_펭귄을_따라_움직인다() {
    let (l0, t0, r0, b0) = hit_rect(0.0, 0.0);
    let (l1, t1, r1, b1) = hit_rect(37.0, -19.0);
    assert_eq!((l1 - l0, t1 - t0, r1 - r0, b1 - b0), (37.0, -19.0, 37.0, -19.0));
}

#[test]
fn 히트_박스는_펭귄_크기에_비례한다() {
    // 크기 % 조절이 `PET_SIZE`를 바꿔도 상자가 따라간다는 못이다. 픽셀 상수를
    // 하나라도 하드코딩하면 이 비례가 깨진다.
    let (l, t, r, b) = hit_rect(0.0, 0.0);
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
