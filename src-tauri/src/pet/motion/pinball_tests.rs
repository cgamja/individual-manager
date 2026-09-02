use crate::pet::test_support::*;
use crate::pet::*;

/// 핀볼 모드를 켠 펭귄. 켜는 것은 설정이지만 코어에서는 필드 하나다.
fn 핀볼_펫() -> Pet {
    let mut p = pet();
    p.set_pinball(true);
    p
}

/// 지정한 높이에서 떨어뜨리고, 바닥에 닿은 횟수와 마지막 동작을 센다.
fn 떨어뜨려_세기(p: &mut Pet, 높이: f64, 한계_ms: u64) -> (u32, Behavior) {
    let w = world();
    p.step(0, &w);
    p.y = BOUNDS.floor_y - 높이;
    p.vy = 0.0;
    p.enter(Behavior::Falling, 0);
    let (mut 바닥_접촉, mut 떠_있었나) = (0, true);
    let mut t = 0;
    while t < 한계_ms {
        t += 50;
        let s = p.step(t, &w);
        let 바닥에 = s.y >= BOUNDS.floor_y - 0.01;
        if 바닥에 && 떠_있었나 {
            바닥_접촉 += 1;
        }
        떠_있었나 = !바닥에;
    }
    (바닥_접촉, p.behavior())
}

#[test]
fn 핀볼이면_세게_떨어져도_널브러지지_않는다() {
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    p.y = BOUNDS.floor_y - 700.0;
    p.enter(Behavior::Falling, 0);
    let mut t = 0;
    while t < 30_000 {
        t += 50;
        let s = p.step(t, &w);
        assert!(
            !matches!(s.behavior, Behavior::Splat | Behavior::Sprawl),
            "핀볼인데 {:?}가 나왔다",
            s.behavior
        );
    }
}

#[test]
fn 핀볼을_끄면_착지_네_갈래가_그대로다() {
    let w = world();
    let mut p = pet();
    p.step(0, &w);
    p.y = BOUNDS.floor_y - 700.0;
    p.enter(Behavior::Falling, 0);
    let mut 널브러졌나 = false;
    let mut t = 0;
    while t < 30_000 {
        t += 50;
        if p.step(t, &w).behavior == Behavior::Sprawl {
            널브러졌나 = true;
        }
    }
    assert!(널브러졌나, "핀볼을 껐는데도 널브러지지 않았다");
}

#[test]
fn 핀볼이면_바닥에서_한참_튄다() {
    let (핀볼_접촉, _) = 떨어뜨려_세기(&mut 핀볼_펫(), 500.0, 60_000);
    let (평소_접촉, _) = 떨어뜨려_세기(&mut pet(), 500.0, 60_000);
    assert!(
        핀볼_접촉 >= 10,
        "핀볼인데 {핀볼_접촉}번밖에 안 튀었다"
    );
    assert!(
        핀볼_접촉 > 평소_접촉 * 2,
        "핀볼({핀볼_접촉})이 평소({평소_접촉})보다 확연히 오래 튀어야 한다"
    );
}

#[test]
fn 핀볼이라도_결국_선다() {
    let (_, 마지막) = 떨어뜨려_세기(&mut 핀볼_펫(), 500.0, 120_000);
    assert!(
        !matches!(마지막, Behavior::Falling | Behavior::Thrown),
        "2분이 지나도 안 멈춘다 ({마지막:?})"
    );
}

#[test]
fn 핀볼이면_벽에서도_거의_안_죽는다() {
    let w = world();
    let 남은_속도 = |핀볼: bool| {
        let mut p = pet();
        p.set_pinball(핀볼);
        p.step(0, &w);
        p.x = BOUNDS.right - 1.0;
        p.y = BOUNDS.floor_y - 400.0;
        p.vx = 600.0;
        p.vy = 0.0;
        p.enter(Behavior::Thrown, 0);
        let mut t = 0;
        while t < 3_000 && p.vx > 0.0 {
            t += 50;
            p.step(t, &w);
        }
        p.vx.abs()
    };
    let 핀볼 = 남은_속도(true);
    let 평소 = 남은_속도(false);
    assert!(
        핀볼 > 평소 * 1.5,
        "핀볼({핀볼:.0})이 평소({평소:.0})보다 훨씬 덜 죽어야 한다"
    );
}

/// 핀볼 펭귄을 한 지점에서 친다. 반환은 그 직후 스냅샷.
fn 쳐본다(nx: f64, ny: f64) -> (Pet, Snapshot) {
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    p.whack(1_000, &w, nx, ny);
    let s = p.snapshot();
    (p, s)
}

#[test]
fn 핀볼에서_아래를_치면_위로_날아간다() {
    let (p, s) = 쳐본다(0.0, 0.4);
    assert_eq!(s.behavior, Behavior::Thrown, "쳤으면 날아가야 한다");
    assert!(p.vy < 0.0, "아래를 쳤는데 위로 안 간다 (vy={})", p.vy);
}

#[test]
fn 핀볼에서_왼쪽을_치면_오른쪽으로_간다() {
    let (p, _) = 쳐본다(-0.4, 0.0);
    assert!(p.vx > 0.0, "왼쪽을 쳤는데 오른쪽으로 안 간다 (vx={})", p.vx);
    assert_eq!(p.snapshot().facing, Facing::Right, "가는 쪽을 봐야 한다");
}

#[test]
fn 핀볼에서_정중앙을_치면_위로_뜬다() {
    let (p, s) = 쳐본다(0.0, 0.0);
    assert_eq!(s.behavior, Behavior::Thrown);
    assert!(p.vy < 0.0, "정중앙을 쳤는데 안 뜬다 (vy={})", p.vy);
    assert!(p.vx.is_finite() && p.vy.is_finite(), "속도가 NaN이다");
}

#[test]
fn 핀볼에서_치는_세기는_세계_폭을_따른다() {
    let 세기 = |폭: f64| {
        let w = World::single(Bounds { left: 0.0, right: 폭, top: 0.0, floor_y: 800.0 });
        let mut p = Pet::new(42, 0, &w);
        p.set_pinball(true);
        p.step(0, &w);
        p.whack(1_000, &w, 0.0, 0.4);
        p.vy.abs()
    };
    assert!(세기(2_000.0) > 세기(500.0) * 2.0, "세계가 넓으면 더 세게 쳐야 한다");
}

#[test]
fn 핀볼에서는_방망이를_휘두르지_않는다() {
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    let 전 = p.snapshot().whack_seq;
    p.whack(1_000, &w, 0.0, 0.4);
    assert_eq!(p.snapshot().whack_seq, 전, "핀볼인데 스윙 횟수가 늘었다");
}

#[test]
fn 핀볼에서_스무_번_쳐도_빽빽대지_않는다() {
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    let mut t = 1_000;
    for _ in 0..(SQUAWK_WHACK_COUNT + 5) {
        p.whack(t, &w, 0.0, 0.4);
        assert_ne!(p.behavior(), Behavior::Squawk, "핀볼인데 빽빽댄다");
        t += 300;
    }
}

#[test]
fn 핀볼을_끄면_클릭이_빠따다() {
    let w = world();
    let mut p = pet();
    p.step(0, &w);
    p.whack(1_000, &w, -0.4, 0.4);
    assert_eq!(p.behavior(), Behavior::Swing, "빠따가 아니다");
    assert_eq!((p.vx, p.vy), (0.0, 0.0), "빠따는 날아가지 않는다");
    assert_eq!(p.snapshot().whack_seq, 1, "스윙 횟수가 안 늘었다");
}
