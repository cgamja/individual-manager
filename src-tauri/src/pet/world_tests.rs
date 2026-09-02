use crate::pet::test_support::*;
use crate::pet::*;

#[test]
fn 빈_화면_목록으로는_세계를_만들_수_없다() {
    assert!(World::new(vec![]).is_none(), "펭귄이 있을 자리가 없다");
}

#[test]
fn 발밑이_속한_화면을_찾는다() {
    let w = 두_화면();
    let left = (500.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
    assert_eq!(w.screen_at(left.0, left.1).map(|s| s.id), Some(1));
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
    let near_left = (1_100.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
    assert_eq!(w.nearest(near_left.0, near_left.1).id, 1);
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
fn 화면_판정_범위는_기준점만큼_밀려_있다() {
    let w = world();
    assert!(w
        .screen_at(BOUNDS.left + PET_SIZE / 2.0, BOUNDS.floor_y + PET_SIZE)
        .is_some());
    assert!(w.screen_at(BOUNDS.left, BOUNDS.top).is_none());
}

#[test]
fn 정확히_같은_거리면_앞_화면이_이긴다() {
    let w = 두_화면();
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
