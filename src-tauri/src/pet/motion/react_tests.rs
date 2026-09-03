use crate::pet::test_support::*;
use crate::pet::*;

#[test]
fn 휘둘러도_날아가지_않는다() {
    let mut p = pet();
    p.step(1_000, &world());
    let before = p.snapshot();
    p.whack(1_000, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing, "클릭하면 바로 휘두른다");

    let mut t = 1_000;
    for _ in 0..30 {
        t += 50;
        let s = p.step(t, &world());
        assert_eq!(s.x, before.x, "옆으로 밀리면 안 된다");
        assert_eq!(s.y, before.y, "떠오르면 안 된다");
        assert_ne!(s.behavior, Behavior::Thrown, "던져진 상태가 되면 안 된다");
    }
}

#[test]
fn 휘두르고_나면_약을_올린다() {
    let mut p = pet();
    p.step(1_000, &world());
    p.whack(1_000, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
    let after = p.step(1_000 + SWING_MS + 20, &world());
    assert!(
        matches!(after.behavior, Behavior::Sassy { .. }),
        "휘두르고 나면 약이 올라야 한다 (실제: {:?})",
        after.behavior
    );
}

#[test]
fn 빠따는_한_번에_한_번씩_횟수가_는다() {
    let mut p = pet();
    assert_eq!(p.snapshot().whack_seq, 0);
    for i in 1..=5u64 {
        p.whack(1_000 + i * 100, &world(), 0.0, 0.0);
        assert_eq!(p.snapshot().whack_seq, i, "{i}번째 빠따가 안 세어졌다");
    }
}

#[test]
fn 던져서_나는_중에_휘둘러도_그_자리에서_마저_떨어진다() {
    let mut p = pet();
    p.drag_start(1_000);
    p.drag_by(0.0, -300.0);
    p.step(1_050, &world());
    p.drag_end(1_100, 600.0, -400.0, &world());
    assert_eq!(p.behavior(), Behavior::Thrown);

    let mut t = 1_100;
    for _ in 0..4 {
        t += 50;
        p.step(t, &world());
    }
    assert_eq!(p.behavior(), Behavior::Thrown, "아직 나는 중이어야 한다");
    assert!(p.snapshot().air, "공중 상태여야 한다");

    p.whack(t, &world(), 0.0, 0.0);
    let hit_y = p.snapshot().y;
    assert_eq!(p.behavior(), Behavior::Swing);
    t += 50;
    let swinging = p.step(t, &world());
    assert_eq!(
        swinging.y, hit_y,
        "휘두른다고 솟아오르거나 떨어지면 안 된다"
    );

    let after = p.step(t + SWING_MS + 20, &world());
    assert_eq!(
        after.behavior,
        Behavior::Falling,
        "공중이었으니 마저 떨어진다"
    );
}

#[test]
fn 빠따는_졸고_있어도_깨운다() {
    let mut p = pet();
    let mut t = 100;
    while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
        p.step(t, &world());
        t += 250;
    }
    assert_eq!(p.behavior(), Behavior::Sleep);
    p.whack(t, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
}

#[test]
fn 휘두른다고_말하지는_않는다() {
    let mut p = pet();
    p.whack(1_000, &world(), 0.0, 0.0);
    assert!(
        p.snapshot().speech.is_none(),
        "클릭으로 말이 나오면 안 된다"
    );
    p.whack(1_100, &world(), 0.0, 0.0);
    p.whack(1_200, &world(), 0.0, 0.0);
    assert!(p.snapshot().speech.is_none(), "연타해도 마찬가지다");
}

/// 연타로 빽빽거리게 만든 펭귄과 터진 시각.
fn 빽빽거리는_펭귄() -> (Pet, u64) {
    let mut p = pet();
    p.step(1_000, &world());
    let mut t = 1_000;
    for _ in 0..SQUAWK_WHACK_COUNT {
        t += 150;
        클릭(&mut p, t);
    }
    assert_eq!(p.behavior(), Behavior::Squawk, "연타로 터져야 한다");
    (p, t)
}

#[test]
fn 짧은_간격으로_스무_번_맞으면_빽빽거린다() {
    let mut p = pet();
    p.step(1_000, &world());
    let mut t = 1_000;
    for i in 1..=SQUAWK_WHACK_COUNT {
        t += 150;
        클릭(&mut p, t);
        if i < SQUAWK_WHACK_COUNT {
            assert_eq!(p.behavior(), Behavior::Swing, "{i}번째까지는 휘두른다");
        }
    }
    assert_eq!(
        p.behavior(),
        Behavior::Squawk,
        "문턱을 넘은 클릭에서 터진다"
    );
}

#[test]
fn 띄엄띄엄_때리면_빽빽거리지_않는다() {
    let mut p = pet();
    let mut t = 1_000;
    for _ in 0..6 {
        t += SQUAWK_GAP_MS + 500;
        클릭(&mut p, t);
        assert_eq!(
            p.behavior(),
            Behavior::Swing,
            "간격이 벌어지면 그냥 휘두른다"
        );
    }
}

#[test]
fn 문턱_직전까지는_안_터지고_한_번_더_때리면_터진다() {
    let mut p = pet();
    let mut t = 300;
    for _ in 1..SQUAWK_WHACK_COUNT {
        클릭(&mut p, t);
        t += 100;
    }
    assert_eq!(p.behavior(), Behavior::Swing, "문턱 직전까지는 휘두른다");
    클릭(&mut p, t);
    assert_eq!(p.behavior(), Behavior::Squawk, "한 번 더 때리면 터진다");
}

#[test]
fn 빽빽거리는_중에_맞아도_끊기지_않는다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    클릭(&mut p, t + 200);
    assert_eq!(p.behavior(), Behavior::Squawk, "스윙으로 끊기면 안 된다");
    클릭(&mut p, t + 400);
    assert_eq!(p.behavior(), Behavior::Squawk);
    let mid = p.step(t + 400 + SQUAWK_MS - 50, &world());
    assert_eq!(mid.behavior, Behavior::Squawk, "새 판이 아직 안 끝났다");
    let after = p.step(t + 400 + SQUAWK_MS + 20, &world());
    assert_ne!(
        after.behavior,
        Behavior::Squawk,
        "손을 떼면 제 시간에 끝난다"
    );
}

#[test]
fn 빽빽거리는_중에_맞은_것은_다음_연타로_세지_않는다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    for i in 1..=3 {
        클릭(&mut p, t + i * 100);
    }
    let end = t + 300 + SQUAWK_MS + 20;
    p.step(end, &world());
    클릭(&mut p, end + 40);
    assert_eq!(p.behavior(), Behavior::Swing, "카운터가 초기화돼야 한다");
}

#[test]
fn 빽빽거리는_동안_제자리에_있다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    let before = p.snapshot();
    let mut now = t;
    for _ in 0..10 {
        now += 50;
        let s = p.step(now, &world());
        assert_eq!(s.x, before.x, "옆으로 움직이면 안 된다");
        assert_eq!(s.y, before.y, "떠오르거나 가라앉으면 안 된다");
    }
}

#[test]
fn 빽빽거리기가_끝나면_유휴로_간다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    let after = p.step(t + SQUAWK_MS + 20, &world());
    assert!(
        matches!(after.behavior, Behavior::Idle { .. }),
        "유휴로 나가야 한다 (실제: {:?})",
        after.behavior
    );
}

#[test]
fn 공중에서_빽빽거리면_끝나고_떨어진다() {
    let mut p = pet();
    p.drag_start(1_000);
    p.drag_by(0.0, -300.0);
    p.step(1_050, &world());
    p.drag_end(1_100, 0.0, 0.0, &world());
    assert!(p.snapshot().air, "공중이어야 한다");

    let mut t = 1_100;
    for _ in 0..SQUAWK_WHACK_COUNT {
        t += 150;
        p.whack(t, &world(), 0.0, 0.0);
    }
    assert_eq!(p.behavior(), Behavior::Squawk, "공중에서도 터진다");
    assert!(p.snapshot().air, "고도를 물려받아야 한다");

    let after = p.step(t + SQUAWK_MS + 20, &world());
    assert_eq!(
        after.behavior,
        Behavior::Falling,
        "공중이었으니 마저 떨어진다"
    );
}

#[test]
fn 빽빽거리다_던져지면_되돌아오지_않는다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    p.drag_start(t + 100);
    p.drag_by(120.0, -80.0);
    p.drag_end(t + 200, 900.0, -600.0, &world());
    assert_eq!(p.behavior(), Behavior::Thrown, "던져진 상태여야 한다");

    클릭(&mut p, t + 300);
    assert_ne!(p.behavior(), Behavior::Squawk, "예산은 나가는 순간 무효다");
}

#[test]
fn 빽빽거리는_중에_들어_올릴_수_있다() {
    let (mut p, t) = 빽빽거리는_펭귄();
    p.drag_start(t + 100);
    assert_eq!(p.behavior(), Behavior::Dragged);
}

#[test]
fn 빽빽거리기는_제자리_동작이다() {
    assert!(!Behavior::Squawk.is_airborne(), "스스로 뜨지 않는다");
    assert!(
        !Behavior::Squawk.is_landing(),
        "바닥에 닿아서 생긴 게 아니다"
    );
    assert!(Behavior::Squawk.moves_window(), "틱을 빠르게 유지해야 한다");
}

#[test]
fn 시키면_바로_빽빽거린다() {
    let mut p = pet();
    p.step(1_000, &world());
    assert!(p.start_squawk(1_000));
    assert_eq!(p.behavior(), Behavior::Squawk);
}

#[test]
fn 공중에서도_시키면_빽빽거린다() {
    let mut p = pet();
    p.drag_start(1_000);
    p.drag_by(0.0, -300.0);
    p.step(1_050, &world());
    p.drag_end(1_100, 0.0, 0.0, &world());
    assert!(p.start_squawk(1_150));
    assert_eq!(p.behavior(), Behavior::Squawk);
    assert!(p.snapshot().air, "바닥으로 끌어내리면 순간이동한다");
}

#[test]
fn 들려_있거나_이미_빽빽거리면_시켜도_안_한다() {
    let mut p = pet();
    p.drag_start(1_000);
    assert!(!p.start_squawk(1_050), "손에 쥔 채로는 안 된다");

    let (mut q, t) = 빽빽거리는_펭귄();
    assert!(!q.start_squawk(t + 100), "재진입하면 웹뷰가 되감지 못한다");
}

// ── 스윙 넉백 ──────────────────────────────────────────────────
//
// 휘두른 방망이가 **앞에 있는 다른 마리**를 날린다. 판정은 `Pets`에 있고
// (`Pet::whack`은 자기 자신만 본다) 난수를 쓰지 않는다.

/// 마리 둘을 원하는 x에 세운다. 둘 다 바닥에 서서 오른쪽을 본다.
fn 두_마리(a_x: f64, b_x: f64) -> (Pets, PetId, PetId) {
    let mut pets = Pets::new();
    let a = pets.add(1, 0, &world(), a_x).expect("첫 마리");
    let b = pets.add(2, 0, &world(), b_x).expect("둘째 마리");
    (pets, a, b)
}

fn 동작(pets: &Pets, id: PetId) -> Behavior {
    pets.get(id).expect("있는 마리").behavior()
}

#[test]
fn 방망이에_맞으면_앞으로_날아간다() {
    let mut p = pet();
    let before = p.snapshot();
    p.swing_knocked(1_000, 1.0, 1_000.0);
    assert_eq!(p.behavior(), Behavior::Thrown, "맞으면 날아간다");

    let mut t = 1_000;
    for _ in 0..4 {
        t += 50;
        p.step(t, &world());
    }
    assert!(p.snapshot().x > before.x, "앞으로 나가야 한다");
}

#[test]
fn 방망이에_맞으면_살짝_떠오른다() {
    let mut p = pet();
    // **첫 틱을 짧게 만든다.** 태어난 직후 1초짜리 틱을 돌리면 중력이 바닥을 한 번
    // 치고 그 반동만으로 떠올라, 솟는 힘이 0이어도 통과하는 시험이 된다.
    p.step(980, &world());
    let before = p.snapshot();
    p.swing_knocked(1_000, 1.0, 1_000.0);
    let s = p.step(1_020, &world());
    assert!(
        s.y < before.y,
        "첫 틱부터 떠야 한다 (before={}, after={})",
        before.y,
        s.y
    );
}

#[test]
fn 넓은_세계에서_맞으면_더_멀리_날아간다() {
    // 고정 px/s로 두면 화면이 넓어질수록 "안 날아간 것"처럼 보인다 (던지기와 같은 근거).
    let 한_틱 = |width: f64| {
        let mut p = pet();
        p.step(980, &world());
        let x0 = p.snapshot().x;
        p.swing_knocked(1_000, 1.0, width);
        p.step(1_020, &world()).x - x0
    };
    assert!(
        한_틱(3_000.0) > 한_틱(1_000.0),
        "세계가 넓으면 같은 스윙이 더 멀리 보낸다"
    );
}

#[test]
fn 맞은_쪽은_방망이를_휘두르지_않는다() {
    let mut p = pet();
    let seq = p.snapshot().whack_seq;
    p.swing_knocked(1_000, 1.0, 1_000.0);
    assert_eq!(
        p.snapshot().whack_seq,
        seq,
        "맞는 쪽이 휘두르면 방망이가 두 개 보인다"
    );
}

#[test]
fn 앞에_있는_마리도_같이_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    let 맞은 = pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_eq!(동작(&pets, a), Behavior::Swing, "때린 마리는 휘두른다");
    assert_eq!(동작(&pets, b), Behavior::Thrown, "앞 마리가 날아가야 한다");
    assert_eq!(맞은, vec![a, b], "겉모습이 바뀐 마리를 전부 돌려준다");
}

#[test]
fn 등_뒤의_마리는_안_날아간다() {
    // 둘 다 오른쪽을 보므로 b에게 a는 등 뒤다.
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    let 맞은 = pets.whack(b, 1_000, &world(), 0.0, 0.0);
    assert_eq!(동작(&pets, b), Behavior::Swing);
    assert_ne!(동작(&pets, a), Behavior::Thrown, "등 뒤는 안 맞는다");
    assert_eq!(맞은, vec![b]);
}

#[test]
fn 왼쪽을_보면_왼쪽_앞의_마리가_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    let c = pets.add(3, 0, &world(), 580.0).expect("셋째 마리");
    // 방향에는 세터가 없다 — 코어 안이라 필드를 그대로 세운다. 걷다가 돌기를
    // 기다리면 시드에 딸린 시험이 된다.
    pets.get_mut(b).unwrap().facing = Facing::Left;

    let 맞은 = pets.whack(b, 1_000, &world(), 0.0, 0.0);
    assert_eq!(
        동작(&pets, a),
        Behavior::Thrown,
        "왼쪽 앞의 마리가 날아간다"
    );
    assert_ne!(동작(&pets, c), Behavior::Thrown, "오른쪽은 이제 등 뒤다");
    assert_eq!(맞은, vec![b, a], "때린 마리 먼저, 나머지는 id 오름차순");

    let 날아간 = pets.get_mut(a).unwrap().step(1_020, &world());
    assert!(
        날아간.x < 300.0,
        "보는 쪽으로 날아가야 한다 (실제: {})",
        날아간.x
    );
}

#[test]
fn 사거리_안의_이웃은_전부_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 400.0);
    let c = pets
        .add(3, 0, &world(), 300.0 + SWING_REACH)
        .expect("셋째 마리");
    let 맞은 = pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_eq!(동작(&pets, b), Behavior::Thrown);
    assert_eq!(
        동작(&pets, c),
        Behavior::Thrown,
        "가운데끼리 정확히 사거리면 닿는다"
    );
    assert_eq!(맞은, vec![a, b, c], "때린 마리 먼저, 나머지는 id 오름차순");
}

#[test]
fn 사거리_밖의_마리는_안_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 300.0 + SWING_REACH + 60.0);
    let 맞은 = pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_ne!(동작(&pets, b), Behavior::Thrown, "방망이가 안 닿는다");
    assert_eq!(맞은, vec![a], "안 날아간 마리를 돌려주면 창만 헛돈다");
}

#[test]
fn 없는_마리를_때리면_아무_일도_없다() {
    // 클릭과 커맨드 사이에 창이 닫히면 브릿지가 이 길로 온다.
    let mut pets = Pets::new();
    assert!(pets.whack(99, 1_000, &world(), 0.0, 0.0).is_empty());
}

#[test]
fn 한_층_위의_마리는_안_맞는다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    {
        let 위 = pets.get_mut(b).unwrap();
        위.drag_start(900);
        위.drag_by(0.0, -(SWING_REACH_V + 80.0));
        위.drag_end(900, 0.0, 0.0, &world());
    }
    pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_eq!(
        동작(&pets, b),
        Behavior::Falling,
        "머리 위를 지나는 마리까지 맞으면 방망이가 아니라 장판이다"
    );
}

#[test]
fn 날아간_마리가_또_다른_마리를_치지_않는다() {
    // a—b는 사거리 안, b—c도 사거리 안, a—c는 사거리 밖이다.
    let (mut pets, a, b) = 두_마리(300.0, 480.0);
    let c = pets.add(3, 0, &world(), 640.0).expect("셋째 마리");
    pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_eq!(동작(&pets, b), Behavior::Thrown, "앞 마리는 날아간다");
    assert_ne!(동작(&pets, c), Behavior::Thrown, "연쇄는 넣지 않았다");
}

#[test]
fn 들고_있는_마리는_안_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    pets.get_mut(b).unwrap().drag_start(900);
    pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert_eq!(
        동작(&pets, b),
        Behavior::Dragged,
        "손과 방망이가 다투면 손이 이긴다"
    );
}

#[test]
fn 볼링_핀은_방망이에_안_맞는다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    assert!(pets.start_bowling(900, BOUNDS), "판이 열려야 한다");
    pets.whack(a, 1_000, &world(), 0.0, 0.0);
    assert!(
        matches!(동작(&pets, b), Behavior::Bowling { .. }),
        "핀을 넘어뜨리는 것은 공의 일이다 (실제: {:?})",
        동작(&pets, b)
    );
}

#[test]
fn 핀볼_모드에서는_이웃이_안_날아간다() {
    let (mut pets, a, b) = 두_마리(300.0, 440.0);
    for id in [a, b] {
        pets.get_mut(id).unwrap().set_pinball(true);
    }
    let 맞은 = pets.whack(a, 1_000, &world(), 0.0, -0.5);
    assert_eq!(
        동작(&pets, a),
        Behavior::Thrown,
        "채에 맞아 자기가 날아간다"
    );
    assert_ne!(동작(&pets, b), Behavior::Thrown, "핀볼에서는 방망이가 없다");
    assert_eq!(맞은, vec![a]);
}

#[test]
fn 연타로_빽빽거리면_이웃이_안_날아간다() {
    let mut pets = Pets::new();
    let a = pets.add(1, 0, &world(), 300.0).expect("첫 마리");
    let mut t = 1_000;
    for _ in 1..SQUAWK_WHACK_COUNT {
        pets.whack(a, t, &world(), 0.0, 0.0);
        t += 100;
    }
    let b = pets.add(2, t, &world(), 440.0).expect("둘째 마리");

    pets.whack(a, t, &world(), 0.0, 0.0);
    assert_eq!(동작(&pets, a), Behavior::Squawk, "스무 번째에 터진다");
    assert_ne!(
        동작(&pets, b),
        Behavior::Thrown,
        "빽빽거리는 클릭은 방망이를 휘두르지 않는다"
    );
}

#[test]
fn 맞은_이웃도_착지_등급을_그대로_탄다() {
    // 세계가 넓으면 넉백도 세다 — 세게 떨어지면 철푸덕이 나와야 한다.
    // 저절로 나는 철푸덕과 달리 **사용자가 방금 만든 결과**다.
    let 넓은 = World::single(Bounds {
        right: 2_000.0,
        ..BOUNDS
    });
    let mut pets = Pets::new();
    let a = pets.add(1, 0, &넓은, 300.0).expect("첫 마리");
    let b = pets.add(2, 0, &넓은, 440.0).expect("둘째 마리");
    pets.whack(a, 1_000, &넓은, 0.0, 0.0);

    let 자취 = drive(pets.get_mut(b).unwrap(), 1_020, 9_000, 20, &넓은);
    assert!(
        자취
            .iter()
            .any(|s| matches!(s.behavior, Behavior::Splat | Behavior::Sprawl)),
        "세게 날아간 이웃은 철푸덕 널브러진다"
    );
}

#[test]
fn 방망이에_맞으면_졸음이_풀린다() {
    let mut p = pet();
    p.step(1_000, &world());
    // 오 분 넘게 아무도 안 건드린 상태다. 맞은 것이 자극으로 안 세어지면
    // 날아가 착지하자마자 그 자리에서 존다.
    let 때린_때 = SLEEP_AFTER_MS + 10_000;
    p.swing_knocked(때린_때, 1.0, 1_000.0);
    let 자취 = drive(&mut p, 때린_때 + 20, 때린_때 + 30_000, 20, &world());
    assert!(
        !자취.iter().any(|s| s.behavior == Behavior::Sleep),
        "맞자마자 자면 맞은 것으로 안 보인다"
    );
}

#[test]
fn 넉백은_난수를_쓰지_않는다() {
    // (1) 때린 마리 — 이웃이 있든 없든 추첨이 밀리지 않는다.
    let 혼자 = {
        let mut pets = Pets::new();
        let a = pets.add(1, 0, &world(), 300.0).unwrap();
        pets.whack(a, 1_000, &world(), 0.0, 0.0);
        drive(pets.get_mut(a).unwrap(), 1_050, 40_000, 50, &world())
    };
    let 둘이서 = {
        let (mut pets, a, _b) = 두_마리(300.0, 440.0);
        pets.whack(a, 1_000, &world(), 0.0, 0.0);
        drive(pets.get_mut(a).unwrap(), 1_050, 40_000, 50, &world())
    };
    assert_eq!(
        혼자, 둘이서,
        "이웃을 날린다고 추첨이 밀리면 골든 수열을 재기준화해야 한다"
    );

    // (2) 맞은 마리 — 판정을 거친 것과 직접 부른 것이 같아야 한다. 사거리 판정이
    //     난수를 한 번이라도 뽑으면 여기서 갈라진다.
    let 맞아서 = {
        let (mut pets, a, b) = 두_마리(300.0, 440.0);
        pets.whack(a, 1_000, &world(), 0.0, 0.0);
        drive(pets.get_mut(b).unwrap(), 1_020, 40_000, 50, &world())
    };
    let 직접 = {
        let (mut pets, _a, b) = 두_마리(300.0, 440.0);
        pets.get_mut(b)
            .unwrap()
            .swing_knocked(1_000, 1.0, world().width());
        drive(pets.get_mut(b).unwrap(), 1_020, 40_000, 50, &world())
    };
    assert_eq!(맞아서, 직접, "판정이 난수를 쓰면 맞은 마리가 갈라진다");
}
