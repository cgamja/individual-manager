use crate::pet::test_support::*;
use crate::pet::*;

#[test]
fn 걷기_중에는_진행_방향으로_위치가_이동한다() {
    let mut p = pet();
    let before = p.snapshot();
    assert_eq!(before.behavior, Behavior::Walk);
    assert_eq!(before.facing, Facing::Right);

    let after = p.step(1_000, &world());
    assert!(after.x > before.x, "오른쪽을 보면 x가 커져야 한다");
}
#[test]
fn 왼쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
    let mut p = Pet::new(7, 0, &world());
    p.facing = Facing::Left;
    p.x = 5.0;

    let s = p.step(200, &world());
    assert_eq!(s.x, BOUNDS.left, "경계를 넘어가면 안 된다");
    assert_eq!(s.behavior, Behavior::Turn);
}
#[test]
fn 오른쪽_경계에_닿으면_방향을_전환하고_경계를_넘지_않는다() {
    let mut p = Pet::new(7, 0, &world());
    p.x = BOUNDS.right - 3.0;

    let s = p.step(200, &world());
    assert_eq!(s.x, BOUNDS.right);
    assert_eq!(s.behavior, Behavior::Turn);
}
#[test]
fn 방향_전환이_끝나면_반대_방향으로_걷는다() {
    let mut p = Pet::new(7, 0, &world());
    p.x = BOUNDS.right - 3.0;
    p.step(200, &world());
    assert_eq!(p.snapshot().facing, Facing::Right);

    let s = p.step(200 + TURN_MS + 10, &world());
    assert_eq!(s.facing, Facing::Left, "전환이 끝나면 방향이 뒤집힌다");
    assert_eq!(s.behavior, Behavior::Walk);
}
/// 오른쪽 벽에 붙여 놓고 한 틱 진행시켜 벽 반응 하나를 본다.
fn 벽_반응(seed: u64) -> Behavior {
    let mut p = Pet::new(seed, 0, &world());
    p.x = BOUNDS.right - 3.0;
    p.step(200, &world()).behavior
}
/// 벽에서 굴러떨어지는 시드 하나를 찾는다.
fn 굴러떨어지는_시드() -> u64 {
    (1u64..10_000)
        .find(|s| 벽_반응(*s) == Behavior::Tumble)
        .expect("굴러떨어지는 시드가 하나도 없다")
}
/// 오른쪽 벽에서 굴러떨어지기 시작한 펭귄. 여러 테스트가 같은 자세로 시작한다.
fn 굴러떨어지는_펭귄() -> Pet {
    let mut p = Pet::new(굴러떨어지는_시드(), 0, &world());
    p.x = BOUNDS.right - 3.0;
    p.step(200, &world());
    p
}
#[test]
fn 벽에_닿으면_굴러떨어지거나_돌아선다() {
    let 반응: Vec<Behavior> = (1u64..200).map(벽_반응).collect();
    assert!(
        반응.contains(&Behavior::Tumble),
        "굴러떨어지는 경우가 하나도 없다"
    );
    assert!(
        반응.contains(&Behavior::Turn),
        "돌아서는 경우가 하나도 없다 — 벽이 곧 넘어지는 곳이 됐다"
    );
    assert!(
        반응
            .iter()
            .all(|b| matches!(b, Behavior::Tumble | Behavior::Turn)),
        "벽 반응은 이 둘뿐이다"
    );
}
#[test]
fn 굴러떨어지기는_벽_반대_방향으로_이동한다() {
    let mut p = 굴러떨어지는_펭귄();
    let s = p.snapshot();
    assert_eq!(s.behavior, Behavior::Tumble);
    assert_eq!(s.facing, Facing::Left, "벽에서 멀어지는 쪽을 본다");

    let after = p.step(300, &world()).x;
    assert!(after < s.x, "오른쪽 벽에서 굴렀으면 x가 작아진다");
}
#[test]
fn 굴러떨어지는_동안_속도가_줄어든다() {
    let mut p = 굴러떨어지는_펭귄();
    let mut 이동량 = Vec::new();
    let mut prev = p.snapshot().x;
    let mut t = 250;
    while t < 200 + TUMBLE_MS {
        let x = p.step(t, &world()).x;
        이동량.push(prev - x);
        prev = x;
        t += 50;
    }
    assert!(
        이동량.first().unwrap() > 이동량.last().unwrap(),
        "뒤로 갈수록 덜 움직여야 구르다 멈추는 것으로 읽힌다: {이동량:?}"
    );
}
#[test]
fn 굴러떨어지기가_끝나면_멈춘다() {
    let mut p = 굴러떨어지는_펭귄();
    drive(&mut p, 250, 200 + TUMBLE_MS, 50, &world());
    let 끝난_뒤 = p.step(200 + TUMBLE_MS + 10, &world()).x;
    let 더_뒤 = p.step(200 + TUMBLE_MS + 200, &world()).x;
    assert_eq!(끝난_뒤, 더_뒤, "굴러떨어지기가 끝나면 더 움직이지 않는다");
}
#[test]
fn 굴러떨어지고_나면_방향이_뒤집혀_있다() {
    let mut p = Pet::new(굴러떨어지는_시드(), 0, &world());
    p.x = BOUNDS.right - 3.0;
    assert_eq!(p.snapshot().facing, Facing::Right);

    let s = p.step(200, &world());
    assert_eq!(s.facing, Facing::Left, "Turn을 탔을 때와 최종 결과가 같다");
}
#[test]
fn 굴러떨어지기_뒤에는_약을_올리거나_유휴로_간다() {
    let mut 나온_동작 = Vec::new();
    for seed in 1u64..300 {
        let mut p = Pet::new(seed, 0, &world());
        p.x = BOUNDS.right - 3.0;
        if p.step(200, &world()).behavior != Behavior::Tumble {
            continue;
        }
        나온_동작.push(p.step(200 + TUMBLE_MS + 10, &world()).behavior);
    }
    assert!(!나온_동작.is_empty(), "굴러떨어지는 시드가 하나도 없다");
    assert!(
        나온_동작
            .iter()
            .all(|b| matches!(b, Behavior::Sassy { .. } | Behavior::Idle { .. })),
        "{나온_동작:?}"
    );
}
#[test]
fn 굴러떨어지기는_지상_동작이다() {
    assert!(!Behavior::Tumble.is_airborne());
    assert!(
        !Behavior::Tumble.is_landing(),
        "바닥에 닿아서 생긴 게 아니다"
    );
    assert!(
        Behavior::Tumble.moves_window(),
        "제자리 애니메이션이 아니다"
    );

    let s = 굴러떨어지는_펭귄().snapshot();
    assert!(!s.air);
    assert_eq!(s.y, BOUNDS.floor_y, "바닥에 붙어 굴러간다");
}
#[test]
fn 굴러떨어지는_중에_클릭하면_방망이를_휘두른다() {
    let mut p = 굴러떨어지는_펭귄();
    p.whack(300, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing);
}
#[test]
fn 굴러떨어지는_중에_들어_올릴_수_있다() {
    let mut p = 굴러떨어지는_펭귄();
    p.drag_start(300);
    assert_eq!(p.behavior(), Behavior::Dragged);
}
#[test]
fn 걸을_폭이_없는_화면에서는_굴러떨어지지_않는다() {
    let narrow = World::single(Bounds {
        left: 10.0,
        right: 10.0,
        top: 0.0,
        floor_y: 50.0,
    });
    for seed in 1u64..200 {
        let mut p = Pet::new(seed, 0, &narrow);
        let seen: Vec<Behavior> = drive(&mut p, 100, 5_000, 100, &narrow)
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert!(!seen.contains(&Behavior::Tumble), "시드 {seed}");
    }
}
/// 걷기가 끝나는 순간의 갈래 하나를 본다. 걷기 시간이 다 되도록 몰아 놓고
/// 한 틱 더 진행시킨다.
fn 걷기_뒤(seed: u64) -> Behavior {
    let w = world();
    let mut p = Pet::new(seed, 0, &w);
    p.x = 400.0;
    let mut t = 50;
    while t < 20_000 {
        let b = p.step(t, &w).behavior;
        if b != Behavior::Walk {
            return b;
        }
        t += 50;
    }
    panic!("시드 {seed}: 걷기가 끝나지 않는다");
}
/// 미끄러지기 시작한 펭귄과 시작 시각.
fn 미끄러지는_펭귄() -> (Pet, u64) {
    let w = world();
    for seed in 1u64..500 {
        let mut p = Pet::new(seed, 0, &w);
        p.x = 400.0;
        let mut t = 50;
        while t < 20_000 {
            if p.step(t, &w).behavior == Behavior::Slide {
                return (p, t);
            }
            t += 50;
        }
    }
    panic!("미끄러지는 시드가 하나도 없다");
}
#[test]
fn 걷기가_끝나면_가끔_미끄러진다() {
    let 갈래: Vec<Behavior> = (1u64..120).map(걷기_뒤).collect();
    assert!(
        갈래.contains(&Behavior::Slide),
        "미끄러지는 경우가 하나도 없다"
    );
    assert!(
        갈래.iter().any(|b| matches!(b, Behavior::Idle { .. })),
        "쉬는 경우가 사라졌다 — 걷고 나면 늘 미끄러진다"
    );
}
#[test]
fn 유휴가_끝났을_때는_미끄러지지_않는다() {
    for seed in 1u64..300 {
        let mut p = Pet::new(seed, 0, &world());
        p.behavior = Behavior::Idle {
            idle: IdleKind::Shake,
        };
        p.pick_next(1_000, BOUNDS);
        assert_ne!(p.behavior, Behavior::Slide, "시드 {seed}");
    }
}
#[test]
fn 미끄러지는_동안_진행_방향으로_이동한다() {
    let (mut p, t) = 미끄러지는_펭귄();
    let 시작 = p.snapshot();
    let 뒤 = p.step(t + 200, &world());
    let 나아간_거리 = (뒤.x - 시작.x) * 시작.facing.sign();
    assert!(나아간_거리 > 0.0, "{나아간_거리}");
}
#[test]
fn 슬라이딩은_걷기보다_빠르다() {
    let (mut p, t) = 미끄러지는_펭귄();
    let 시작_x = p.snapshot().x;
    let facing = p.snapshot().facing;
    let 뒤 = p.step(t + 200, &world());
    let 미끄러진_거리 = (뒤.x - 시작_x).abs();
    let 걸었을_거리 = WALK_SPEED * 0.2;
    assert!(
        미끄러진_거리 > 걸었을_거리,
        "{미끄러진_거리} vs 걷기 {걸었을_거리}"
    );
    let _ = facing;
}
#[test]
fn 미끄러지는_동안_속도가_줄어든다() {
    let (mut p, t0) = 미끄러지는_펭귄();
    let mut 이동량 = Vec::new();
    let mut prev = p.snapshot().x;
    let sign = p.snapshot().facing.sign();
    let mut t = t0 + 50;
    while t < t0 + SLIDE_MS {
        let x = p.step(t, &world()).x;
        이동량.push((x - prev) * sign);
        prev = x;
        t += 50;
    }
    assert!(
        이동량.first().unwrap() > 이동량.last().unwrap(),
        "뒤로 갈수록 덜 움직여야 주르륵 멈추는 것으로 읽힌다: {이동량:?}"
    );
}
#[test]
fn 슬라이딩이_끝나면_멈춘다() {
    let (mut p, t0) = 미끄러지는_펭귄();
    drive(&mut p, t0 + 50, t0 + SLIDE_MS, 50, &world());
    let 끝난_뒤 = p.step(t0 + SLIDE_MS + 10, &world()).x;
    let 더_뒤 = p.step(t0 + SLIDE_MS + 60, &world()).x;
    assert!(
        (끝난_뒤 - 더_뒤).abs() < 0.001 || p.behavior() != Behavior::Slide,
        "끝났는데도 미끄러진다"
    );
}
#[test]
fn 미끄러진_거리는_매번_다르다() {
    let w = world();
    let mut 거리 = Vec::new();
    for seed in 1u64..400 {
        let mut p = Pet::new(seed, 0, &w);
        p.x = 400.0;
        let mut t = 50;
        while t < 20_000 {
            if p.step(t, &w).behavior == Behavior::Slide {
                let 시작 = p.snapshot().x;
                let sign = p.snapshot().facing.sign();
                drive(&mut p, t + 50, t + SLIDE_MS, 50, &w);
                거리.push(((p.snapshot().x - 시작) * sign * 10.0).round() as i64);
                break;
            }
            t += 50;
        }
        if 거리.len() >= 8 {
            break;
        }
    }
    assert!(거리.len() >= 3, "표본이 모자라다: {거리:?}");
    assert!(
        거리.iter().collect::<std::collections::HashSet<_>>().len() > 1,
        "거리가 늘 같다: {거리:?}"
    );
}
#[test]
fn 슬라이딩이_걷기보다_멀리_간다() {
    let 최소_거리 = SLIDE_SPEED.0 * (SLIDE_MS as f64 / 1000.0) / 2.0;
    let 걷기_최대 = WALK_SPEED * (WALK_MS.1 as f64 / 1000.0);
    assert!(최소_거리 > 걷기_최대, "{최소_거리} vs {걷기_최대}");
}
#[test]
fn 미끄러지다_벽에_닿으면_돌아서거나_굴러떨어진다() {
    let w = world();
    let (mut p, _) = 미끄러지는_펭귄();
    let 벽 = if p.snapshot().facing == Facing::Right {
        BOUNDS.right
    } else {
        BOUNDS.left
    };
    p.x = 벽 - p.snapshot().facing.sign() * 2.0;
    let s = p.step(p.last_step_ms + 100, &w);
    assert!(
        matches!(s.behavior, Behavior::Turn | Behavior::Tumble),
        "{:?}",
        s.behavior
    );
    assert!(s.x >= BOUNDS.left && s.x <= BOUNDS.right, "경계를 넘었다");
}
#[test]
fn 걸을_폭이_없는_화면에서는_미끄러지지_않는다() {
    let narrow = World::single(Bounds {
        left: 10.0,
        right: 10.0,
        top: 0.0,
        floor_y: 50.0,
    });
    for seed in 1u64..200 {
        let mut p = Pet::new(seed, 0, &narrow);
        let seen: Vec<Behavior> = drive(&mut p, 100, 20_000, 100, &narrow)
            .iter()
            .map(|s| s.behavior)
            .collect();
        assert!(!seen.contains(&Behavior::Slide), "시드 {seed}");
    }
}
#[test]
fn 슬라이딩은_지상_동작이다() {
    assert!(!Behavior::Slide.is_airborne());
    assert!(!Behavior::Slide.is_landing());
    assert!(Behavior::Slide.moves_window(), "창이 따라 움직여야 한다");
    let (p, _) = 미끄러지는_펭귄();
    assert!(!p.snapshot().air);
    assert_eq!(p.snapshot().y, BOUNDS.floor_y);
}
#[test]
fn 미끄러지는_중에_클릭하면_방망이를_휘두른다() {
    let (mut p, t) = 미끄러지는_펭귄();
    p.whack(t + 100, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing);
}
#[test]
fn 미끄러지는_중에_들어_올릴_수_있다() {
    let (mut p, t) = 미끄러지는_펭귄();
    p.drag_start(t + 100);
    assert_eq!(p.behavior(), Behavior::Dragged);
}

#[test]
fn 시키면_바로_미끄러진다() {
    let mut p = pet();
    p.x = 400.0;
    assert!(p.start_slide(1_000));
    assert_eq!(p.behavior(), Behavior::Slide);
    let 뒤 = p.step(1_200, &world());
    assert_ne!(뒤.x, 400.0, "시켰는데 제자리다");
}

#[test]
fn 공중이거나_들려_있으면_시켜도_미끄러지지_않는다() {
    let mut 헤엄 = pet();
    헤엄.air = true;
    assert!(!헤엄.start_slide(1_000));
    assert_ne!(헤엄.behavior(), Behavior::Slide);

    let mut 들림 = pet();
    들림.drag_start(1_000);
    assert!(!들림.start_slide(1_100));
    assert_eq!(들림.behavior(), Behavior::Dragged);
}

#[test]
fn 걸어다닐_폭이_없는_화면에서도_영원히_돌지_않는다() {
    let narrow = World::single(Bounds {
        left: 10.0,
        right: 10.0,
        top: 0.0,
        floor_y: 50.0,
    });
    let mut p = Pet::new(5, 0, &narrow);
    let seen = drive(&mut p, 100, 40_000, 250, &narrow);
    assert!(
        seen.iter().any(|s| !matches!(s.behavior, Behavior::Turn)),
        "회전 말고 다른 동작으로 넘어가야 한다"
    );
    assert!(seen.iter().all(|s| s.x == narrow.first().bounds.left));
}
