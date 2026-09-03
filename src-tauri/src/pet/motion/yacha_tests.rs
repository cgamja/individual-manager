//! 야차 마리 국면의 검사. **핵심은 "맞아도 밀려나지 않는다"다** (R7).

use crate::pet::test_support::*;
use crate::pet::*;

fn 링에_선_펭귄() -> (Pet, World) {
    let world = world();
    let mut pet = Pet::new(1, 0, &world);
    let spot = (
        (BOUNDS.left + BOUNDS.right) / 2.0,
        (BOUNDS.top + BOUNDS.floor_y) / 2.0,
    );
    assert!(pet.start_yacha(0, spot, Facing::Right));
    let mut now = 0;
    while !pet.yacha_stood() && now < 20_000 {
        now += 50;
        pet.step(now, &world);
    }
    assert!(pet.yacha_stood(), "한가운데에 못 섰다");
    (pet, world)
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
    // 판이 화면 세로 중앙이라 하나라도 지상이면 그 국면의 마리만 바닥으로
    // 끌려 내려가 혼자 화면 아래에 눕는다.
    for phase in [
        YachaPhase::Gather,
        YachaPhase::Hunt,
        YachaPhase::Circle,
        YachaPhase::Back,
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
fn 난투_중에는_스스로_안_움직인다() {
    // 좌표의 유일한 출처가 판이라야 넉백이 끼어들 자리가 없다.
    let (mut pet, world) = 링에_선_펭귄();
    let 처음 = (pet.snapshot().x, pet.snapshot().y);
    let mut now = 1_000;
    while now < 4_000 {
        now += 50;
        pet.step(now, &world);
    }
    assert_eq!((pet.snapshot().x, pet.snapshot().y), 처음, "혼자 움직였다");
}

#[test]
fn 판이_준_자리를_그대로_받는다() {
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_apply(1_000, (300.0, 200.0), YachaPhase::Hunt, Facing::Left);
    assert_eq!(pet.snapshot().x, 300.0);
    assert_eq!(pet.snapshot().y, 200.0);
    assert_eq!(pet.snapshot().facing, Facing::Left);
    assert!(matches!(
        pet.behavior(),
        Behavior::Yacha {
            yacha: YachaPhase::Hunt
        }
    ));
}

#[test]
fn 같은_국면을_다시_주면_되감기지_않는다() {
    // 매 틱 `enter`를 밟으면 CSS 애니메이션이 되감겨 주먹이 영영 안 뻗는다.
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_apply(1_000, (300.0, 200.0), YachaPhase::Punch, Facing::Right);
    let 끝나는_시각 = pet.behavior_until_for_test();
    pet.yacha_apply(1_040, (302.0, 200.0), YachaPhase::Punch, Facing::Right);
    assert_eq!(
        pet.behavior_until_for_test(),
        끝나는_시각,
        "같은 국면인데 만료가 밀렸다 — 애니메이션이 되감긴다"
    );
}

#[test]
fn 쓰러진_마리는_스스로_일어나지_않는다() {
    let (mut pet, world) = 링에_선_펭귄();
    pet.yacha_apply(1_000, (300.0, 200.0), YachaPhase::Down, Facing::Right);
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
}

#[test]
fn 세레모니_중에는_참여자로_안_센다() {
    // 안 그러면 벨트 수여가 나오기 전에 판이 접힌다.
    let (mut pet, _) = 링에_선_펭귄();
    pet.yacha_champ(1_000);
    assert!(pet.in_yacha());
    assert!(!pet.is_yachaing());
}

#[test]
fn 판이_끝나도_철푸덕하지_않는다() {
    let (mut pet, world) = 링에_선_펭귄();
    pet.leave_ring(1_000, BOUNDS);
    assert!(
        matches!(pet.behavior(), Behavior::Swim),
        "헤엄 하강이 아니라 {:?}로 나갔다",
        pet.behavior()
    );
    let mut now = 1_000;
    while now < 30_000 {
        now += 50;
        pet.step(now, &world);
        if pet.behavior().is_landing() {
            assert!(
                matches!(pet.behavior(), Behavior::Land),
                "철푸덕하거나 널브러졌다: {:?}",
                pet.behavior()
            );
            return;
        }
        if matches!(pet.behavior(), Behavior::Idle { .. } | Behavior::Walk) {
            return;
        }
    }
}

#[test]
fn 판에서_빠지면_평소_동작으로_돌아간다() {
    let (mut pet, _) = 링에_선_펭귄();
    pet.leave_ring(1_000, BOUNDS);
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
    // 맞기만 한 마리는 안 오른다 — 라운드마다 딱 한 마리다.
    pet.yacha_apply(1_000, (300.0, 200.0), YachaPhase::Hurt, Facing::Right);
    assert_eq!(pet.snapshot().punch_seq, 0);
    pet.yacha_thud(false);
    assert_eq!(pet.snapshot().punch_seq, 1);
    assert!(!pet.snapshot().punch_down);
    pet.yacha_thud(true);
    assert_eq!(pet.snapshot().punch_seq, 2);
    assert!(pet.snapshot().punch_down, "쓰러뜨린 한 방이 표시가 안 된다");
}
