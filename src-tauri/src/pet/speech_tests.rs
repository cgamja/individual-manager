//! 집결형 한 판이 도는 동안 말풍선을 막는 규칙의 검사.
//!
//! **판정이 `Behavior::silences_speech` 하나에 모여 있다.** 모드마다 제 자리에서
//! 막게 두면 새 모드를 얹을 때 빠뜨리는 것이 기본값이 되기 때문이다.
//! **핀볼 것을 반드시 함께 본다** — 목록에 하나 더 넣는 실수를 그게 잡는다.


use super::*;

const 세계: Bounds = Bounds {
    left: 0.0,
    right: 1_440.0,
    top: 0.0,
    floor_y: 800.0,
};

fn 무리(n: usize) -> (Pets, World) {
    let world = World::single(세계);
    let mut pets = Pets::new();
    for i in 0..n {
        pets.add(7, 0, &world, 200.0 + 60.0 * i as f64)
            .expect("마릿수 상한 안이다");
    }
    (pets, world)
}

/// `ms` 동안 돌리면서 말풍선이 한 번이라도 떴는지 본다.
fn 말풍선이_떴나(pets: &mut Pets, world: &World, from: u64, ms: u64) -> bool {
    let mut now = from;
    let 끝 = from + ms;
    while now < 끝 {
        now += 50;
        for (_, snap) in pets.step_all(now, |_| Some(world)) {
            if snap.speech.is_some() {
                return true;
            }
        }
    }
    false
}

#[test]
fn 볼링_중에는_말풍선이_안_뜬다() {
    let (mut pets, world) = 무리(4);
    assert!(pets.start_bowling(0, 세계));
    assert!(
        !말풍선이_떴나(&mut pets, &world, 0, 30_000),
        "판이 도는데 말풍선이 떴다"
    );
}

#[test]
fn 발리볼_중에는_말풍선이_안_뜬다() {
    let (mut pets, world) = 무리(4);
    assert_eq!(pets.start_volleyball(0, 세계, 1), Ok(()));
    assert!(
        !말풍선이_떴나(&mut pets, &world, 0, 18_000),
        "판이 도는데 말풍선이 떴다"
    );
}

#[test]
fn 야차_중에는_말풍선이_안_뜬다() {
    let (mut pets, world) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    assert!(
        !말풍선이_떴나(&mut pets, &world, 0, 20_000),
        "판이 도는데 말풍선이 떴다"
    );
}

#[test]
fn 핀볼_중에는_말풍선이_뜬다() {
    // **사용자가 명시적으로 제외했다.** 핀볼은 판이 아니라 세계의 규칙이라
    // 화면 가운데를 가리지 않는다. 목록에 하나 더 넣는 실수를 이 검사가 잡는다.
    let (mut pets, world) = 무리(2);
    for id in pets.ids() {
        pets.get_mut(id).unwrap().set_pinball(true);
    }
    assert!(
        말풍선이_떴나(&mut pets, &world, 0, 60_000),
        "핀볼인데 말풍선이 막혔다"
    );
}

#[test]
fn 평소에는_말풍선이_뜬다() {
    let (mut pets, world) = 무리(2);
    assert!(말풍선이_떴나(&mut pets, &world, 0, 60_000));
}

#[test]
fn 판이_시작되면_떠_있던_말풍선이_지워진다() {
    let (mut pets, world) = 무리(2);
    // 말풍선이 뜰 때까지 돌린다.
    let mut now = 0;
    while now < 60_000 {
        now += 50;
        pets.step_all(now, |_| Some(&world));
        if pets
            .ids()
            .iter()
            .any(|id| pets.get(*id).unwrap().snapshot().speech.is_some())
        {
            break;
        }
    }
    assert!(
        pets.ids()
            .iter()
            .any(|id| pets.get(*id).unwrap().snapshot().speech.is_some()),
        "준비: 말풍선이 안 떴다"
    );

    assert_eq!(pets.start_yacha(now, 세계, 1), Ok(()));
    now += 50;
    let 스냅 = pets.step_all(now, |_| Some(&world));
    for (id, s) in 스냅 {
        assert!(s.speech.is_none(), "{id}번의 말풍선이 안 지워졌다");
    }
}

#[test]
fn 판이_끝나면_말풍선이_돌아온다() {
    let (mut pets, world) = 무리(2);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    // 판이 끝날 때까지 돌린다.
    let mut now = 0;
    while now < 60_000 && pets.yacha().is_some() {
        now += 50;
        pets.step_all(now, |_| Some(&world));
    }
    assert!(pets.yacha().is_none(), "판이 안 끝났다");
    assert!(
        말풍선이_떴나(&mut pets, &world, now, 60_000),
        "판이 끝났는데 말풍선이 안 돌아왔다"
    );
}

#[test]
fn 판이_끝나자마자_밀린_대사가_튀어나오지_않는다() {
    // 막힌 동안의 대사는 그냥 안 나온 것이다.
    let (mut pets, world) = 무리(2);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    let mut now = 0;
    while now < 60_000 && pets.yacha().is_some() {
        now += 50;
        pets.step_all(now, |_| Some(&world));
    }
    // 판이 끝난 **직후** 한 틱에는 조용해야 한다.
    now += 50;
    for (id, s) in pets.step_all(now, |_| Some(&world)) {
        assert!(s.speech.is_none(), "{id}번이 끝나자마자 말했다");
    }
}

#[test]
fn 판정은_한_곳에만_있다() {
    // 집결형 셋만 막고 나머지는 낸다 — "모르면 낸다"가 기본값이다.
    assert!(Behavior::Bowling {
        bowling: BowlingPhase::Ready
    }
    .silences_speech());
    assert!(Behavior::Volleyball {
        volley: VolleyPhase::Ready
    }
    .silences_speech());
    assert!(Behavior::Yacha {
        yacha: YachaPhase::Guard
    }
    .silences_speech());

    for b in [
        Behavior::Walk,
        Behavior::Turn,
        Behavior::Swim,
        Behavior::Sleep,
        Behavior::Squawk,
        Behavior::Swing,
        Behavior::Dragged,
        Behavior::Falling,
        Behavior::Thrown,
        Behavior::Land,
        Behavior::Splat,
        Behavior::Sprawl,
        Behavior::Tumble,
        Behavior::Slide,
        Behavior::Idle {
            idle: IdleKind::Stretch,
        },
        Behavior::Sassy {
            sassy: SassyKind::EyeRoll,
        },
        Behavior::Freakout {
            freakout: FreakoutPhase::Dash,
        },
        Behavior::IceFishing {
            fishing: FishingPhase::Wait,
        },
    ] {
        assert!(!b.silences_speech(), "{b:?}가 말풍선을 막는다");
    }
}
