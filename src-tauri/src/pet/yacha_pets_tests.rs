//! `Pets`에 붙은 야차의 검사 — 상호 배제, 핀볼·스윙 제외, 한 판의 길이.

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

/// 한 판을 끝까지 돌린다. 반환값은 (걸린 시간 ms, 지나온 국면들).
fn 한_판(pets: &mut Pets, world: &World) -> (u64, Vec<RingPhase>) {
    let mut now = 0u64;
    let mut 본_국면 = Vec::new();
    while now < YACHA_MAX_MS {
        now += 50;
        pets.step_all(now, |_| Some(world));
        match pets.yacha().map(Yacha::phase) {
            Some(p) => {
                if 본_국면.last() != Some(&p) {
                    본_국면.push(p);
                }
            }
            None => return (now, 본_국면),
        }
    }
    panic!("판이 안 끝났다: {본_국면:?}");
}

#[test]
fn 한_마리면_판이_안_열린다() {
    let (mut pets, _) = 무리(1);
    assert_eq!(pets.start_yacha(0, 세계, 1), Err(YachaRefusal::TooFew));
    assert!(pets.yacha().is_none());
}

#[test]
fn 홀수여도_판이_열린다() {
    // 비치발리볼과 갈리는 지점 — 팀이 없는 난투라 홀수도 정상이다.
    let (mut pets, _) = 무리(3);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    assert_eq!(pets.yacha().unwrap().participants().len(), 3);
}

#[test]
fn 화면이_좁으면_모자란_것보다_먼저_거절한다() {
    let 좁다 = Bounds {
        left: 0.0,
        right: 100.0,
        top: 0.0,
        floor_y: 800.0,
    };
    let (mut pets, _) = 무리(1);
    assert_eq!(pets.start_yacha(0, 좁다, 1), Err(YachaRefusal::NoRoom));
}

#[test]
fn 세_판은_서로를_배제한다() {
    let (mut pets, _) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    assert!(!pets.start_bowling(0, 세계), "야차 중에 볼링이 열렸다");
    assert_eq!(
        pets.start_volleyball(0, 세계, 1),
        Err(VolleyRefusal::BoardBusy),
        "야차 중에 발리볼이 열렸다"
    );

    let (mut pets, _) = 무리(4);
    assert!(pets.start_bowling(0, 세계));
    assert_eq!(
        pets.start_yacha(0, 세계, 1),
        Err(YachaRefusal::BoardBusy),
        "볼링 중에 야차가 열렸다"
    );

    let (mut pets, _) = 무리(4);
    assert_eq!(pets.start_volleyball(0, 세계, 1), Ok(()));
    assert_eq!(
        pets.start_yacha(0, 세계, 1),
        Err(YachaRefusal::BoardBusy),
        "발리볼 중에 야차가 열렸다"
    );
}

#[test]
fn 판이_도는_동안_핀볼_충돌이_쉰다() {
    // 이웃 간격(96px)이 충돌 반경보다 좁아서, 안 쉬면 링에 서기만 해도 서로를
    // 튕긴다. "서로 튕겨나가지 않는다"(R7)의 마지막 문이다.
    let (mut pets, world) = 무리(4);
    for id in pets.ids() {
        pets.get_mut(id).unwrap().set_pinball(true);
    }
    assert_eq!(pets.start_yacha(0, 세계, 3), Ok(()));
    let mut now = 0;
    while now < 12_000 {
        now += 50;
        pets.step_all(now, |_| Some(&world));
        let Some(board) = pets.yacha() else { break };
        if board.phase() != RingPhase::Brawl {
            continue;
        }
        let 서있는 = board.standing();
        for id in 서있는 {
            let b = pets.get(id).unwrap().behavior();
            assert!(
                matches!(b, Behavior::Yacha { .. }),
                "핀볼 충돌로 링에서 튕겨 나갔다: {b:?}"
            );
        }
    }
}

#[test]
fn 방망이_스윙이_링의_이웃을_안_날린다() {
    // 스윙 사거리(200px)가 이웃 간격(96px)의 두 배라, 이 가드가 빠지면 한 번
    // 휘두를 때 링의 절반이 날아간다.
    let (mut pets, world) = 무리(5);
    assert_eq!(pets.start_yacha(0, 세계, 4), Ok(()));
    let mut now = 0;
    while pets.yacha().map(Yacha::phase) == Some(RingPhase::Gathering) && now < 10_000 {
        now += 50;
        pets.step_all(now, |_| Some(&world));
    }
    let 링위 = pets.yacha().unwrap().standing();
    let 맞을놈 = 링위[2];
    pets.whack(맞을놈, now, &world, 0.5, 0.5);
    let 남은 = 링위
        .iter()
        .filter(|id| **id != 맞을놈)
        .filter(|id| matches!(pets.get(**id).unwrap().behavior(), Behavior::Yacha { .. }))
        .count();
    assert_eq!(남은, 링위.len() - 1, "이웃이 스윙에 날아갔다");
}

#[test]
fn 판_도중_마리를_지워도_안_깨진다() {
    let (mut pets, world) = 무리(5);
    assert_eq!(pets.start_yacha(0, 세계, 5), Ok(()));
    let mut now = 0;
    let mut 지울_것 = pets.ids();
    while now < YACHA_MAX_MS {
        now += 50;
        pets.step_all(now, |_| Some(&world));
        if now % 3_000 == 0 && 지울_것.len() > 1 {
            let id = 지울_것.remove(0);
            pets.remove(id);
        }
        if pets.yacha().is_none() {
            return;
        }
    }
    panic!("판이 안 끝났다");
}

#[test]
fn 한_판이_스물다섯에서_서른초쯤_걸린다() {
    for n in [2usize, 3, 5, 8] {
        for seed in [1u64, 12_345] {
            let (mut pets, world) = 무리(n);
            assert_eq!(pets.start_yacha(0, 세계, seed), Ok(()));
            let (걸린, 본_국면) = 한_판(&mut pets, &world);
            assert!(
                (15_000..=40_000).contains(&걸린),
                "{n}마리 시드 {seed}: 한 판이 {걸린}ms 걸렸다"
            );
            assert!(
                본_국면.contains(&RingPhase::Ceremony),
                "{n}마리 시드 {seed}: 세레모니가 안 나왔다 — {본_국면:?}"
            );
        }
    }
}

#[test]
fn 판이_끝나면_전부_평소로_돌아간다() {
    let (mut pets, world) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 2), Ok(()));
    한_판(&mut pets, &world);
    for id in pets.ids() {
        let b = pets.get(id).unwrap().behavior();
        assert!(
            !matches!(b, Behavior::Yacha { .. }),
            "{id}번이 국면에 갇혔다: {b:?}"
        );
    }
}

#[test]
fn 쓰러진_마리가_링에_그대로_눕는다() {
    let (mut pets, world) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 8), Ok(()));
    let mut now = 0;
    let mut 봤다 = false;
    while now < YACHA_MAX_MS {
        now += 50;
        pets.step_all(now, |_| Some(&world));
        let Some(board) = pets.yacha() else { break };
        let 서있는 = board.standing();
        let 누운놈: Vec<PetId> = board
            .participants()
            .into_iter()
            .filter(|id| !서있는.contains(id))
            .collect();
        for id in 누운놈 {
            봤다 = true;
            let b = pets.get(id).unwrap().behavior();
            assert!(
                matches!(
                    b,
                    Behavior::Yacha {
                        yacha: YachaPhase::Down
                    }
                ),
                "쓰러진 {id}번이 {b:?}다"
            );
        }
    }
    assert!(봤다, "아무도 안 쓰러졌다");
}

#[test]
fn 쓰러뜨린_한_방은_코어가_켠다() {
    // **프론트가 아니라 코어가 켜는지**를 본다. 프론트 검사는 `punch_down: true`인
    // 스냅샷을 손으로 만들어 통과하므로, 코어가 그걸 한 번도 안 켜도 초록이었다.
    let (mut pets, world) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 8), Ok(()));
    let mut now = 0;
    let mut 낮은_한_방 = false;
    while now < YACHA_MAX_MS {
        now += 50;
        for (_, s) in pets.step_all(now, |_| Some(&world)) {
            if s.punch_down {
                낮은_한_방 = true;
            }
        }
        if pets.yacha().is_none() {
            break;
        }
    }
    assert!(
        낮은_한_방,
        "판이 다 도는 동안 쓰러뜨린 한 방이 한 번도 안 났다"
    );
}

#[test]
fn 막힌_주먹이_신호로_나간다() {
    // 막히면 맞은 쪽이 `Guard` 그대로라 **국면으로는 알 수 없다.**
    let (mut pets, world) = 무리(6);
    assert_eq!(pets.start_yacha(0, 세계, 3), Ok(()));
    let mut now = 0;
    let mut 막힘 = false;
    while now < YACHA_MAX_MS {
        now += 50;
        for (_, s) in pets.step_all(now, |_| Some(&world)) {
            if s.punch_blocked {
                막힘 = true;
            }
        }
        if pets.yacha().is_none() {
            break;
        }
    }
    assert!(막힘, "가드가 한 번도 안 막혔다 — 신호가 안 나간다");
}

#[test]
fn 판이_도는_중에_펭귄을_꺼도_판이_남지_않는다() {
    // `clear()`가 판을 안 지우면 `pet_summary().yacha`가 영원히 참으로 남아
    // 판 버튼 셋이 계속 잠긴다.
    let (mut pets, _) = 무리(4);
    assert_eq!(pets.start_yacha(0, 세계, 1), Ok(()));
    assert!(pets.yacha().is_some());
    pets.clear();
    assert!(pets.yacha().is_none(), "펭귄을 껐는데 판이 남아 있다");
}
