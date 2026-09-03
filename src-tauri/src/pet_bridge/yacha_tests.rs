//! 야차 브릿지의 검사 — 좌표 환산과 라벨 등록.

use super::*;

#[test]
fn 미녀_창_원점이_펭귄_창_규칙을_따른다() {
    // 같은 그림을 같은 여백으로 그리므로 자리 계산도 같아야 한다.
    for scale in [0.5, 1.0, 1.5] {
        assert_eq!(
            queen_window_origin(300.0, 200.0, scale),
            window_origin(300.0, 200.0, scale)
        );
    }
}

#[test]
fn 미녀_창_원점이_배율을_탄다() {
    let 하나 = queen_window_origin(300.0, 200.0, 1.0);
    let 반 = queen_window_origin(300.0, 200.0, 0.5);
    assert!(반.0 < 하나.0 && 반.1 < 하나.1, "배율이 안 걸렸다");
}

#[test]
fn 미녀_라벨이_capabilities에_등록돼_있다() {
    // 빠뜨리면 컴파일·테스트·경고가 전부 통과하고 **런타임에서만 조용히
    // reject된다** — 미녀 창의 `listen`이 아무것도 못 받는다.
    let json = include_str!("../../capabilities/default.json");
    assert!(
        json.contains(QUEEN_LABEL),
        "capabilities에 `{QUEEN_LABEL}`이 없다"
    );
}

#[test]
fn 창_생성_실패에_상한이_있다() {
    assert!(YACHA_WINDOW_MAX_FAILS > 0);
    assert!(YACHA_WINDOW_MAX_FAILS <= 10, "너무 오래 두드린다");
}
