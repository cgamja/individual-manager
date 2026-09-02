use crate::pet::test_support::*;
use crate::pet::*;

#[test]
fn 드래그_중에는_자율_이동이_멈추고_주어진_위치를_따른다() {
    let mut p = pet();
    p.drag_start(1_000);
    let before = p.snapshot();

    let s = p.step(2_000, &world());
    assert_eq!(s.x, before.x);
    assert_eq!(s.behavior, Behavior::Dragged);

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
fn 들어_올렸다_놓으면_여전히_떨어진다() {
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
