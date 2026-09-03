use crate::pet::test_support::*;
use crate::pet::*;

/// 시켜서 안물 중인 펭귄.
fn 안물하는_펭귄() -> (Pet, u64) {
    let mut p = pet();
    p.step(1_000, &world());
    assert!(p.start_dont_ask(1_000), "시키면 시작해야 한다");
    (p, 1_000)
}

#[test]
fn 버튼으로_시키면_안물_동작에_들어간다() {
    let (p, _) = 안물하는_펭귄();
    assert!(matches!(p.behavior, Behavior::DontAsk));
}

#[test]
fn 안물은_정해진_시간_뒤에_끝난다() {
    let (mut p, t0) = 안물하는_펭귄();
    let s = p.step(t0 + DONT_ASK_MS - 1, &world());
    assert!(
        matches!(s.behavior, Behavior::DontAsk),
        "1ms 전에는 아직 한다"
    );
    let s = p.step(t0 + DONT_ASK_MS, &world());
    assert!(!matches!(s.behavior, Behavior::DontAsk), "제 시간에 끝난다");
}

#[test]
fn 바닥에서_끝나면_유휴로_돌아간다() {
    let (mut p, t0) = 안물하는_펭귄();
    assert!(!p.air, "바닥에서 시작했다");
    let s = p.step(t0 + DONT_ASK_MS, &world());
    assert!(matches!(s.behavior, Behavior::Idle { .. }));
}

#[test]
fn 공중에서_끝나면_낙하로_떨어진다() {
    let mut p = pet();
    p.step(1_000, &world());
    p.enter(Behavior::Swim, 60_000);
    assert!(p.air, "헤엄은 공중이다");
    assert!(p.start_dont_ask(1_000));
    let s = p.step(1_000 + DONT_ASK_MS, &world());
    assert!(matches!(s.behavior, Behavior::Falling));
}

#[test]
fn 공중에서_시켜도_공중_상태가_유지된다() {
    // `air`가 꺼지면 끝나고 낙하로 안 가고 그 높이에 붙어 버린다.
    let mut p = pet();
    p.step(1_000, &world());
    p.enter(Behavior::Swim, 60_000);
    p.start_dont_ask(1_000);
    assert!(p.air, "안물은 그 자리에서 하는 동작이라 높이를 잃지 않는다");
}

#[test]
fn 이미_안물_중이면_거부한다() {
    let (mut p, t0) = 안물하는_펭귄();
    let 끝날_시각 = p.behavior_until_ms;
    assert!(!p.start_dont_ask(t0 + 500), "두 번째는 거부한다");
    assert_eq!(p.behavior_until_ms, 끝날_시각, "판이 연장되지 않는다");
}

#[test]
fn 들고_있으면_거부한다() {
    let mut p = pet();
    p.drag_start(1_000);
    assert!(matches!(p.behavior, Behavior::Dragged));
    assert!(!p.start_dont_ask(1_000));
}

#[test]
fn 빠따를_맞으면_안물이_끊긴다() {
    let (mut p, t0) = 안물하는_펭귄();
    클릭(&mut p, t0 + 500);
    assert!(!matches!(p.behavior, Behavior::DontAsk), "방망이가 이긴다");
}

#[test]
fn 저절로는_안물이_안_나온다() {
    // 버튼 전용이다 — `pick_next`에 끼면 뒤의 모든 빈도가 밀린다 (R9).
    for seed in 0..5u64 {
        let w = world();
        let mut p = Pet::new(seed, 0, &w);
        for s in drive(&mut p, 100, 4 * 60 * 60_000, 100, &w) {
            assert!(
                !matches!(s.behavior, Behavior::DontAsk),
                "시드 {seed}에서 저절로 나왔다"
            );
        }
    }
}
