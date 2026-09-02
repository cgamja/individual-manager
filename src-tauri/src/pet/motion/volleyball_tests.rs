use crate::pet::test_support::*;
use crate::pet::*;

/// 판은 화면 세로 중앙이다 — 모래(화면 바닥)가 아니다.
const 자리: (f64, f64) = (600.0, (BOUNDS.top + BOUNDS.floor_y) / 2.0);
const 폭: (f64, f64) = (400.0, 800.0);

fn 코트로(pet: &mut Pet, now_ms: u64) {
    assert!(pet.start_volley(now_ms, 자리, 폭, Facing::Right));
}

/// 자리에 설 때까지 굴린다.
fn 세운다(pet: &mut Pet, world: &World, mut now_ms: u64) -> u64 {
    코트로(pet, now_ms);
    for _ in 0..200 {
        now_ms += 50;
        pet.step(now_ms, world);
        if pet.volley_stood() {
            return now_ms;
        }
    }
    panic!("자리에 못 섰다");
}

#[test]
fn 자기_자리에_도착하면_선다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let now = 세운다(&mut pet, &world, 0);
    let s = pet.snapshot();
    assert_eq!(
        s.behavior,
        Behavior::Volleyball {
            volley: VolleyPhase::Ready
        }
    );
    assert!((s.x - 자리.0).abs() < 1e-9, "x가 자리에 안 앉았다: {}", s.x);
    assert!((s.y - 자리.1).abs() < 1e-9, "y가 자리에 안 앉았다: {}", s.y);
    let _ = now;
}

#[test]
fn 판_위에서는_내내_공중이다() {
    // **판이 화면 세로 중앙으로 올라가면서 뒤집힌 전제다.** 예전에는 코트가
    // 모래사장이라 서면 지상이었지만, 이제는 볼링 핀처럼 떠 있다 — 한 국면이라도
    // 지상이면 `clamp`가 그때 펭귄을 바닥으로 끌어내려 코트에서 떨어진다.
    // "국면마다 `air`가 맞게 서 있는가"를 보는 이 테스트의 의도는 그대로다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    코트로(&mut pet, 0);
    assert!(pet.snapshot().air, "모이는 중에는 날아가야 한다");
    세운다_그대로(&mut pet, &world);
    assert!(pet.snapshot().air, "서 있을 때도 판 위에 떠 있다");
    assert!(
        (pet.snapshot().y - 자리.1).abs() < 1e-9,
        "판 높이에서 벗어났다: {}",
        pet.snapshot().y
    );
}

#[test]
fn 순간이동하지_않는다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let 시작 = pet.snapshot().x;
    코트로(&mut pet, 0);
    pet.step(50, &world);
    let 한틱 = (pet.snapshot().x - 시작).abs();
    assert!(
        한틱 <= VOLLEY_GATHER_SPEED * 0.05 + 1e-6,
        "한 틱에 {한틱}px 나 갔다"
    );
}

#[test]
fn 들고_있으면_판에_안_들어간다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    pet.drag_start(0);
    assert!(!pet.start_volley(100, 자리, 폭, Facing::Right));
    assert_eq!(pet.snapshot().behavior, Behavior::Dragged);
}

#[test]
fn 이미_판에_있으면_다시_안_들어간다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    코트로(&mut pet, 0);
    assert!(!pet.start_volley(100, (0.0, 0.0), 폭, Facing::Right));
}

#[test]
fn 받으러_뛰면_목적지에_선다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    pet.volley_chase(now, 500.0);
    assert_eq!(
        pet.snapshot().behavior,
        Behavior::Volleyball {
            volley: VolleyPhase::Chase
        }
    );
    for _ in 0..200 {
        now += 50;
        pet.step(now, &world);
        if pet.snapshot().behavior
            == (Behavior::Volleyball {
                volley: VolleyPhase::Ready,
            })
        {
            break;
        }
    }
    assert!((pet.snapshot().x - 500.0).abs() < 1e-6, "목적지에 안 섰다");
}

#[test]
fn 뛰어도_자기_코트_밖으로_안_나간다() {
    // 판이 실수해도 네트를 넘어가는 그림이 안 나와야 한다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    pet.volley_chase(now, 5_000.0);
    for _ in 0..400 {
        now += 50;
        pet.step(now, &world);
        assert!(
            pet.snapshot().x <= 폭.1 + 1e-6,
            "코트 밖으로 나갔다: {}",
            pet.snapshot().x
        );
    }
}

#[test]
fn 뛰라고_다시_시켜도_국면을_되감지_않는다() {
    // 매 틱 국면을 새로 밟으면 CSS 애니메이션이 계속 되감겨 뛰는 그림이 굳는다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let now = 세운다(&mut pet, &world, 0);
    pet.volley_chase(now, 500.0);
    let 처음 = pet.behavior_until_ms;
    pet.volley_chase(now + 100, 450.0);
    assert_eq!(pet.behavior_until_ms, 처음, "국면을 다시 밟았다");
    assert!((pet.target.0 - 450.0).abs() < 1e-9, "목적지는 갈아 끼워야 한다");
}

#[test]
fn 때리는_동안_제자리에_있다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    pet.volley_bump(now);
    let (x, y) = (pet.snapshot().x, pet.snapshot().y);
    for _ in 0..5 {
        now += 50;
        pet.step(now, &world);
    }
    assert!((pet.snapshot().x - x).abs() < 1e-9);
    assert!((pet.snapshot().y - y).abs() < 1e-9);
}

#[test]
fn 때리고_나면_다시_선다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    pet.volley_bump(now);
    now += VOLLEY_BUMP_MS + 50;
    pet.step(now, &world);
    assert_eq!(
        pet.snapshot().behavior,
        Behavior::Volleyball {
            volley: VolleyPhase::Ready
        }
    );
}

#[test]
fn 이긴_쪽은_좋아하고_진_쪽은_약_오른다() {
    let world = world();
    let mut 이긴 = Pet::new(1, 0, &world);
    let mut 진 = Pet::new(2, 0, &world);
    let now = 세운다(&mut 이긴, &world, 0);
    세운다(&mut 진, &world, 0);
    이긴.volley_finish(now, true);
    진.volley_finish(now, false);
    assert_eq!(
        이긴.snapshot().behavior,
        Behavior::Volleyball {
            volley: VolleyPhase::Cheer
        }
    );
    assert_eq!(
        진.snapshot().behavior,
        Behavior::Volleyball {
            volley: VolleyPhase::Sulk
        }
    );
}

#[test]
fn 축하가_끝나면_선_자리에서_내려앉는다() {
    // **판이 공중이라 나가는 길이 낙하다** (볼링의 `Scatter`와 같은 자리).
    // 지키려던 것은 "원래 있던 자리로 되돌려 보내지 않는다"(R12)이고, 그건
    // **가로 자리가 안 변한다**는 뜻이라 그대로 유효하다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    let 선_자리 = pet.snapshot().x;
    pet.volley_finish(now, true);
    now += VOLLEY_CHEER_MS + 50;
    pet.step(now, &world);
    // **자유낙하가 아니라 내려앉기다.** 판이 화면 세로 중앙이라 떨어뜨리면
    // 착지 속도가 철푸덕 문턱을 넘어 매 판마다 전원이 철푸덕한다.
    assert_eq!(
        pet.snapshot().behavior,
        Behavior::Swim,
        "공중에서 축하가 끝났으면 날개를 저어 내려앉아야 한다"
    );
    assert!(
        (pet.snapshot().x - 선_자리).abs() < 1e-6,
        "가로 자리가 움직였다: {} → {}",
        선_자리,
        pet.snapshot().x
    );
    // 떨어지고 나면 평소 동작으로 돌아간다.
    for _ in 0..200 {
        now += 50;
        pet.step(now, &world);
    }
    assert!(
        !matches!(pet.snapshot().behavior, Behavior::Volleyball { .. }),
        "아직 코트에 있다: {:?}",
        pet.snapshot().behavior
    );
}

#[test]
fn 판이_끝나도_철푸덕하지_않는다() {
    // **20초짜리 판의 끝이 여덟 마리 동시 철푸덕이면 안 된다.** 이 레포는 헤엄이
    // 끝날 때마다 저절로 나던 철푸덕을 이미 한 번 걷어냈고, 판을 화면 세로
    // 중앙으로 올리면서 같은 것이 돌아올 뻔했다 (낙하 328px → 착지 767px/s).
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let mut now = 세운다(&mut pet, &world, 0);
    pet.volley_finish(now, true);
    now += VOLLEY_CHEER_MS + 50;
    for _ in 0..400 {
        now += 50;
        pet.step(now, &world);
        let b = pet.snapshot().behavior;
        assert!(
            !matches!(b, Behavior::Splat | Behavior::Sprawl),
            "판이 끝나고 {b:?} 했다 — 내려앉기가 아니라 떨어졌다"
        );
        if !pet.snapshot().air {
            break;
        }
    }
    assert!(!pet.snapshot().air, "바닥까지 안 내려왔다");
}

#[test]
fn 득점하면_판에서_빠진다() {
    // `Cheer`/`Sulk`는 판이 이미 끝난 뒤의 여운이라, 판이 붙들면 코트가 안 걷힌다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let now = 세운다(&mut pet, &world, 0);
    assert!(pet.is_volleying());
    pet.volley_finish(now, true);
    assert!(!pet.is_volleying());
}

#[test]
fn 랠리_국면은_난수를_소비하지_않는다() {
    // 판이 마리의 난수를 태우면 참여한 마리만 이후 동작 시퀀스가 밀린다.
    let world = world();
    let mut 판에_넣은 = Pet::new(0xABCD, 0, &world);
    let mut 그냥 = Pet::new(0xABCD, 0, &world);

    let mut now = 세운다(&mut 판에_넣은, &world, 0);
    판에_넣은.volley_chase(now, 500.0);
    for _ in 0..20 {
        now += 50;
        판에_넣은.step(now, &world);
    }
    판에_넣은.volley_bump(now);
    now += VOLLEY_BUMP_MS + 50;
    판에_넣은.step(now, &world);

    assert_eq!(
        판에_넣은.next_u64(),
        그냥.next_u64(),
        "랠리가 마리의 난수를 태웠다"
    );
}

#[test]
fn 안전_상한이_지나면_스스로_풀린다() {
    // 판이 사라져도 영원히 코트에 서 있지 않는다.
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    세운다(&mut pet, &world, 0);
    pet.step(VOLLEY_MAX_MS + 10_000, &world);
    assert!(
        !matches!(pet.snapshot().behavior, Behavior::Volleyball { .. }),
        "안전 상한을 넘겼는데 아직 코트에 있다"
    );
}

#[test]
fn 네트를_보고_선다() {
    let world = world();
    let mut 왼쪽 = Pet::new(1, 0, &world);
    assert!(왼쪽.start_volley(0, 자리, 폭, Facing::Right));
    세운다_그대로(&mut 왼쪽, &world);
    assert_eq!(왼쪽.snapshot().facing, Facing::Right);

    let mut 오른쪽 = Pet::new(2, 0, &world);
    assert!(오른쪽.start_volley(0, 자리, 폭, Facing::Left));
    세운다_그대로(&mut 오른쪽, &world);
    assert_eq!(오른쪽.snapshot().facing, Facing::Left);
}

/// 이미 `start_volley`를 부른 마리를 자리에 세운다.
fn 세운다_그대로(pet: &mut Pet, world: &World) {
    let mut now = 0;
    for _ in 0..200 {
        now += 50;
        pet.step(now, world);
        if pet.volley_stood() {
            return;
        }
    }
    panic!("자리에 못 섰다");
}
