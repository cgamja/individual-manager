use crate::pet::test_support::*;
use crate::pet::*;

#[test]
fn 같은_시드는_같은_동작_시퀀스를_낳는다() {
    let mut a = Pet::new(2024, 0, &world());
    let mut b = Pet::new(2024, 0, &world());
    let seq_a: Vec<Behavior> = drive(&mut a, 100, 60_000, 100, &world())
        .iter()
        .map(|s| s.behavior)
        .collect();
    let seq_b: Vec<Behavior> = drive(&mut b, 100, 60_000, 100, &world())
        .iter()
        .map(|s| s.behavior)
        .collect();
    assert_eq!(seq_a, seq_b);
    let mut c = Pet::new(999, 0, &world());
    let seq_c: Vec<Behavior> = drive(&mut c, 100, 60_000, 100, &world())
        .iter()
        .map(|s| s.behavior)
        .collect();
    assert_ne!(seq_a, seq_c);
}

#[test]
fn 여러_종류의_동작이_나타난다() {
    let mut p = pet();
    let kinds: std::collections::HashSet<Behavior> = drive(&mut p, 100, 80_000, 100, &world())
        .iter()
        .map(|s| s.behavior)
        .collect();
    assert!(
        kinds.len() >= 3,
        "80초 동안 최소 3가지 동작이 나와야 한다 (실제: {kinds:?})"
    );
}

#[test]
fn 유휴_동작은_연속으로_같은_종류가_반복되지_않는다() {
    let mut p = pet();
    let idles: Vec<IdleKind> = drive(&mut p, 100, 80_000, 100, &world())
        .iter()
        .filter_map(|s| match s.behavior {
            Behavior::Idle { idle } => Some(idle),
            _ => None,
        })
        .collect();
    let mut compressed: Vec<IdleKind> = Vec::new();
    for k in idles {
        if compressed.last() != Some(&k) {
            compressed.push(k);
        }
    }
    for pair in compressed.windows(2) {
        assert_ne!(pair[0], pair[1], "같은 유휴 동작이 연달아 선택됐다");
    }
}

#[test]
fn 오랫동안_자극이_없으면_졸기로_전이한다() {
    let mut p = pet();
    let seen = drive(&mut p, 100, SLEEP_AFTER_MS + 30_000, 250, &world());
    assert!(
        seen.iter().any(|s| s.behavior == Behavior::Sleep),
        "자극 없이 오래 두면 졸기가 나와야 한다"
    );
}

#[test]
fn 졸기_전까지는_움직이는_시간이_멈춰_있는_시간보다_길다() {
    let mut p = pet();
    let seen = drive(&mut p, 100, 120_000, 100, &world());
    let moving = seen
        .iter()
        .filter(|s| {
            matches!(s.behavior, Behavior::Walk | Behavior::Turn) || s.behavior.is_airborne()
        })
        .count();
    assert!(
        moving * 2 > seen.len(),
        "움직이는 비중이 절반을 넘어야 한다 (이동 {moving} / 전체 {})",
        seen.len()
    );
}

#[test]
fn 졸기_상태에서는_위치가_변하지_않는다() {
    let mut p = pet();
    let mut t = 100;
    while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
        p.step(t, &world());
        t += 250;
    }
    assert_eq!(p.behavior(), Behavior::Sleep, "졸기에 도달해야 한다");

    let x = p.snapshot().x;
    for _ in 0..20 {
        t += 250;
        p.step(t, &world());
        if p.behavior() != Behavior::Sleep {
            break;
        }
    }
    assert_eq!(p.snapshot().x, x, "자는 동안에는 움직이지 않는다");
    assert!(!Behavior::Sleep.moves_window(), "졸기는 창을 옮기지 않는다");
}

#[test]
fn 대사_추첨값은_배정밀도에서_안전한_범위다() {
    let mut p = pet();
    for i in 0..200u64 {
        p.say(1_000 + i);
        let roll = p.snapshot().speech.unwrap().roll;
        assert!(
            roll < (1u64 << 53),
            "배정밀도로 정확히 표현돼야 한다: {roll}"
        );
    }
}

#[test]
fn 같은_대사가_연달아_나와도_새_발화로_구분된다() {
    let mut p = pet();
    p.say(1_000);
    let first = p.snapshot().speech.unwrap();
    p.say(1_100);
    let second = p.snapshot().speech.unwrap();
    assert!(second.seq > first.seq, "발화 번호가 늘어야 한다");
}

#[test]
fn 말풍선은_시간이_지나면_사라진다() {
    let mut p = pet();
    p.say(1_000);
    assert!(
        p.step(1_500, &world()).speech.is_some(),
        "금방 사라지면 못 읽는다"
    );
    assert!(
        p.step(1_000 + SPEECH_MS + 100, &world()).speech.is_none(),
        "계속 떠 있으면 안 된다"
    );
}

#[test]
fn 가만_둬도_가끔_한마디_한다() {
    let mut p = pet();
    let seen = drive(&mut p, 100, 120_000, 100, &world());
    let spoke: std::collections::HashSet<u64> = seen
        .iter()
        .filter_map(|s| s.speech.map(|v| v.seq))
        .collect();
    assert!(
        spoke.len() >= 2,
        "2분 동안 한마디도 안 하면 심심하다 (실제 {})",
        spoke.len()
    );
}

fn pets_with_one() -> Pets {
    let mut pets = Pets::new();
    pets.add(1, 0, &world(), BOUNDS.left)
        .expect("첫 마리는 들어간다");
    pets
}

#[test]
fn 같은_순간에_태어나도_첫마디_시각이_다르다() {
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
    let b = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
    let first = |pets: &mut Pets, id| {
        let mut t = 0;
        while t < 60_000 {
            t += 100;
            if pets.get_mut(id).unwrap().step(t, &world()).speech.is_some() {
                return t;
            }
        }
        panic!("60초 안에 한마디도 안 했다");
    };
    assert_ne!(first(&mut pets, a), first(&mut pets, b));
}

#[test]
fn 펭귄을_추가하면_새_id를_받는다() {
    let mut pets = pets_with_one();
    let second = pets.add(1, 0, &world(), 300.0).expect("두 번째도 들어간다");
    assert_eq!(pets.len(), 2);
    assert!(pets.get(second).is_some());
}

#[test]
fn 지운_id는_다시_쓰이지_않는다() {
    let mut pets = pets_with_one();
    let second = pets.add(1, 0, &world(), 300.0).unwrap();
    assert!(pets.remove(second));
    let third = pets.add(1, 0, &world(), 300.0).unwrap();
    assert_ne!(
        second, third,
        "닫히는 중인 창과 새 창이 같은 라벨을 다투면 창 이동이 엉뚱한 쪽으로 간다"
    );
}

#[test]
fn 마지막_한_마리는_삭제되지_않는다() {
    let mut pets = pets_with_one();
    let only = pets.ids()[0];
    assert!(!pets.remove(only), "전부 없애는 것은 on/off의 일이다");
    assert_eq!(pets.len(), 1);
}

#[test]
fn 창이_사라진_펭귄은_마지막_한_마리여도_정리된다() {
    let mut pets = pets_with_one();
    let only = pets.ids()[0];
    pets.forget(only);
    assert!(pets.is_empty(), "창이 없는 펭귄은 사용자의 선택이 아니다");
}

#[test]
fn 상한을_넘겨_추가하면_거부된다() {
    let mut pets = Pets::new();
    for _ in 0..MAX_PETS {
        assert!(pets.add(1, 0, &world(), BOUNDS.left).is_some());
    }
    assert!(pets.add(1, 0, &world(), BOUNDS.left).is_none());
    assert_eq!(pets.len(), MAX_PETS);
}

#[test]
fn 마리마다_시드가_달라_다르게_움직인다() {
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
    let b = pets.add(7, 0, &world(), BOUNDS.left).unwrap();
    let mut diverged = false;
    let mut t = 0;
    while t < 60_000 && !diverged {
        t += 100;
        let sa = pets.get_mut(a).unwrap().step(t, &world());
        let sb = pets.get_mut(b).unwrap().step(t, &world());
        diverged = sa.x != sb.x || sa.behavior != sb.behavior;
    }
    assert!(diverged, "시드가 같으면 한 마리가 복제된 것처럼 보인다");
}

#[test]
fn 새_펭귄은_지정한_x에서_시작한다() {
    let mut pets = Pets::new();
    let id = pets.add(1, 0, &world(), 640.0).unwrap();
    assert_eq!(pets.get(id).unwrap().snapshot().x, 640.0);
}

#[test]
fn 시작_x는_영역_밖으로_나가지_않는다() {
    let mut pets = Pets::new();
    let id = pets.add(1, 0, &world(), BOUNDS.right + 5_000.0).unwrap();
    assert_eq!(pets.get(id).unwrap().snapshot().x, BOUNDS.right);
}

#[test]
fn 작업_영역이_바뀌면_다음_step에서_경계_안으로_들어온다() {
    let mut p = pet();
    p.x = 900.0;
    let narrow = Bounds {
        left: 0.0,
        right: 400.0,
        top: 0.0,
        floor_y: 600.0,
    };
    let s = p.step(1_000, &World::single(narrow));
    assert!(s.x <= narrow.right, "좁아진 영역 안으로 들어와야 한다");
    assert_eq!(s.y, narrow.floor_y, "바닥도 새 영역을 따른다");
}

#[test]
fn 기준점은_펭귄_발밑_중앙이다() {
    let mut p = pet();
    p.x = 300.0;
    p.y = 400.0;
    assert_eq!(p.anchor(), (300.0 + PET_SIZE / 2.0, 400.0 + PET_SIZE));
}

#[test]
fn 빈도_등급이_순서대로다() {
    let mut 기본 = 0; // Walk·Idle
    let mut 자주 = 0; // Swim
    let mut 가끔 = 0; // IceFishing
    for seed in 1u64..5 {
        let 전체 = 삼십분(seed);
        let mut 직전 = String::new();
        for s in 전체 {
            let 이름 = match s.behavior {
                Behavior::Walk | Behavior::Idle { .. } => "기본",
                Behavior::Swim => "자주",
                Behavior::IceFishing { .. } => "가끔",
                _ => "",
            }
            .to_string();
            if !이름.is_empty() && 이름 != 직전 {
                match 이름.as_str() {
                    "기본" => 기본 += 1,
                    "자주" => 자주 += 1,
                    _ => 가끔 += 1,
                }
            }
            직전 = 이름;
        }
    }
    assert!(기본 > 자주, "기본({기본})이 자주({자주})보다 잦아야 한다");
    assert!(자주 > 가끔, "자주({자주})가 가끔({가끔})보다 잦아야 한다");
}

#[test]
fn 희귀는_가끔보다_두_자릿수_드물다() {
    let 가끔 = ICE_FISHING_PERMILLE as f64 / 1_000.0;
    let 희귀 = 1.0 / FREAKOUT_ONE_IN as f64;
    assert!(
        가끔 / 희귀 >= 100.0,
        "발작(1/{})이 얼음낚시({}‰)보다 두 자릿수 드물지 않다",
        FREAKOUT_ONE_IN,
        ICE_FISHING_PERMILLE
    );
}

// ── 여러 마리를 한 자리에서 돌리기 ──

/// 한 번에 돌린 결과와 따로 돌린 결과를 **매 틱** 대조한다. 끝에서 한 번만 보면
/// 중간에 갈렸다가 우연히 붙은 경우를 놓치고, 갈린 시점도 알 수 없다.
/// `Snapshot` 전체를 비교한다 — 필드를 골라 비교하면 안 고른 필드가 조용히 갈린다.
fn 한_번에_돌린_것과_따로_돌린_것을_매_틱_대조한다(
    together: &mut Pets,
    apart: &mut Pets,
    w: &World,
    until_ms: u64,
) {
    let mut t = 0;
    while t < until_ms {
        t += 100;
        let 한_번에 = together.step_all(t, |_| Some(w));
        let 따로: Vec<(PetId, Snapshot)> = apart
            .ids()
            .into_iter()
            .map(|id| (id, apart.get_mut(id).unwrap().step(t, w)))
            .collect();
        assert_eq!(
            한_번에, 따로,
            "루프를 옮기는 리팩터링이므로 결과가 달라지면 안 된다 (t={t}ms)"
        );
    }
}

#[test]
fn 여러_마리를_한_번에_돌려도_따로_돌린_것과_같다() {
    let w = world();
    let mut together = Pets::new();
    let mut apart = Pets::new();
    for pets in [&mut together, &mut apart] {
        pets.add(7, 0, &w, BOUNDS.left).unwrap();
        pets.add(7, 0, &w, BOUNDS.left + 200.0).unwrap();
        pets.add(7, 0, &w, BOUNDS.left + 400.0).unwrap();
    }
    한_번에_돌린_것과_따로_돌린_것을_매_틱_대조한다(&mut together, &mut apart, &w, 1_800_000);
}

/// 위 테스트는 확률에 기대므로 **국면이 있는 동작이 창 안에 안 나타날 수 있다**
/// (얼음낚시 7‰, 발작 1/30000). 걸리는 것을 기다리지 않고 직접 걸어서 대조한다.
#[test]
fn 국면이_있는_동작이_섞여도_한_번에_돌린_결과가_같다() {
    let w = world();
    let mut together = Pets::new();
    let mut apart = Pets::new();
    for pets in [&mut together, &mut apart] {
        let 낚시 = pets.add(7, 0, &w, BOUNDS.left).unwrap();
        let 발작 = pets.add(7, 0, &w, BOUNDS.left + 150.0).unwrap();
        let 맞음 = pets.add(7, 0, &w, BOUNDS.left + 300.0).unwrap();
        let 들림 = pets.add(7, 0, &w, BOUNDS.left + 450.0).unwrap();
        assert!(pets.get_mut(낚시).unwrap().start_fishing(0));
        assert!(pets.get_mut(발작).unwrap().start_freakout(0));
        pets.get_mut(맞음).unwrap().whack(0, &w, 0.0, 1.0);
        pets.get_mut(들림).unwrap().drag_start(0);
    }
    // 얼음낚시 한 판이 30~60초라 90초면 정리까지 다 지나간다.
    한_번에_돌린_것과_따로_돌린_것을_매_틱_대조한다(&mut together, &mut apart, &w, 90_000);
}

#[test]
fn step_all은_id_오름차순으로_돈다() {
    let w = world();
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &w, BOUNDS.left).unwrap();
    let b = pets.add(7, 0, &w, BOUNDS.left + 100.0).unwrap();
    let c = pets.add(7, 0, &w, BOUNDS.left + 200.0).unwrap();

    let order: Vec<PetId> = pets
        .step_all(100, |_| Some(&w))
        .iter()
        .map(|(id, _)| *id)
        .collect();

    assert_eq!(order, vec![a, b, c], "창 이동 순서가 매 틱 달라지면 안 된다");
}

#[test]
fn 펭귄이_없으면_step_all은_빈_결과를_준다() {
    let w = world();
    let mut pets = Pets::new();
    assert!(pets.step_all(100, |_| Some(&w)).is_empty());
}

#[test]
fn 세계를_못_읽은_마리는_건너뛴다() {
    let w = world();
    let mut pets = Pets::new();
    let a = pets.add(7, 0, &w, BOUNDS.left).unwrap();
    let b = pets.add(7, 0, &w, BOUNDS.left + 200.0).unwrap();

    let before = pets.get(b).unwrap().snapshot();
    let stepped = pets.step_all(100, |id| (id == a).then_some(&w));

    assert_eq!(stepped.len(), 1, "세계가 없으면 그 마리는 이번 틱을 쉰다");
    assert_eq!(stepped[0].0, a);
    let after = pets.get(b).unwrap().snapshot();
    assert_eq!((before.x, before.y), (after.x, after.y));
}

// ── 비치발리볼 판의 수명 ───────────────────────────────────────

/// 1440px 화면쯤의 넓은 경계 — 코트가 들어간다.
const 넓은_경계: Bounds = Bounds {
    left: 52.0,
    right: 1_248.0,
    top: 80.0,
    floor_y: 700.0,
};

fn 여러_마리(n: usize) -> Pets {
    let w = World::single(넓은_경계);
    let mut pets = Pets::new();
    for i in 0..n {
        pets.add(1_000 + i as u64, 0, &w, 넓은_경계.left + i as f64 * 60.0)
            .expect("상한 안이다");
    }
    pets
}

/// 판이 끝날 때까지 굴린다. 반환은 (끝난 시각, 마지막까지 본 국면들).
fn 판을_굴린다(pets: &mut Pets, world: &World, 최대_ms: u64) -> u64 {
    let mut now = 0u64;
    while now < 최대_ms {
        now += 50;
        pets.step_all(now, |_| Some(world));
        if pets.volleyball().is_none() {
            return now;
        }
    }
    panic!("판이 {최대_ms}ms 안에 안 끝났다");
}

#[test]
fn 한_마리면_판을_열지_않는다() {
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(1);
    let 전 = pets.get(1).unwrap().snapshot().behavior;
    assert_eq!(
        pets.start_volleyball(0, 넓은_경계, 7),
        Err(VolleyRefusal::TooFew)
    );
    assert!(pets.volleyball().is_none());
    assert_eq!(pets.get(1).unwrap().snapshot().behavior, 전, "동작이 바뀌었다");
    let _ = w;
}

#[test]
fn 홀수면_판을_열지_않는다() {
    // 팀이 갈리지 않으면 한쪽이 덜 뛰고, "누가 받으러 뛰는가"라는 이 판의 유일한
    // 볼거리가 한쪽으로 기운다. 세 마리·다섯 마리·일곱 마리 전부 거절한다.
    for n in [3usize, 5, 7] {
        let mut pets = 여러_마리(n);
        let 전: Vec<_> = pets
            .ids()
            .iter()
            .map(|id| pets.get(*id).unwrap().snapshot().behavior)
            .collect();
        assert_eq!(
            pets.start_volleyball(0, 넓은_경계, 7),
            Err(VolleyRefusal::Odd),
            "{n}마리인데 판이 열렸다"
        );
        assert!(pets.volleyball().is_none());
        let 후: Vec<_> = pets
            .ids()
            .iter()
            .map(|id| pets.get(*id).unwrap().snapshot().behavior)
            .collect();
        assert_eq!(전, 후, "{n}마리: 거절했는데 동작이 바뀌었다");
    }
}

#[test]
fn 짝수면_판이_열린다() {
    // 홀수 거절이 짝수까지 막지 않는지 — 상한(8)까지 본다.
    for n in [2usize, 4, 6, 8] {
        let mut pets = 여러_마리(n);
        assert_eq!(
            pets.start_volleyball(0, 넓은_경계, 7),
            Ok(()),
            "{n}마리인데 판이 안 열렸다"
        );
        assert!(pets.volleyball().is_some());
    }
}

#[test]
fn 한_마리는_홀수가_아니라_모자란_것이다() {
    // 사용자에게 "짝수로 맞춰라"가 아니라 "둘부터"가 맞는 설명이다.
    let mut pets = 여러_마리(1);
    assert_eq!(
        pets.start_volleyball(0, 넓은_경계, 7),
        Err(VolleyRefusal::TooFew)
    );
}

#[test]
fn 두_마리면_판이_열린다() {
    let mut pets = 여러_마리(2);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    assert!(pets.volleyball().is_some());
    for id in pets.ids() {
        assert!(matches!(
            pets.get(id).unwrap().snapshot().behavior,
            Behavior::Volleyball { .. }
        ));
    }
}

#[test]
fn 좁은_화면에서는_판을_열_수_없다() {
    let 좁은 = Bounds {
        left: 0.0,
        right: 200.0,
        top: 0.0,
        floor_y: 400.0,
    };
    let mut pets = 여러_마리(4);
    assert_eq!(
        pets.start_volleyball(0, 좁은, 7),
        Err(VolleyRefusal::NoRoom)
    );
}

#[test]
fn 이미_판이_돌면_다시_열지_않는다() {
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    assert_eq!(
        pets.start_volleyball(100, 넓은_경계, 9),
        Err(VolleyRefusal::BoardBusy)
    );
}

#[test]
fn 볼링이_도는_중에는_비치발리볼을_못_연다() {
    let mut pets = 여러_마리(4);
    assert!(pets.start_bowling(0, 넓은_경계));
    assert_eq!(
        pets.start_volleyball(100, 넓은_경계, 7),
        Err(VolleyRefusal::BoardBusy)
    );
}

#[test]
fn 비치발리볼이_도는_중에는_볼링을_못_연다() {
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    assert!(!pets.start_bowling(100, 넓은_경계));
}

#[test]
fn 참여_마리가_둘_미만이_되면_판이_접힌다() {
    let w = World::single(넓은_경계);
    // **짝수로 연다** — 홀수는 판이 아예 안 열린다(`홀수면_판을_열지_않는다`).
    // 접히는 것은 **연 뒤에** 마리가 빠지는 이야기라 여는 조건과 별개다.
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    // 넷 중 셋을 집어 들면 하나만 남는다.
    for id in [1u32, 2, 3] {
        pets.get_mut(id).unwrap().drag_start(100);
    }
    pets.step_all(150, |_| Some(&w));
    assert!(pets.volleyball().is_none(), "혼자 남았는데 판이 살아 있다");
}

#[test]
fn 드래그로_빠진_마리는_참여_목록에서_빠진다() {
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    pets.get_mut(2).unwrap().drag_start(100);
    pets.step_all(150, |_| Some(&w));
    let board = pets.volleyball().expect("셋이 남았으니 판은 산다");
    assert!(!board.participants().contains(&2));
    assert_eq!(board.participants().len(), 3);
}

#[test]
fn 펭귄을_지우면_판에서도_빠진다() {
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    assert!(pets.remove(2));
    assert!(!pets.volleyball().unwrap().participants().contains(&2));
    // 창이 사라진 경우(`forget`)도 같다.
    pets.forget(3);
    assert!(!pets.volleyball().unwrap().participants().contains(&3));
}

#[test]
fn 펭귄을_전부_끄면_판이_사라진다() {
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    pets.clear();
    assert!(pets.volleyball().is_none());
}

#[test]
fn 판을_강제로_접으면_전부_평소로_돌아간다() {
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    pets.end_volleyball(100);
    assert!(pets.volleyball().is_none());
    // 귀결 국면을 지나 유휴로 간다.
    let mut now = 100;
    for _ in 0..40 {
        now += 50;
        pets.step_all(now, |_| Some(&w));
    }
    for id in pets.ids() {
        assert!(
            !matches!(
                pets.get(id).unwrap().snapshot().behavior,
                Behavior::Volleyball { .. }
            ),
            "{id}번이 아직 코트에 있다"
        );
    }
}

#[test]
fn 한_판이_스스로_돌고_끝난다() {
    // 버튼 한 번으로 시작해 20초쯤 뒤 아무도 안 건드려도 끝난다.
    let w = World::single(넓은_경계);
    for seed in 1u64..=5 {
        let mut pets = 여러_마리(4);
        assert_eq!(pets.start_volleyball(0, 넓은_경계, seed), Ok(()));
        let 끝난 = 판을_굴린다(&mut pets, &w, 60_000);
        assert!(
            (15_000..=28_000).contains(&끝난),
            "시드 {seed}: {끝난}ms — 20초쯤이 아니다"
        );
        for id in pets.ids() {
            let b = pets.get(id).unwrap().snapshot().behavior;
            assert!(
                !matches!(b, Behavior::Volleyball { volley: VolleyPhase::Gather | VolleyPhase::Ready | VolleyPhase::Chase | VolleyPhase::Bump }),
                "{id}번이 랠리 국면에 남았다: {b:?}"
            );
        }
    }
}

#[test]
fn 랠리_중에_받으러_뛰는_마리가_생긴다() {
    // KTD3-1 — 뛰는 그림이 랠리 화면의 절반이다. 안 뛰면 20초가 통째로 빈다.
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 3), Ok(()));
    let mut 뛴_적 = false;
    let mut 때린_적 = false;
    let mut now = 0u64;
    while now < 40_000 && pets.volleyball().is_some() {
        now += 50;
        pets.step_all(now, |_| Some(&w));
        for id in pets.ids() {
            match pets.get(id).unwrap().snapshot().behavior {
                Behavior::Volleyball {
                    volley: VolleyPhase::Chase,
                } => 뛴_적 = true,
                Behavior::Volleyball {
                    volley: VolleyPhase::Bump,
                } => 때린_적 = true,
                _ => {}
            }
        }
    }
    assert!(뛴_적, "한 번도 안 뛰었다");
    assert!(때린_적, "한 번도 안 때렸다");
}

#[test]
fn 득점하면_이긴_쪽과_진_쪽이_갈린다() {
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(4);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 3), Ok(()));
    let mut now = 0u64;
    let (mut 좋아함, mut 약오름) = (false, false);
    while now < 40_000 && pets.volleyball().is_some() {
        now += 50;
        pets.step_all(now, |_| Some(&w));
        for id in pets.ids() {
            match pets.get(id).unwrap().snapshot().behavior {
                Behavior::Volleyball {
                    volley: VolleyPhase::Cheer,
                } => 좋아함 = true,
                Behavior::Volleyball {
                    volley: VolleyPhase::Sulk,
                } => 약오름 = true,
                _ => {}
            }
        }
    }
    assert!(좋아함 && 약오름, "이긴 쪽({좋아함})과 진 쪽({약오름})이 안 갈렸다");
}

#[test]
fn 같은_시드는_같은_판을_낳는다() {
    // 매 틱 전체 스냅샷을 대조한다 (PRINCIPLE 3).
    let w = World::single(넓은_경계);
    let mut a = 여러_마리(4);
    let mut b = 여러_마리(4);
    assert_eq!(a.start_volleyball(0, 넓은_경계, 0xC0FFEE), Ok(()));
    assert_eq!(b.start_volleyball(0, 넓은_경계, 0xC0FFEE), Ok(()));
    for t in 1..=500u64 {
        let now = t * 50;
        assert_eq!(
            a.step_all(now, |_| Some(&w)),
            b.step_all(now, |_| Some(&w)),
            "{now}ms 에서 갈렸다"
        );
    }
}

#[test]
fn 판을_못_열면_마리의_동작이_한_틱도_안_바뀐다() {
    // 거절이 부작용을 남기면 "눌렀는데 아무 일도 없다"가 아니라 "눌렀더니
    // 이상해졌다"가 된다.
    let 좁은 = Bounds {
        left: 0.0,
        right: 200.0,
        top: 0.0,
        floor_y: 400.0,
    };
    let w = World::single(좁은);
    let mut 누른 = 여러_마리(2);
    let mut 안_누른 = 여러_마리(2);
    assert_eq!(누른.start_volleyball(0, 좁은, 7), Err(VolleyRefusal::NoRoom));
    for t in 1..=200u64 {
        let now = t * 50;
        assert_eq!(
            누른.step_all(now, |_| Some(&w)),
            안_누른.step_all(now, |_| Some(&w)),
            "{now}ms 에서 갈렸다 — 거절이 부작용을 남겼다"
        );
    }
}

#[test]
fn 판이_접혀도_남은_마리는_평소로_돌아간다() {
    // 두 마리 판에서 하나를 끌어내면 남은 하나가 `Ready`에 갇힌다 — 그 국면의
    // 시각은 국면 길이가 아니라 **안전 상한**(60초)이라, 코트도 공도 사라진
    // 바탕화면에 비키니만 입은 채 1분을 서 있게 된다.
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(2);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    pets.get_mut(1).unwrap().drag_start(100);
    pets.step_all(150, |_| Some(&w));
    assert!(pets.volleyball().is_none(), "혼자 남았으면 판이 접힌다");

    let 남은 = pets.get(2).unwrap().snapshot().behavior;
    assert!(
        !matches!(
            남은,
            Behavior::Volleyball {
                volley: VolleyPhase::Gather
                    | VolleyPhase::Ready
                    | VolleyPhase::Chase
                    | VolleyPhase::Bump
            }
        ),
        "판이 사라졌는데 남은 마리가 랠리 국면에 갇혔다: {남은:?}"
    );
}

#[test]
fn 펭귄을_지워서_판이_접혀도_남은_마리가_안_갇힌다() {
    // `leave_volleyball` 경로도 같다 — 삭제·창 소실 둘 다 여기를 지난다.
    // **접는 일은 다음 틱이 한다** (`leave_volleyball` 문서 참고): 남은 마리를
    // 풀어 주려면 시각이 필요한데 `Pets`는 시계를 갖지 않는다. 노출은 한 틱이다.
    let w = World::single(넓은_경계);
    let mut pets = 여러_마리(2);
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 7), Ok(()));
    assert!(pets.remove(1));
    pets.step_all(50, |_| Some(&w));
    assert!(pets.volleyball().is_none(), "다음 틱에 판이 접힌다");
    let 남은 = pets.get(2).unwrap().snapshot().behavior;
    assert!(
        !matches!(
            남은,
            Behavior::Volleyball {
                volley: VolleyPhase::Gather
                    | VolleyPhase::Ready
                    | VolleyPhase::Chase
                    | VolleyPhase::Bump
            }
        ),
        "판이 사라졌는데 남은 마리가 랠리 국면에 갇혔다: {남은:?}"
    );
}

#[test]
fn 핀볼_충돌은_비치발리볼_판이_도는_동안_쉰다() {
    // **판이 도는 동안 마리는 판이 몬다** (`step_bowling`의 KTD8과 같은 규칙).
    // 핀볼 충돌 반경(104px)이 코트 이웃 간격보다 좁고 받을 마리는 그 사이를
    // 가로질러 뛰므로, 쉬지 않으면 `bumped`가 `Thrown`으로 넘겨 **랠리가
    // 몇 초 만에 찢어진다.** 볼링이 같은 이유로 같은 가드를 갖는다.
    let w = World::single(넓은_경계);
    // **여덟 마리라야 드러난다** — 넷이면 코트 간격이 350px라 반경 104px에
    // 한 번도 안 걸린다. 최대 마릿수에서 간격이 117px로 좁아지고, 받을 마리가
    // 그 사이를 뛰어 지나간다.
    let mut pets = 여러_마리(8);
    for id in pets.ids() {
        pets.get_mut(id).unwrap().set_pinball(true);
    }
    assert_eq!(pets.start_volleyball(0, 넓은_경계, 3), Ok(()));

    let mut now = 0u64;
    while now < 25_000 && pets.volleyball().is_some() {
        now += 50;
        pets.step_all(now, |_| Some(&w));
        if let Some(board) = pets.volleyball() {
            // **한 마리도 안 빠져야 한다.** `>= 2`로 두면 여덟이 둘로 줄어도
            // 통과해 찢어진 것을 놓친다. 여기서 마리를 뺄 다른 경로는 없다.
            //
            // 득점(`Point`)은 예외다 — 그때는 전원이 축하·약오름으로 넘어가면서
            // 참여 목록이 비는 것이 **정상 동작**이다.
            if board.phase() != CourtPhase::Point {
                assert_eq!(
                    board.participants().len(),
                    8,
                    "{now}ms: 핀볼 충돌이 랠리를 찢었다"
                );
            }
        }
    }
    assert!(now < 25_000, "판이 안 끝났다");
}
