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
    assert!(핀볼_접촉 >= 10, "핀볼인데 {핀볼_접촉}번밖에 안 튀었다");
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
        let w = World::single(Bounds {
            left: 0.0,
            right: 폭,
            top: 0.0,
            floor_y: 800.0,
        });
        let mut p = Pet::new(42, 0, &w);
        p.set_pinball(true);
        p.step(0, &w);
        p.whack(1_000, &w, 0.0, 0.4);
        p.vy.abs()
    };
    assert!(
        세기(2_000.0) > 세기(500.0) * 2.0,
        "세계가 넓으면 더 세게 쳐야 한다"
    );
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

// ── 마리끼리 부딪히기 ──
//
// 판정은 `Pet::bump_of`(쌍 하나의 물리)와 `Pets::collide_pinball`(전 마리를 훑는
// 루프) 둘로 나뉜다. 앞의 것은 직접 부딪혀 보고, 뒤의 것은 `step_all`을 돌려서 본다.

/// 핀볼을 켜고 **중심**을 지정한 자리에 세운 펭귄. `Pet`의 `x`/`y`가 왼쪽 위
/// 모서리라 거리 계산과 헷갈리지 않게 중심으로 놓는다.
fn 중심에(cx: f64, cy: f64, vx: f64, vy: f64) -> Pet {
    let mut p = 핀볼_펫();
    p.step(0, &world());
    p.x = cx - PET_SIZE / 2.0;
    p.y = cy - PET_SIZE / 2.0;
    p.vx = vx;
    p.vy = vy;
    p.enter(Behavior::Thrown, 0);
    p
}

fn 중심(p: &Pet) -> (f64, f64) {
    (p.center_x(), p.center_y())
}

fn 속력(p: &Pet) -> f64 {
    (p.vx * p.vx + p.vy * p.vy).sqrt()
}

/// 한 쌍을 실제로 부딪혀 본다. 부딪혔으면 `true`.
fn 부딪힌다(a: &mut Pet, a전: (f64, f64), b: &mut Pet, b전: (f64, f64)) -> bool {
    let Some(((nx, ny), j)) = a.bump_of(a전, b, b전) else {
        return false;
    };
    a.bumped(1_000, nx, ny, j, BOUNDS.right);
    b.bumped(1_000, -nx, -ny, j, BOUNDS.right);
    true
}

/// 직전 위치가 지금 위치와 같은 — 즉 **제자리에 겹쳐 있는** 쌍을 부딪혀 본다.
/// 스침 판정(지나온 경로)이 아니라 임펄스만 보는 테스트들이 쓴다.
fn 제자리에서_부딪힌다(a: &mut Pet, b: &mut Pet) -> bool {
    let (a전, b전) = (중심(a), 중심(b));
    부딪힌다(a, a전, b, b전)
}

/// 제자리에서 마주 본 채 겹친 두 마리 — 쌍 테스트 대부분이 이 배치를 쓴다.
/// 간격 80은 부딪히는 반경(104)보다 좁다.
fn 마주_본_쌍(a속도: f64, b속도: f64) -> (Pet, Pet) {
    (
        중심에(400.0, 400.0, a속도, 0.0),
        중심에(480.0, 400.0, b속도, 0.0),
    )
}

#[test]
fn 마주_보고_날아오면_양쪽_다_튕겨_나간다() {
    let (mut a, mut b) = 마주_본_쌍(300.0, -300.0);
    assert!(제자리에서_부딪힌다(&mut a, &mut b), "안 부딪혔다");
    assert!(a.vx < 0.0, "오른쪽으로 가던 쪽이 안 돌아섰다 (vx={})", a.vx);
    assert!(b.vx > 0.0, "왼쪽으로 가던 쪽이 안 돌아섰다 (vx={})", b.vx);
}

#[test]
fn 한쪽만_날아와도_맞은_쪽이_밀려난다() {
    let (mut a, mut b) = 마주_본_쌍(600.0, 0.0);
    assert!(제자리에서_부딪힌다(&mut a, &mut b), "안 부딪혔다");
    assert!(b.vx > 0.0, "서 있던 쪽이 안 밀렸다 (vx={})", b.vx);
    assert_eq!(b.behavior(), Behavior::Thrown, "맞았으면 날아가야 한다");
    assert!(a.vx < 600.0, "친 쪽도 속도를 잃어야 한다 (vx={})", a.vx);
    assert_eq!(a.behavior(), Behavior::Thrown);
}

#[test]
fn 부딪혀도_속도의_합이_커지지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(500.0, -320.0);
    let 전 = 속력(&a) + 속력(&b);
    assert!(제자리에서_부딪힌다(&mut a, &mut b));
    let 후 = 속력(&a) + 속력(&b);
    assert!(
        후 <= 전 + 1e-9,
        "반발 계수가 1을 넘는다 — 여덟 마리가 뒤엉키면 영영 안 멎는다 ({전} → {후})"
    );
}

#[test]
fn 멀어지는_중이면_부딪히지_않는다() {
    // 반경 안에 있지만 서로 등지고 간다.
    let (mut a, mut b) = 마주_본_쌍(-300.0, 300.0);
    assert!(
        !제자리에서_부딪힌다(&mut a, &mut b),
        "멀어지는 쌍을 또 튕기면 겹친 자리에서 잔진동한다"
    );
}

#[test]
fn 빠르게_지나가도_이웃을_뛰어넘지_않는다() {
    // 한 틱에 800px을 날아 이웃을 관통한다. 시작도 끝도 반경 밖이다.
    let a전 = (100.0, 400.0);
    let mut a = 중심에(900.0, 400.0, 4_000.0, 0.0);
    let mut b = 중심에(500.0, 400.0, 0.0, 0.0);
    let b전 = 중심(&b);
    assert!(
        (a전.0 - b전.0).abs() > PINBALL_COLLIDE_RADIUS
            && (중심(&a).0 - b전.0).abs() > PINBALL_COLLIDE_RADIUS,
        "이 테스트는 시작도 끝도 반경 밖이어야 의미가 있다"
    );
    assert!(
        부딪힌다(&mut a, a전, &mut b, b전),
        "지금 위치만 보면 스쳐 지나가면서도 어느 틱에도 안 잡힌다"
    );
    assert!(b.vx > 0.0, "가던 쪽으로 밀어야 한다 (vx={})", b.vx);
}

#[test]
fn 바닥에_선_마리는_맞으면_뜬다() {
    let mut a = 중심에(400.0, BOUNDS.floor_y, 600.0, 0.0);
    let mut b = 중심에(480.0, BOUNDS.floor_y, 0.0, 0.0);
    // 서 있는 상태를 만든다 — 공중이 아니어야 띄우기가 걸린다.
    b.enter(Behavior::Walk, 100_000);
    assert!(제자리에서_부딪힌다(&mut a, &mut b));
    assert!(
        b.vy < 0.0,
        "수평으로만 밀리면 다음 틱에 곧바로 착지해 맞은 것이 안 보인다 (vy={})",
        b.vy
    );
}

#[test]
fn 핀볼이_꺼진_쌍은_부딪히지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(300.0, -300.0);
    a.set_pinball(false);
    b.set_pinball(false);
    assert!(
        !제자리에서_부딪힌다(&mut a, &mut b),
        "핀볼을 껐는데 부딪혔다"
    );
}

#[test]
fn 들려_있는_마리는_부딪히지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(600.0, 0.0);
    b.drag_start(0);
    assert!(
        !제자리에서_부딪힌다(&mut a, &mut b),
        "사용자가 들고 있는 마리를 밀어낼 수는 없다"
    );
}

#[test]
fn 정확히_겹쳐도_속도가_망가지지_않는다() {
    let mut a = 중심에(400.0, 400.0, 600.0, 0.0);
    let mut b = 중심에(400.0, 400.0, 0.0, 0.0);
    assert!(제자리에서_부딪힌다(&mut a, &mut b));
    for p in [&a, &b] {
        assert!(p.vx.is_finite() && p.vy.is_finite(), "속도가 NaN이다");
    }
    assert!(b.vx > 0.0, "다가온 방향으로 밀어야 한다 (vx={})", b.vx);
}

#[test]
fn 제자리에_겹쳐_선_둘은_밀리지_않는다() {
    let mut a = 중심에(400.0, 400.0, 0.0, 0.0);
    let mut b = 중심에(400.0, 400.0, 0.0, 0.0);
    assert!(
        !제자리에서_부딪힌다(&mut a, &mut b),
        "겹쳐 서는 것은 평소에도 일어난다 — 밀 이유가 없다"
    );
}

#[test]
fn 부딪혀도_난수를_쓰지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(500.0, -500.0);
    let (a난수, b난수) = (a.rng, b.rng);
    assert!(제자리에서_부딪힌다(&mut a, &mut b));
    assert_eq!(
        (a.rng, b.rng),
        (a난수, b난수),
        "판정이 난수를 쓰면 골든 수열을 재기준화해야 한다"
    );
}

// ── 전 마리 판정 (`Pets::step_all`) ──

/// 핀볼을 켠 두 마리를 만든다. 첫째는 살짝 떠서 오른쪽으로 날아가고, 둘째는 그
/// 경로 위 바닥에 서 있다. 반환은 `(판, 나는 마리, 선 마리)`.
fn 날아가는_한_마리와_선_한_마리(핀볼: bool) -> (Pets, PetId, PetId) {
    let w = world();
    let mut pets = Pets::new();
    let 나는 = pets.add(7, 0, &w, 100.0).unwrap();
    let 선 = pets.add(7, 0, &w, 400.0).unwrap();
    for id in [나는, 선] {
        let p = pets.get_mut(id).unwrap();
        p.set_pinball(핀볼);
        // 걷기를 길게 걸어 이 창에서는 다음 동작 추첨이 돌지 않게 한다.
        p.enter(Behavior::Walk, 100_000);
    }
    let p = pets.get_mut(나는).unwrap();
    p.y = BOUNDS.floor_y - 60.0;
    p.vx = 2_000.0;
    p.vy = 0.0;
    p.enter(Behavior::Thrown, 0);
    (pets, 나는, 선)
}

#[test]
fn 핀볼에서_날아온_마리가_서_있는_마리를_날린다() {
    let w = world();
    let (mut pets, _, 선) = 날아가는_한_마리와_선_한_마리(true);
    let mut t = 0;
    while t < 1_000 && pets.get(선).unwrap().behavior() != Behavior::Thrown {
        t += 50;
        pets.step_all(t, |_| Some(&w));
    }
    let 맞은 = pets.get(선).unwrap();
    assert_eq!(
        맞은.behavior(),
        Behavior::Thrown,
        "서 있던 마리가 안 날아갔다"
    );
    assert!(맞은.vx > 0.0, "가던 쪽으로 밀려야 한다 (vx={})", 맞은.vx);
}

#[test]
fn 핀볼을_끄면_그냥_통과한다() {
    let w = world();
    let (mut pets, _, 선) = 날아가는_한_마리와_선_한_마리(false);
    let mut t = 0;
    while t < 1_000 {
        t += 50;
        pets.step_all(t, |_| Some(&w));
        assert_ne!(
            pets.get(선).unwrap().behavior(),
            Behavior::Thrown,
            "핀볼을 껐는데 부딪혔다 (t={t}ms)"
        );
    }
}

#[test]
fn 걷는_두_마리는_서로를_밀지_않는다() {
    let w = world();
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &w, 400.0).unwrap();
    let b = pets.add(7, 0, &w, 400.0).unwrap();
    for id in [a, b] {
        let p = pets.get_mut(id).unwrap();
        p.set_pinball(true);
        p.enter(Behavior::Walk, 100_000);
    }
    let mut t = 0;
    while t < 3_000 {
        t += 50;
        for (id, snapshot) in pets.step_all(t, |_| Some(&w)) {
            assert_ne!(
                snapshot.behavior,
                Behavior::Thrown,
                "겹쳐 걷는 마리가 서로를 밀었다 (id={id}, t={t}ms)"
            );
        }
    }
}

#[test]
fn 여덟_마리가_뒤엉켜도_결국_멎는다() {
    let w = world();
    let mut pets = Pets::new();
    for i in 0..MAX_PETS {
        let id = pets.add(7, 0, &w, 100.0 + i as f64 * 90.0).unwrap();
        let p = pets.get_mut(id).unwrap();
        p.set_pinball(true);
        p.y = BOUNDS.floor_y - 300.0;
        p.vx = if i % 2 == 0 { 1_200.0 } else { -1_200.0 };
        p.vy = 400.0;
        p.enter(Behavior::Thrown, 0);
    }
    let mut t = 0;
    while t < 120_000 {
        t += 50;
        for (id, s) in pets.step_all(t, |_| Some(&w)) {
            assert!(
                s.x.is_finite() && s.y.is_finite(),
                "좌표가 NaN이다 (id={id}, t={t}ms)"
            );
            assert!(
                s.x >= BOUNDS.left - 0.5 && s.x <= BOUNDS.right + 0.5,
                "세계 밖으로 나갔다 (id={id}, x={}, t={t}ms)",
                s.x
            );
        }
    }
    for id in pets.ids() {
        let p = pets.get(id).unwrap();
        assert!(
            속력(p) < 4_000.0,
            "부딪힘이 에너지를 불렸다 (id={id}, 속력={})",
            속력(p)
        );
    }
}
