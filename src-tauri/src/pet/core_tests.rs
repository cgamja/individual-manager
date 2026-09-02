use crate::pet::test_support::*;
use crate::pet::*;

use super::test_support::*;
use super::*;

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
