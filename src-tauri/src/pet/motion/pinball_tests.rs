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

/// 틱 하나의 길이(초) — 브릿지의 `TICK_MS`와 같다.
const 틱: f64 = 0.05;

/// 틱 시작 중심 하나로 자취를 만든다.
fn 자취(from: (f64, f64)) -> Sweep {
    Sweep { from, seconds: 틱 }
}

/// 한 쌍을 실제로 부딪혀 본다. 자취(`a전`/`b전`)는 **틱 시작의 중심**이다.
/// 부딪혔으면 `true`.
fn 부딪힌다(a: &mut Pet, a전: (f64, f64), b: &mut Pet, b전: (f64, f64)) -> bool {
    let Some(((nx, ny), j)) = a.bump_of(자취(a전), b, 자취(b전)) else {
        return false;
    };
    a.bumped(1_000, nx, ny, j, BOUNDS.right);
    b.bumped(1_000, -nx, -ny, j, BOUNDS.right);
    true
}

/// 각자 자기 속도로 한 틱을 날아와 지금 자리에 있는 쌍을 부딪혀 본다.
/// **판정은 자취로 속도를 재므로** 시작 자리를 속도에서 거꾸로 만든다 — 제자리에
/// 세워 두고 속도 필드만 채우면 실제 틱에서는 나올 수 없는 상태가 된다.
fn 날아와서_부딪힌다(a: &mut Pet, b: &mut Pet) -> bool {
    let a전 = (중심(a).0 - a.vx * 틱, 중심(a).1 - a.vy * 틱);
    let b전 = (중심(b).0 - b.vx * 틱, 중심(b).1 - b.vy * 틱);
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
    assert!(날아와서_부딪힌다(&mut a, &mut b), "안 부딪혔다");
    assert!(a.vx < 0.0, "오른쪽으로 가던 쪽이 안 돌아섰다 (vx={})", a.vx);
    assert!(b.vx > 0.0, "왼쪽으로 가던 쪽이 안 돌아섰다 (vx={})", b.vx);
}

#[test]
fn 한쪽만_날아와도_맞은_쪽이_밀려난다() {
    let (mut a, mut b) = 마주_본_쌍(600.0, 0.0);
    assert!(날아와서_부딪힌다(&mut a, &mut b), "안 부딪혔다");
    assert!(b.vx > 0.0, "서 있던 쪽이 안 밀렸다 (vx={})", b.vx);
    assert_eq!(b.behavior(), Behavior::Thrown, "맞았으면 날아가야 한다");
    assert!(a.vx < 600.0, "친 쪽도 속도를 잃어야 한다 (vx={})", a.vx);
    assert_eq!(a.behavior(), Behavior::Thrown);
}

#[test]
fn 부딪혀도_속도의_합이_커지지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(500.0, -320.0);
    let 전 = 속력(&a) + 속력(&b);
    assert!(날아와서_부딪힌다(&mut a, &mut b));
    let 후 = 속력(&a) + 속력(&b);
    assert!(
        후 <= 전 + 1e-9,
        "반발 계수가 1을 넘는다 — 여덟 마리가 뒤엉키면 영영 안 멎는다 ({전} → {후})"
    );
}

/// 위 테스트는 **공중** 쌍이라 바닥 띄우기 분기를 한 번도 안 지난다. 띄우기가 세로를
/// 그냥 얹으면 속력이 √(1+비율²)배로 늘어 바닥 높이 충돌에서만 반발 계수가 1을 넘는데,
/// 공중 쌍만 보면 그게 안 보인다.
#[test]
fn 바닥에_선_마리를_쳐도_속도의_합이_커지지_않는다() {
    let mut a = 중심에(400.0, BOUNDS.floor_y, 600.0, 0.0);
    let mut b = 중심에(480.0, BOUNDS.floor_y, 0.0, 0.0);
    for p in [&mut a, &mut b] {
        p.enter(Behavior::Walk, 100_000);
    }
    let 전 = 속력(&a) + 속력(&b);
    assert!(날아와서_부딪힌다(&mut a, &mut b));
    let 후 = 속력(&a) + 속력(&b);
    assert!(
        후 <= 전 + 1e-9,
        "바닥 띄우기가 속력을 불렸다 — 방향만 틀어야 한다 ({전} → {후})"
    );
}

#[test]
fn 멀어지는_중이면_부딪히지_않는다() {
    // 반경 안에 있지만 서로 등지고 간다.
    let (mut a, mut b) = 마주_본_쌍(-300.0, 300.0);
    assert!(
        !날아와서_부딪힌다(&mut a, &mut b),
        "멀어지는 쌍을 또 튕기면 겹친 자리에서 잔진동한다"
    );
}

/// 한 틱에 800px을 날아 이웃을 관통한다. 시작도 끝도 반경 밖이다.
/// `세로차`는 두 마리 중심의 세로 간격 — **0이 아닌 값이 진짜 시험이다.**
fn 관통시킨다(세로차: f64) -> (Pet, Pet, bool) {
    let a전 = (100.0, 400.0);
    let mut a = 중심에(900.0, 400.0, 4_000.0, 0.0);
    let mut b = 중심에(500.0, 400.0 + 세로차, 0.0, 0.0);
    let b전 = 중심(&b);
    assert!(
        (a전.0 - b전.0).abs() > PINBALL_COLLIDE_RADIUS
            && (중심(&a).0 - b전.0).abs() > PINBALL_COLLIDE_RADIUS,
        "이 테스트는 시작도 끝도 반경 밖이어야 의미가 있다"
    );
    let 부딪혔나 = 부딪힌다(&mut a, a전, &mut b, b전);
    (a, b, 부딪혔나)
}

#[test]
fn 빠르게_지나가도_이웃을_뛰어넘지_않는다() {
    let (_, b, 부딪혔나) = 관통시킨다(0.0);
    assert!(
        부딪혔나,
        "지금 위치만 보면 스쳐 지나가면서도 어느 틱에도 안 잡힌다"
    );
    assert!(b.vx > 0.0, "가던 쪽으로 밀어야 한다 (vx={})", b.vx);
}

/// **정면(세로차 0)은 퇴화 경로다** — 상대 위치가 원점을 지나 방향 대체 분기로 빠진다.
/// 비껴 지나갈 때가 진짜 스침이고, 거기서 법선을 최근접점으로 잡으면 그 방향이 상대
/// 변위와 직교해 **접근 판정이 스침을 전부 기각한다.** 증상은 "부딪혔는데 아무 일도
/// 안 일어난다"뿐이라 정면 케이스만 보면 판정이 죽은 줄 모른다.
#[test]
fn 비껴_지나가도_제대로_된_세기로_부딪힌다() {
    let 정면_세기 = {
        let (_, b, 부딪혔나) = 관통시킨다(0.0);
        assert!(부딪혔나);
        속력(&b)
    };
    for 세로차 in [1.0, 20.0, 60.0, 95.0] {
        let (_, b, 부딪혔나) = 관통시킨다(세로차);
        assert!(부딪혔나, "세로차 {세로차}px에서 스침을 놓쳤다");
        assert!(
            속력(&b) > 정면_세기 * 0.3,
            "세로차 {세로차}px에서 세기가 {:.0}으로 무너졌다 (정면은 {정면_세기:.0}) \
             — 법선이 상대 변위와 직교해 접근 성분이 0이 된 것이다",
            속력(&b)
        );
    }
}

/// 반경은 **창이 아니라 몸통**이다. 창(한 변 `PET_SIZE`)은 겹치는데 그림은 안 닿는
/// 거리가 있고, 거기서 튕기면 허공에서 부딪히는 것으로 보인다. `PINBALL_COLLIDE_RADIUS`
/// 자체를 쓰지 않고 절대 거리로 못 박아야 상수를 키웠을 때 이 테스트가 잡는다.
#[test]
fn 창은_겹쳐도_몸통이_안_닿으면_부딪히지_않는다() {
    let 간격 = PET_SIZE * 0.9;
    assert!(간격 < PET_SIZE, "창이 겹치는 거리여야 의미가 있다");
    let mut a = 중심에(400.0, 400.0, 600.0, 0.0);
    let mut b = 중심에(400.0 + 간격, 400.0, 0.0, 0.0);
    assert!(
        !날아와서_부딪힌다(&mut a, &mut b),
        "창만 겹쳤는데 부딪혔다 — 반경이 몸통보다 크다"
    );
}

#[test]
fn 반경_밖으로_지나가면_부딪히지_않는다() {
    let (_, b, 부딪혔나) = 관통시킨다(PINBALL_COLLIDE_RADIUS + 1.0);
    assert!(!부딪혔나, "반경 밖인데 부딪혔다");
    assert_eq!((b.vx, b.vy), (0.0, 0.0), "안 닿았으면 안 움직여야 한다");
}

#[test]
fn 바닥에_선_마리는_맞으면_뜬다() {
    let mut a = 중심에(400.0, BOUNDS.floor_y, 600.0, 0.0);
    let mut b = 중심에(480.0, BOUNDS.floor_y, 0.0, 0.0);
    // 서 있는 상태를 만든다 — 공중이 아니어야 띄우기가 걸린다.
    b.enter(Behavior::Walk, 100_000);
    assert!(날아와서_부딪힌다(&mut a, &mut b));
    assert!(
        b.vy < 0.0,
        "수평으로만 밀리면 다음 틱에 곧바로 착지해 맞은 것이 안 보인다 (vy={})",
        b.vy
    );
    // **조금** 뜨는 것이지 솟는 것이 아니다 — 옆에서 맞았으면 옆으로 날아가야 한다.
    assert!(
        b.vx.abs() > b.vy.abs(),
        "옆에서 맞았는데 위로 솟는다 (vx={:.0}, vy={:.0})",
        b.vx,
        b.vy
    );
}

#[test]
fn 핀볼이_꺼진_쌍은_부딪히지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(300.0, -300.0);
    a.set_pinball(false);
    b.set_pinball(false);
    assert!(!날아와서_부딪힌다(&mut a, &mut b), "핀볼을 껐는데 부딪혔다");
}

#[test]
fn 들려_있는_마리는_부딪히지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(600.0, 0.0);
    b.drag_start(0);
    assert!(
        !날아와서_부딪힌다(&mut a, &mut b),
        "사용자가 들고 있는 마리를 밀어낼 수는 없다"
    );
}

#[test]
fn 정확히_겹쳐도_속도가_망가지지_않는다() {
    // 지금 중심이 **정확히** 같다 — 방향을 위치에서 못 뽑는 자리다.
    let mut a = 중심에(400.0, 400.0, 600.0, 0.0);
    let mut b = 중심에(400.0, 400.0, 0.0, 0.0);
    assert_eq!(중심(&a), 중심(&b), "이 테스트는 정확히 겹쳐야 의미가 있다");
    assert!(날아와서_부딪힌다(&mut a, &mut b));
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
        !날아와서_부딪힌다(&mut a, &mut b),
        "겹쳐 서는 것은 평소에도 일어난다 — 밀 이유가 없다"
    );
}

/// 자극이 5분간 없으면 펭귄은 존다. 얻어맞는 것을 자극으로 안 세면 **랠리 한복판에서**
/// 그 문턱에 걸린다 — 다른 마리에게만 맞으며 튀는 마리에게는 클릭이 한 번도 없다.
#[test]
fn 부딪히는_것도_자극이다() {
    let (mut a, mut b) = 마주_본_쌍(600.0, 0.0);
    for p in [&mut a, &mut b] {
        p.last_stimulus_ms = 0;
    }
    assert!(날아와서_부딪힌다(&mut a, &mut b));
    for p in [&a, &b] {
        assert_eq!(
            p.last_stimulus_ms, 1_000,
            "얻어맞은 것이 자극으로 안 세어졌다"
        );
    }
}

#[test]
fn 부딪혀도_난수를_쓰지_않는다() {
    let (mut a, mut b) = 마주_본_쌍(500.0, -500.0);
    let (a난수, b난수) = (a.rng, b.rng);
    assert!(날아와서_부딪힌다(&mut a, &mut b));
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
    let (mut pets, 나는, 선) = 날아가는_한_마리와_선_한_마리(true);
    let 처음_속력 = 속력(pets.get(나는).unwrap());
    let mut 부딪힌_틱을_봤나 = false;
    let mut t = 0;
    while t < 1_000 && !부딪힌_틱을_봤나 {
        t += 50;
        let stepped = pets.step_all(t, |_| Some(&w));
        // **부딪힌 바로 그 틱의 스냅샷**을 본다. 브릿지에 나가는 것이 이 반환값이라,
        // 부딪힘 뒤 스냅샷을 다시 발급하지 않으면 웹뷰가 한 틱 늦은 동작을 그린다 —
        // 다음 틱까지 기다려서 보면 그 지연이 안 보인다.
        if pets.get(선).unwrap().behavior() == Behavior::Thrown {
            let 스냅샷 = stepped
                .iter()
                .find(|(id, _)| *id == 선)
                .map(|(_, s)| *s)
                .expect("이번 틱에 돈 마리다");
            assert_eq!(
                스냅샷.behavior,
                Behavior::Thrown,
                "부딪힌 틱의 스냅샷이 아직 {:?}다 — 웹뷰가 한 틱 늦게 그린다",
                스냅샷.behavior
            );
            assert!(스냅샷.air, "날아갔으면 떠 있어야 한다");
            부딪힌_틱을_봤나 = true;
        }
    }
    assert!(부딪힌_틱을_봤나, "서 있던 마리가 안 날아갔다");
    let 맞은 = pets.get(선).unwrap();
    assert!(맞은.vx > 0.0, "가던 쪽으로 밀려야 한다 (vx={})", 맞은.vx);
    // **양쪽 다 바뀐다** — 친 쪽이 그대로면 볼링 연쇄와 다를 바 없다.
    assert!(
        속력(pets.get(나는).unwrap()) < 처음_속력,
        "친 쪽이 속도를 하나도 안 잃었다"
    );
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

/// 겹쳐 걷는 두 마리는 서로를 밀지 않는다. **시드를 다르게 준다** — 같은 시드로 같은
/// 자리에 놓으면 두 마리가 영원히 똑같이 움직여 상대 위치가 늘 정확히 0이라, 방향 대체
/// 분기에서 즉시 빠져나가고 정작 보려던 것을 안 본다.
#[test]
fn 겹쳐_걷는_두_마리는_서로를_밀지_않는다() {
    let w = world();
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &w, 400.0).unwrap();
    let b = pets.add(11, 0, &w, 430.0).unwrap();
    for id in [a, b] {
        let p = pets.get_mut(id).unwrap();
        p.set_pinball(true);
        // 걷기를 길게 걸어 이 창에서는 다음 동작 추첨이 돌지 않게 한다.
        p.enter(Behavior::Walk, 100_000);
    }
    // 서로 마주 보게 세운다 — 마주 걸어와도 안 밀린다는 것까지 본다.
    pets.get_mut(a).unwrap().facing = Facing::Right;
    pets.get_mut(b).unwrap().facing = Facing::Left;
    let mut 가장_가까웠던 = f64::MAX;
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
        let 사이 = (pets.get(a).unwrap().center_x() - pets.get(b).unwrap().center_x()).abs();
        가장_가까웠던 = 가장_가까웠던.min(사이);
    }
    assert!(
        가장_가까웠던 < PINBALL_COLLIDE_RADIUS,
        "3초 내내 반경 밖에 있었으면 아무것도 검증하지 않은 것이다 \
         (가장 가까웠을 때 {가장_가까웠던:.0}px)"
    );
}

#[test]
fn 볼링_판이_도는_동안에는_마리끼리_부딪히지_않는다() {
    let w = world();
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &w, 400.0).unwrap();
    let b = pets.add(11, 0, &w, 480.0).unwrap();
    // 셋째는 계속 핀으로 서 있는다 — 아무도 안 남으면 판이 그 자리에서 접혀
    // 게이트가 열려 버려 아무것도 검증하지 못한다.
    let 남는_핀 = pets.add(13, 0, &w, 900.0).unwrap();
    for id in [a, b, 남는_핀] {
        pets.get_mut(id).unwrap().set_pinball(true);
    }
    assert!(pets.start_bowling(0, BOUNDS), "판이 안 열렸다");
    // 공에 맞아 튕겨 나간 핀 둘이 마주 보고 날아가는 상황을 만든다.
    for (id, vx) in [(a, 900.0), (b, -900.0)] {
        let p = pets.get_mut(id).unwrap();
        p.y = BOUNDS.floor_y - 300.0;
        p.vx = vx;
        p.vy = 0.0;
        p.enter(Behavior::Thrown, 0);
    }
    assert!(pets.bowling().is_some(), "판이 벌써 접혔다");
    let 전 = (pets.get(a).unwrap().vx, pets.get(b).unwrap().vx);
    pets.step_all(50, |_| Some(&w));
    assert_eq!(
        (pets.get(a).unwrap().vx, pets.get(b).unwrap().vx),
        전,
        "판이 도는 동안 핀은 판이 몬다 — 두 물리가 같은 마리를 두고 다투면 \
         맞은 핀이 board.knock을 거치지 않고 빠져 볼링 연쇄가 조용히 끊긴다"
    );
}

/// 핀볼을 켠 마리와 안 켠 마리가 섞여 있어도 켠 쌍은 부딪힌다. 전 마리를 훑는
/// 조기 탈출이 "전부 켜졌을 때만"으로 바뀌면 판정이 통째로 죽는다.
#[test]
fn 핀볼을_켠_쌍만_부딪힌다() {
    let w = world();
    let (mut pets, _, 선) = 날아가는_한_마리와_선_한_마리(true);
    let 구경꾼 = pets.add(9, 0, &w, 900.0).unwrap();
    let p = pets.get_mut(구경꾼).unwrap();
    p.set_pinball(false);
    p.enter(Behavior::Walk, 100_000);
    let mut t = 0;
    while t < 1_000 && pets.get(선).unwrap().behavior() != Behavior::Thrown {
        t += 50;
        pets.step_all(t, |_| Some(&w));
    }
    assert_eq!(
        pets.get(선).unwrap().behavior(),
        Behavior::Thrown,
        "핀볼을 안 켠 마리가 섞였다고 판정이 통째로 죽었다"
    );
    assert_ne!(
        pets.get(구경꾼).unwrap().behavior(),
        Behavior::Thrown,
        "핀볼을 안 켠 마리는 부딪히지 않는다"
    );
}

/// 프로덕션 틱은 밀리면 `MAX_STEP_MS`(250ms)까지 늘어나고, 넓은 화면에서는 던지기
/// 상한이 세계 폭에 비례해 올라가는데 부딪히는 반경은 고정 px다. 볼링의 "확장
/// 모니터에서 한 줄만 쓰러졌다"가 나온 것과 같은 자리다.
#[test]
fn 틱이_밀리고_화면이_넓어도_뛰어넘지_않는다() {
    let 넓은 = World::single(Bounds {
        left: 0.0,
        right: 3_840.0,
        top: 0.0,
        floor_y: 1_000.0,
    });
    let mut pets = Pets::new();
    let 나는 = pets.add(7, 0, &넓은, 100.0).unwrap();
    let 선 = pets.add(11, 0, &넓은, 1_800.0).unwrap();
    for id in [나는, 선] {
        let p = pets.get_mut(id).unwrap();
        p.set_pinball(true);
        p.enter(Behavior::Walk, 100_000);
    }
    let p = pets.get_mut(나는).unwrap();
    p.y = 1_000.0 - 40.0;
    p.vx = 3_840.0 * 1.4;
    p.enter(Behavior::Thrown, 0);

    let mut t = 0;
    while t < 3_000 && pets.get(선).unwrap().behavior() != Behavior::Thrown {
        // 밀린 틱 — 한 틱에 1,300px 넘게 날아 반경(104)의 열 배를 지난다.
        t += 250;
        pets.step_all(t, |_| Some(&넓은));
    }
    assert_eq!(
        pets.get(선).unwrap().behavior(),
        Behavior::Thrown,
        "한 틱에 화면 3분의 1을 날아가면서 이웃을 통과했다"
    );
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
    let mut 다_멎은_적_있나 = false;
    let mut t = 0;
    while t < 20_000 {
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
        다_멎은_적_있나 |= pets
            .ids()
            .iter()
            .all(|id| 속력(pets.get(*id).unwrap()) == 0.0);
    }
    // **여기가 이 테스트의 전부다.** 좌표가 유한하고 세계 안이라는 것은 `clamp`가
    // 무조건 보장하고 속력 상한은 `clamp_throw`가 보장하므로, 그것만 보면 임펄스를
    // 백 배로 불려도 초록이다. 실제로 봐야 하는 것은 **난장판이 스스로 잦아드는가**다.
    //
    // 끝 시각의 상태를 보면 안 된다 — 멎은 뒤에도 펭귄들은 계속 살아서 헤엄치고
    // 떨어지고, 핀볼에서는 그것도 서로를 친다. 이 앱에 "영원한 정지"는 없다.
    assert!(
        다_멎은_적_있나,
        "여덟 마리를 한꺼번에 던졌는데 20초 안에 한 번도 전부 멎지 않았다 — \
         부딪힘이 에너지를 만들고 있다"
    );
}
