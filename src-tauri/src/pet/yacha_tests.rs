//! 단체 야차 판의 검사. 국면 전이·다운 일정·종료 증명이 핵심이다.

use super::*;

const 세계: Bounds = Bounds {
    left: 0.0,
    right: 1_440.0,
    top: 0.0,
    floor_y: 800.0,
};

fn 자리들() -> Huddle {
    Huddle::new(세계).expect("이 세계에는 뭉칠 자리가 있다")
}

fn 판(n: usize, seed: u64) -> Yacha {
    let ids: Vec<PetId> = (1..=n as PetId).collect();
    Yacha::new(ids, 자리들(), 0, seed)
}

/// 서 있는 마리를 스탠스 순서대로 (id, 몸통 가운데 x)로 만든다.
fn 자리(board: &Yacha) -> Vec<(PetId, f64)> {
    board
        .standing()
        .iter()
        .map(|id| {
            let (x, _) = board.stance_for(*id).expect("서 있는 마리는 자리가 있다");
            (*id, x + PET_SIZE / 2.0)
        })
        .collect()
}

/// 난투를 끝까지 돌린다. 반환값은 (최후의 1인, 그때 시각).
fn 난투를_끝까지(board: &mut Yacha, n: usize) -> (PetId, u64) {
    let mut hits: std::collections::BTreeMap<PetId, u32> = std::collections::BTreeMap::new();
    let mut now = 0u64;
    while now < YACHA_MAX_MS {
        now += 50; // 20Hz 틱
        if board.round_due(now) {
            let outcome = board.plan_round(now, &자리(board));
            for (_, target) in &outcome.punches {
                *hits.entry(*target).or_insert(0) += 1;
            }
        }
        if board.down_due(now) {
            let 후보: Vec<(PetId, u32)> = board
                .standing()
                .iter()
                .map(|id| (*id, hits.get(id).copied().unwrap_or(0)))
                .collect();
            if let Some(쓰러진) = board.take_down(&후보) {
                hits.remove(&쓰러진);
            }
        }
        if board.standing().len() <= 1 {
            let champ = board.standing()[0];
            return (champ, now);
        }
    }
    panic!("{n}마리 판이 안 끝났다");
}

#[test]
fn 세계가_좁으면_판이_안_열린다() {
    let 좁다 = Bounds {
        left: 0.0,
        right: 100.0,
        top: 0.0,
        floor_y: 800.0,
    };
    assert!(Huddle::new(좁다).is_none());
    let 낮다 = Bounds {
        left: 0.0,
        right: 1_440.0,
        top: 0.0,
        floor_y: 100.0,
    };
    assert!(Huddle::new(낮다).is_none());
}

#[test]
fn 같은_시드는_같은_판을_낳는다() {
    let a = 판(5, 12_345);
    let b = 판(5, 12_345);
    assert_eq!(a.order(), b.order(), "서는 순서가 다르다");
    assert_eq!(a.down_schedule(), b.down_schedule(), "다운 일정이 다르다");
    assert_eq!(a.brawl_until_ms(), b.brawl_until_ms(), "예산이 다르다");
}

#[test]
fn 다른_시드는_다른_대진을_낳는다() {
    // 다섯 마리의 순열은 120가지다 — 시드 몇 개 중 하나라도 달라야 섞은 것이다.
    let 기준 = 판(5, 1).order();
    assert!(
        (2..40).any(|s| 판(5, s).order() != 기준),
        "시드를 바꿔도 서는 순서가 그대로다 — 안 섞고 있다"
    );
}

#[test]
fn 다운_일정이_증가하고_예산_안에_있다() {
    for n in [2usize, 3, 5, 8] {
        for seed in [1u64, 7, 99, 12_345] {
            let board = 판(n, seed);
            let 일정 = board.down_schedule();
            assert_eq!(일정.len(), n - 1, "{n}마리면 다운은 {}번이다", n - 1);
            for w in 일정.windows(2) {
                assert!(w[0] < w[1], "{n}마리 시드 {seed}: 다운 일정이 안 는다");
            }
            assert!(
                *일정.last().unwrap() < board.brawl_until_ms(),
                "{n}마리 시드 {seed}: 마지막 다운이 예산을 넘는다"
            );
        }
    }
}

#[test]
fn 다운_간격이_고르다() {
    // 앞에 몰려 쓰러지고 뒤가 비면 "하나씩 줄어드는" 리듬이 죽는다.
    // 구조적으로 참인 것을 못 박는다: 한 칸은 라운드 몇 번은 되고(눈에 보인다),
    // 평균의 두 배를 넘지 않는다(빈 구간이 안 생긴다).
    for n in [3usize, 5, 8] {
        for seed in [1u64, 4_242, 77, 90_210] {
            let board = 판(n, seed);
            let 일정 = board.down_schedule();
            let mut 간격: Vec<u64> = vec![일정[0]];
            for w in 일정.windows(2) {
                간격.push(w[1] - w[0]);
            }
            let 평균 = board.brawl_until_ms() / n as u64;
            for (k, g) in 간격.iter().enumerate() {
                assert!(
                    *g >= YACHA_ROUND_MS,
                    "{n}마리 시드 {seed}: {k}번째 다운이 앞 다운과 {g}ms밖에 안 떨어졌다"
                );
                assert!(
                    *g <= 평균 * 2,
                    "{n}마리 시드 {seed}: {k}번째 다운 전에 {g}ms(평균 {평균}ms)나 빈다"
                );
            }
        }
    }
}

#[test]
fn 판_길이가_마릿수에_비례하지_않는다() {
    let (_, 둘) = 난투를_끝까지(&mut 판(2, 777), 2);
    let (_, 여덟) = 난투를_끝까지(&mut 판(8, 777), 8);
    let (작, 큰) = if 둘 < 여덟 { (둘, 여덟) } else { (여덟, 둘) };
    assert!(
        큰 < 작 * 2,
        "두 마리 {둘}ms vs 여덟 마리 {여덟}ms — 마릿수에 비례한다"
    );
}

#[test]
fn 아무리_오래_돌아도_최후의_1인이_나온다() {
    for n in [2usize, 3, 4, 5, 6, 7, 8] {
        for seed in [1u64, 31, 500, 90_210] {
            let mut board = 판(n, seed);
            let (champ, at) = 난투를_끝까지(&mut board, n);
            assert!(board.participants().contains(&champ));
            assert!(
                at <= YACHA_BRAWL_MS.1 + 2_000,
                "{n}마리 시드 {seed}: 난투가 {at}ms나 걸렸다"
            );
        }
    }
}

#[test]
fn 가장_많이_맞은_마리가_쓰러진다() {
    let mut board = 판(4, 5);
    let 언제 = board.down_schedule()[0];
    assert!(!board.down_due(언제 - 1));
    assert!(board.down_due(언제));
    // 3번이 몰아 맞았다.
    let 후보 = vec![(1, 1u32), (2, 2), (3, 9), (4, 0)];
    assert_eq!(board.take_down(&후보), Some(3));
    assert!(!board.standing().contains(&3));
}

#[test]
fn 세_대도_안_맞았으면_다운이_미뤄진다() {
    let mut board = 판(4, 5);
    let 후보 = vec![(1, 0u32), (2, 1), (3, 2), (4, 0)];
    assert_eq!(board.take_down(&후보), None, "덜 맞았는데 쓰러졌다");
    assert_eq!(board.standing().len(), 4);
}

#[test]
fn 최다_피격이_같으면_id가_작은_쪽이_쓰러진다() {
    let mut board = 판(4, 5);
    let 후보 = vec![(1, 5u32), (2, 5), (3, 5), (4, 5)];
    assert_eq!(board.take_down(&후보), Some(1));
}

#[test]
fn 이웃은_항상_사정거리_안이다() {
    // 종료 증명 ② — 뭉쳐 있으므로 누구든 칠 수 있고, 서 있는 마리가 둘 이상인
    // 동안 피격 수가 반드시 는다.
    for n in 2..=8 {
        for seed in [1u64, 42, 7_777] {
            let board = 판(n, seed);
            let 자리: Vec<f64> = board
                .participants()
                .iter()
                .map(|id| board.stance_for(*id).unwrap().0 + PET_SIZE / 2.0)
                .collect();
            for (i, a) in 자리.iter().enumerate() {
                let 가장_가까운 = 자리
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, b)| (a - b).abs())
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    가장_가까운 <= YACHA_REACH_X,
                    "{n}마리 시드 {seed}: {i}번의 가장 가까운 이웃이 {가장_가까운}px다"
                );
            }
        }
    }
}

#[test]
fn 대형을_만들지_않는다() {
    // 사용자 지시 — 줄 세우면 안 된다. 간격이 전부 같으면 그게 대형이다.
    let board = 판(6, 31);
    let mut x: Vec<f64> = board
        .participants()
        .iter()
        .map(|id| board.stance_for(*id).unwrap().0)
        .collect();
    x.sort_by(f64::total_cmp);
    let 간격: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
    let 최대 = 간격.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let 최소 = 간격.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(최대 - 최소 > 8.0, "간격이 고르다 — 줄을 세우고 있다: {간격:?}");
}

#[test]
fn 서로_겹친다() {
    // "얽히고 섥히는 느낌" — 마리마다 실루엣이 물리는 이웃이 하나는 있어야 한다.
    for n in [2usize, 4, 6, 8] {
        for seed in [1u64, 5, 42, 90_210] {
            let board = 판(n, seed);
            let 자리: Vec<f64> = board
                .participants()
                .iter()
                .map(|id| board.stance_for(*id).unwrap().0)
                .collect();
            for (i, a) in 자리.iter().enumerate() {
                let 가장_가까운 = 자리
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, b)| (a - b).abs())
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    가장_가까운 < PET_SIZE,
                    "{n}마리 시드 {seed}: {i}번이 이웃과 {가장_가까운}px 떨어져 안 겹친다"
                );
            }
        }
    }
}

#[test]
fn 완전히_포개지지는_않는다() {
    for n in 2..=8 {
        for seed in [1u64, 42, 7_777, 90_210] {
            let board = 판(n, seed);
            let 자리: Vec<(f64, f64)> = board
                .participants()
                .iter()
                .map(|id| board.stance_for(*id).unwrap())
                .collect();
            for i in 0..자리.len() {
                for j in (i + 1)..자리.len() {
                    let d = (자리[i].0 - 자리[j].0).hypot(자리[i].1 - 자리[j].1);
                    assert!(d > 1.0, "{n}마리 시드 {seed}: {i}번과 {j}번이 같은 자리다");
                }
            }
        }
    }
}

#[test]
fn 깊게_겹친_둘은_y가_다르다() {
    // 창 z 순서를 제어할 수 없으므로 깊이는 y로만 말한다 (KTD9c).
    for seed in [1u64, 42, 7_777, 90_210, 5] {
        let board = 판(8, seed);
        let 자리: Vec<(f64, f64)> = board
            .participants()
            .iter()
            .map(|id| board.stance_for(*id).unwrap())
            .collect();
        for i in 0..자리.len() {
            for j in (i + 1)..자리.len() {
                if (자리[i].0 - 자리[j].0).abs() > PET_SIZE / 2.0 {
                    continue;
                }
                assert!(
                    (자리[i].1 - 자리[j].1).abs() >= YACHA_HUDDLE_MIN_DY,
                    "시드 {seed}: 깊게 겹친 {i}·{j}번의 y가 같다"
                );
            }
        }
    }
}

#[test]
fn 펀치는_좌표를_만들지_않는다() {
    // 판이 돌려주는 것은 id 쌍뿐이다 — 좌표가 없으니 밀어낼 방법도 없다 (R7).
    let mut board = 판(4, 3);
    let outcome = board.plan_round(YACHA_ROUND_MS, &자리(&board));
    assert!(!outcome.punches.is_empty());
    for (때린놈, 맞은놈) in &outcome.punches {
        assert_ne!(때린놈, 맞은놈, "자기를 때린다");
        assert!(board.standing().contains(맞은놈));
    }
}

#[test]
fn 라운드마다_대표_타격은_하나뿐이다() {
    let mut board = 판(8, 11);
    let mut now = 0;
    for _ in 0..10 {
        now += YACHA_ROUND_MS;
        let outcome = board.plan_round(now, &자리(&board));
        assert!(
            outcome.thud.is_some(),
            "치는 마리가 있는데 대표가 없다"
        );
        assert!(board.standing().contains(&outcome.thud.unwrap()));
    }
}

#[test]
fn 라운드마다_치는_쪽이_바뀐다() {
    // 홀짝 교대라 두 라운드를 합치면 서 있는 마리 전부가 한 번씩 친다.
    let mut board = 판(4, 8);
    let a = board.plan_round(YACHA_ROUND_MS, &자리(&board));
    let b = board.plan_round(YACHA_ROUND_MS * 2, &자리(&board));
    let mut 친놈: Vec<PetId> = a
        .punches
        .iter()
        .chain(b.punches.iter())
        .map(|(p, _)| *p)
        .collect();
    친놈.sort_unstable();
    친놈.dedup();
    assert_eq!(친놈.len(), 4, "두 라운드에 네 마리가 다 안 쳤다: {친놈:?}");
}

#[test]
fn 참여가_하나로_줄면_그대로_챔피언이다() {
    let mut board = 판(2, 2);
    board.leave(1);
    assert_eq!(board.standing(), vec![2]);
    assert_eq!(board.participants(), vec![2]);
}

#[test]
fn 판에서_빠지면_다운_일정이_짧아진다() {
    // 남은 마리가 줄면 남은 다운도 줄어야 한다 — 안 그러면 영영 안 끝난다.
    let mut board = 판(5, 6);
    assert_eq!(board.down_schedule().len(), 4);
    board.leave(1);
    board.leave(2);
    assert!(
        board.down_schedule().len() <= 2,
        "빠진 만큼 다운이 안 줄었다"
    );
}

#[test]
fn 세레모니는_정해진_순서로_간다() {
    let mut board = 판(2, 9);
    board.crown(1_000, 1);
    assert_eq!(board.phase(), RingPhase::Victory);
    assert_eq!(board.champion(), Some(1));

    let mut now = 1_000;
    let mut 본_국면 = vec![RingPhase::Victory];
    while board.phase() != RingPhase::Done {
        now += 50;
        board.step(now, 0.05);
        if *본_국면.last().unwrap() != board.phase() {
            본_국면.push(board.phase());
        }
    }
    assert_eq!(
        본_국면,
        vec![
            RingPhase::Victory,
            RingPhase::QueenIn,
            RingPhase::Belting,
            RingPhase::Ceremony,
            RingPhase::Exiting,
            RingPhase::Done,
        ]
    );
}

#[test]
fn 미녀는_오른쪽_밖에서_걸어_들어온다() {
    let mut board = 판(2, 9);
    board.crown(0, 1);
    let 처음 = board.snapshot().queen.expect("승리 국면부터 미녀가 있다");
    assert!(
        처음.x > 세계.right,
        "미녀가 화면 안에서 튀어나온다 (x={})",
        처음.x
    );
    let mut now = 0;
    while board.phase() != RingPhase::Belting && now < YACHA_MAX_MS {
        now += 50;
        board.step(now, 0.05);
    }
    let 도착 = board.snapshot().queen.expect("벨트 국면에도 미녀가 있다");
    assert!(도착.x < 세계.right, "미녀가 화면 안으로 안 들어왔다");
    assert!(
        도착.x > board.stance_for(1).expect("챔피언은 서 있다").0,
        "미녀가 챔피언을 지나쳐 갔다"
    );
}

#[test]
fn 벨트는_채운_뒤에만_챔피언에게_있다() {
    let mut board = 판(2, 9);
    board.crown(0, 1);
    assert!(!board.belt_on_champion(), "승리하자마자 벨트를 차고 있다");
    let mut now = 0;
    while board.phase() != RingPhase::Ceremony && now < YACHA_MAX_MS {
        now += 50;
        board.step(now, 0.05);
    }
    assert!(board.belt_on_champion(), "세레모니인데 벨트가 없다");
}

#[test]
fn 판은_상한을_넘겨_살지_않는다() {
    let board = 판(3, 1);
    assert!(!board.expired(YACHA_MAX_MS - 1));
    assert!(board.expired(YACHA_MAX_MS + 1));
}
