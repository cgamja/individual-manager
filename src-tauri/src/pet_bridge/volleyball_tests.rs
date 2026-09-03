use super::*;

fn 공(x: f64, y: f64, flying: bool) -> VolleyBallSnapshot {
    VolleyBallSnapshot { x, y, flying }
}

#[test]
fn 공_창_좌상단은_공_중심에서_반지름을_뺀다() {
    let (x, y) = vball_window_origin(500.0, 400.0);
    assert_eq!(x, 500.0 - VBALL_WINDOW_SIZE / 2.0);
    assert_eq!(y, 400.0 - VBALL_WINDOW_SIZE / 2.0);
}

#[test]
fn 공_창은_공보다_크지_않다() {
    // 창이 공보다 크면 투명한 테두리가 그만큼 클릭을 가린다 —
    // 지금은 통과시키지만, 크기를 맞춰 두면 그 방어에 기대지 않아도 된다.
    assert_eq!(VBALL_WINDOW_SIZE, crate::pet::VOLLEY_BALL_SIZE);
}

#[test]
fn 날아가는_동안만_겉모습이_바뀐다() {
    assert!(volley_look_of(&공(0.0, 0.0, true)));
    assert!(!volley_look_of(&공(0.0, 0.0, false)));
    // 위치는 겉모습에 안 들어간다 — 넣으면 날아가는 내내 20Hz로 리렌더한다.
    assert_eq!(
        volley_look_of(&공(0.0, 0.0, true)),
        volley_look_of(&공(999.0, 999.0, true))
    );
}

#[test]
fn 코트와_공의_라벨이_capabilities에_등록돼_있다() {
    // **없으면 이 창들의 `listen`이 컴파일·테스트를 다 통과하고 런타임에서만
    // 조용히 reject된다.** 이 대조가 그 조용한 실패를 막는 유일한 장치다.
    let caps = include_str!("../../capabilities/default.json");
    assert!(
        caps.contains(&format!("\"{COURT_LABEL}\"")),
        "capabilities에 {COURT_LABEL}이 없다"
    );
    assert!(
        caps.contains(&format!("\"{VBALL_LABEL}\"")),
        "capabilities에 {VBALL_LABEL}이 없다"
    );
}

#[test]
fn 코트는_펭귄보다_아래_레벨이다() {
    // 위에 두면 클릭 통과가 실패했을 때 펭귄이 통째로 안 만져진다.
    // 레벨과 클릭 통과는 서로를 대신하지 못하므로 **둘 다** 건다.
    #[cfg(target_os = "macos")]
    assert!(COURT_WINDOW_LEVEL < 3, "펭귄 레벨(3)보다 낮아야 한다");
}

#[test]
fn 판이_끝나는_순간에만_알린다() {
    // `bowling_over`를 그대로 쓴다 — "살아 있다가 사라졌다"의 판정이 같다.
    assert!(bowling_over(true, false));
    assert!(!bowling_over(false, false));
    assert!(!bowling_over(true, true));
    assert!(!bowling_over(false, true));
}

#[test]
fn 실패_상한이_있다() {
    // 재시도에 끝이 없으면 20Hz로 영원히 두드리면서 펭귄은 코트에 굳는다.
    assert!(VOLLEY_WINDOW_MAX_FAILS > 0);
    assert!(VOLLEY_WINDOW_MAX_FAILS <= 10);
}
