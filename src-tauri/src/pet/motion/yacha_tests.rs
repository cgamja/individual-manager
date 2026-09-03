//! 야차 마리 국면의 검사. **핵심은 "맞아도 x가 안 변한다"다** (R7).

use crate::pet::test_support::*;
use crate::pet::*;

fn 링에_선_펭귄() -> (Pet, World) {
    let world = world();
    let bounds = BOUNDS;
    let mut pet = Pet::new(1, 0, &world);
    let spot = ((bounds.left + bounds.right) / 2.0, bounds.floor_y / 2.0);
    assert!(pet.start_yacha(0, spot, Facing::Right));
    // 도착할 때까지 돌린다.
    let mut now = 0;
    while !pet.yacha_stood() && now < 20_000 {
        now += 50;
        pet.step(now, &world);
    }
    assert!(pet.yacha_stood(), "링에 못 섰다");
    (pet, world)
}

#[test]
fn 펀치를_맞아도_x가_변하지_않는다() {
    let (mut pet, world) = 링에_선_펭귄();
    let 처음 = pet.snapshot().x;
    pet.yacha_hurt(1_000, Facing::Left);
    assert_eq!(pet.snapshot().x, 처음, "맞자마자 밀렸다");
    // 휘청이는 내내도 그대로여야 한다.
    let mut now = 1_000;
    while now < 1_000 + YACHA_HURT_MS + 200 {
        now += 50;
        pet.step(now, &world);
        assert_eq!(pet.snapshot().x, 처음, "{now}ms에 밀렸다");
    }
}

#[test]
fn 펀치를_뻗어도_x가_변하지_않는다() {
    let (mut pet, world) = 링에_선_펭귄();
    let 처음 = pet.snapshot().x;
    pet.yacha_punch(1_000, Facing::Left);
    let mut now = 1_000;
    while now < 1_000 + YACHA_PUNCH_MS + 200 {
        now += 50;
        pet.step(now, &world);
        assert_eq!(pet.snapshot().x, 처음, "{now}ms에 밀렸다");
    }
}

#[test]
fn 맞으면_때린_쪽을_본다() {
    // 화남 표시가 둘 사이에 뜨게 하는 유일한 장치다 (KTD9d).
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_hurt(1_000, Facing::Left);
    assert_eq!(pet.snapshot().facing, Facing::Left);
    pet.yacha_hurt(1_200, Facing::Right);
    assert_eq!(pet.snapshot().facing, Facing::Right);
}

#[test]
fn 맞으면_피격_수가_는다() {
    let (mut pet, _) = 링에_선_펭귄();
    assert_eq!(pet.yacha_hits(), 0);
    pet.yacha_hurt(1_000, Facing::Left);
    pet.yacha_hurt(1_400, Facing::Left);
    assert_eq!(pet.yacha_hits(), 2);
}

#[test]
fn 모이기가_끝나면_가드로_넘어간다() {
    let (pet, _) = 링에_선_펭귄();
    assert!(matches!(
        pet.behavior(),
        Behavior::Yacha {
            yacha: YachaPhase::Guard
        }
    ));
}

#[test]
fn 야차는_전_국면이_공중이다() {
    // 링이 화면 세로 중앙이라 하나라도 지상이면 그 국면의 마리만 바닥으로
    // 끌려 내려가 혼자 화면 아래에 눕는다.
    for phase in [
        YachaPhase::Gather,
        YachaPhase::Guard,
        YachaPhase::Punch,
        YachaPhase::Hurt,
        YachaPhase::Down,
        YachaPhase::Win,
        YachaPhase::Champ,
    ] {
        assert!(
            Behavior::Yacha { yacha: phase }.is_airborne(),
            "{phase:?}가 지상이다"
        );
    }
}

#[test]
fn 펀치는_정해진_시간_뒤_가드로_돌아온다() {
    let (mut pet, world) = 링에_선_펭귄();
    pet.yacha_punch(1_000, Facing::Left);
    pet.step(1_000 + YACHA_PUNCH_MS - 50, &world);
    assert!(matches!(
        pet.behavior(),
        Behavior::Yacha {
            yacha: YachaPhase::Punch
        }
    ));
    pet.step(1_000 + YACHA_PUNCH_MS + 50, &world);
    assert!(matches!(
        pet.behavior(),
        Behavior::Yacha {
            yacha: YachaPhase::Guard
        }
    ));
}

#[test]
fn 쓰러진_마리는_스스로_일어나지_않는다() {
    let (mut pet, world) = 링에_선_펭귄();
    pet.yacha_down(1_000);
    let 누운_자리 = pet.snapshot().x;
    let mut now = 1_000;
    while now < 40_000 {
        now += 500;
        pet.step(now, &world);
    }
    assert!(
        matches!(
            pet.behavior(),
            Behavior::Yacha {
                yacha: YachaPhase::Down
            }
        ),
        "40초 뒤에 스스로 일어났다"
    );
    assert_eq!(pet.snapshot().x, 누운_자리, "누운 채로 흘러갔다");
}

#[test]
fn 쓰러진_마리는_판에서_안_빠진다() {
    // 링에 누워 있어야 하므로 참여자이긴 하다.
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_down(1_000);
    assert!(pet.is_yachaing());
    assert!(pet.yacha_is_down());
    assert!(!pet.yacha_stood());
}

#[test]
fn 세레모니_중에는_참여자로_안_센다() {
    // 안 그러면 축하 그림이 나오기 전에 링이 사라진다.
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_champ(1_000);
    assert!(!pet.is_yachaing());
}

#[test]
fn 판이_끝나도_철푸덕하지_않는다() {
    // 링이 화면 세로 중앙이라 자유낙하로 두면 착지 속도가 SPLAT_MIN_IMPACT를
    // 넘어 매 판마다 전원이 동시에 철푸덕한다.
    let (mut pet, world) = 링에_선_펭귄();
    let bounds = BOUNDS;
    pet.leave_ring(1_000, bounds);
    assert!(
        matches!(pet.behavior(), Behavior::Swim),
        "헤엄 하강이 아니라 {:?}로 나갔다",
        pet.behavior()
    );
    let mut now = 1_000;
    let mut 착지 = None;
    while now < 30_000 {
        now += 50;
        pet.step(now, &world);
        if pet.behavior().is_landing() {
            착지 = Some(pet.behavior());
            break;
        }
        if matches!(pet.behavior(), Behavior::Idle { .. } | Behavior::Walk) {
            break;
        }
    }
    if let Some(b) = 착지 {
        assert!(
            matches!(b, Behavior::Land),
            "철푸덕하거나 널브러졌다: {b:?}"
        );
    }
}

#[test]
fn 판에서_빠지면_평소_동작으로_돌아간다() {
    let (mut pet, _world) = 링에_선_펭귄();
    let bounds = BOUNDS;
    pet.leave_ring(1_000, bounds);
    assert!(!matches!(pet.behavior(), Behavior::Yacha { .. }));
}

#[test]
fn 이미_판에_있는_마리는_다시_안_받는다() {
    let (mut pet, _) = 링에_선_펭귄();
    assert!(!pet.start_yacha(2_000, (100.0, 100.0), Facing::Right));
}

#[test]
fn 들려_있으면_판에_안_들어간다() {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    pet.drag_start(0);
    assert!(!pet.start_yacha(0, (100.0, 100.0), Facing::Right));
}

#[test]
fn 대표_타격만_소리_신호를_올린다() {
    let (mut pet, _) = 링에_선_펭귄();
    assert_eq!(pet.snapshot().punch_seq, 0);
    pet.yacha_hurt(1_000, Facing::Left);
    assert_eq!(pet.snapshot().punch_seq, 0, "맞았다고 소리가 나면 안 된다");
    pet.yacha_thud(false);
    assert_eq!(pet.snapshot().punch_seq, 1);
    assert!(!pet.snapshot().punch_down);
    pet.yacha_thud(true);
    assert_eq!(pet.snapshot().punch_seq, 2);
    assert!(pet.snapshot().punch_down, "쓰러뜨린 한 방이 표시가 안 된다");
}

#[test]
fn 이웃이_쓰러지면_자리를_다시_잡는다() {
    let (mut pet, world) = 링에_선_펭귄();
    let 처음 = pet.snapshot().x;
    pet.yacha_restance((처음 + 200.0, pet.snapshot().y));
    let mut now = 1_000;
    while now < 5_000 {
        now += 50;
        pet.step(now, &world);
    }
    assert!(
        (pet.snapshot().x - (처음 + 200.0)).abs() < 1.0,
        "새 자리로 안 붙었다: {} → {}",
        처음,
        pet.snapshot().x
    );
}
