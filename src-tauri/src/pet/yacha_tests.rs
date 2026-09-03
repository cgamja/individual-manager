//! 단체 야차 판의 검사.
//!
//! **성질을 본다, 값이 아니라.** 아티팩트와 난수 구현이 다르므로 초당 펀치가
//! 정확히 같을 수는 없다 — 플랜의 "합격 기준" 절이 못 박은 성질 셋을 본다:
//! ① 박자가 없다 ② 마릿수가 늘수록 초당 펀치가 는다 ③ 난투 길이는 마릿수와 무관하다.

use super::*;

const 세계: Bounds = Bounds {
    left: 0.0,
    right: 1_440.0,
    top: 0.0,
    floor_y: 800.0,
};

fn 판(n: usize, seed: u64) -> Yacha {
    let ids: Vec<PetId> = (1..=n as PetId).collect();
    let arena = Arena::new(세계).expect("이 세계에는 판이 들어간다");
    let mut b = Yacha::new(ids, arena, 0, seed);
    b.begin_brawl(0);
    b
}

/// 난투를 `ms`만큼 돌린다. 반환값은 (모든 주먹의 시각, 마리별 이동폭).
fn 난투(board: &mut Yacha, ms: u64) -> (Vec<u64>, Vec<f64>) {
    let ids = board.participants();
    let mut 시각 = Vec::new();
    let mut 범위: BTreeMap<PetId, (f64, f64)> = ids
        .iter()
        .map(|id| {
            let (x, _) = board.xy_of(*id).unwrap();
            (*id, (x, x))
        })
        .collect();
    let mut now = 0u64;
    while now < ms {
        now += 50; // 20Hz 틱
        board.step_brawl(now);
        for p in board.punches() {
            let _ = p;
            시각.push(now);
        }
        for id in &ids {
            if let Some((x, _)) = board.xy_of(*id) {
                let e = 범위.get_mut(id).unwrap();
                e.0 = e.0.min(x);
                e.1 = e.1.max(x);
            }
        }
    }
    (시각, 범위.values().map(|(lo, hi)| hi - lo).collect())
}

#[test]
fn 세계가_좁으면_판이_안_열린다() {
    let 좁다 = Bounds {
        left: 0.0,
        right: 100.0,
        top: 0.0,
        floor_y: 800.0,
    };
    assert!(Arena::new(좁다).is_none());
    let 낮다 = Bounds {
        left: 0.0,
        right: 1_440.0,
        top: 0.0,
        floor_y: 100.0,
    };
    assert!(Arena::new(낮다).is_none());
}

#[test]
fn 같은_시드는_같은_판을_낳는다() {
    let mut a = 판(5, 12_345);
    let mut b = 판(5, 12_345);
    let (ta, _) = 난투(&mut a, 6_000);
    let (tb, _) = 난투(&mut b, 6_000);
    assert_eq!(ta, tb, "같은 시드인데 주먹 시각이 다르다");
    for id in a.participants() {
        assert_eq!(a.xy_of(id), b.xy_of(id), "{id}번의 자리가 다르다");
    }
}

#[test]
fn 다른_시드는_다른_판을_낳는다() {
    let mut a = 판(4, 1);
    let mut b = 판(4, 99);
    let (ta, _) = 난투(&mut a, 6_000);
    let (tb, _) = 난투(&mut b, 6_000);
    assert_ne!(ta, tb);
}

#[test]
fn 박자가_없다() {
    // 마리마다 제 상태 기계라 같은 시각에 몰리지 않는다. 다 같이 치면
    // "퍽퍽퍽퍽"이 아니라 메트로놈이 된다.
    let mut board = 판(6, 20_260_903);
    let (시각, _) = 난투(&mut board, 10_000);
    assert!(시각.len() >= 20, "주먹이 {}발뿐이다", 시각.len());
    let mut 셈: BTreeMap<u64, usize> = BTreeMap::new();
    for t in &시각 {
        *셈.entry(*t).or_insert(0) += 1;
    }
    let 최다 = *셈.values().max().unwrap();
    assert!(
        최다 <= 3,
        "한 틱에 {최다}발이 몰렸다 — 박자가 생겼다"
    );
}

#[test]
fn 마릿수가_늘면_초당_펀치가_는다() {
    let mut 앞 = 0.0;
    for n in [2usize, 4, 8] {
        let mut board = 판(n, 20_260_903);
        let (시각, _) = 난투(&mut board, YACHA_BRAWL_MS);
        let 초당 = 시각.len() as f64 / (YACHA_BRAWL_MS as f64 / 1_000.0);
        assert!(
            초당 > 앞,
            "{n}마리가 {초당:.1}발/초 — 앞({앞:.1})보다 안 늘었다"
        );
        앞 = 초당;
    }
}

#[test]
fn 헛스윙이_거의_없다() {
    // v5는 사정거리 밖에서도 스윙을 골라 4마리 14초에 29발뿐이었다.
    // **닿을 때만 친다**로 바꾼 것이 이 판의 밀도다.
    let mut board = 판(4, 20_260_903);
    let (시각, _) = 난투(&mut board, YACHA_BRAWL_MS);
    let 초당 = 시각.len() as f64 / (YACHA_BRAWL_MS as f64 / 1_000.0);
    assert!(초당 >= 2.0, "4마리에 {초당:.1}발/초 — 너무 성기다");
}

#[test]
fn 사방으로_다닌다() {
    // 제자리에서 치고받으면 "움직임이 거의 없다"가 된다. v7에서 판을 넓히고
    // 상대 재선택을 늘려 이동폭을 키웠다 (*"좀 더 넓게 움직이고 싸우게 해줘"*).
    // **일찍 쓰러진 놈은 거기서 멈추므로 최소가 아니라 최대를 본다.**
    for (n, 문턱) in [(4usize, 110.0f64), (8, 180.0)] {
        let mut board = 판(n, 20_260_903);
        let (_, 이동폭) = 난투(&mut board, YACHA_BRAWL_MS);
        let 최대 = 이동폭.iter().cloned().fold(0.0f64, f64::max);
        assert!(
            최대 > 문턱,
            "{n}마리: 가장 많이 움직인 마리가 {최대:.0}px뿐이다 (문턱 {문턱:.0})"
        );
    }
}

#[test]
fn 판_밖으로_안_나간다() {
    let mut board = 판(8, 7);
    난투(&mut board, YACHA_BRAWL_MS);
    for id in board.participants() {
        let (x, y) = board.xy_of(id).unwrap();
        assert!(
            (세계.left..=세계.right).contains(&x),
            "{id}번이 가로로 나갔다: {x}"
        );
        assert!(
            y >= YACHA_ARENA_Y.0 - 1.0 && y <= YACHA_ARENA_Y.1 + 1.0,
            "{id}번이 세로로 나갔다: {y}"
        );
    }
}

#[test]
fn 완전히_포개지지는_않는다() {
    // 비켜서기(`separate`)가 도는지 본다.
    let mut board = 판(8, 5);
    난투(&mut board, 8_000);
    let ids = board.standing();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let (ax, ay) = board.xy_of(ids[i]).unwrap();
            let (bx, by) = board.xy_of(ids[j]).unwrap();
            let d = (bx - ax).hypot((by - ay) * YACHA_YW);
            assert!(d > 1.0, "{}번과 {}번이 같은 자리다", ids[i], ids[j]);
        }
    }
}

#[test]
fn 가드는_실제로_막는다() {
    let mut board = 판(2, 3);
    let ids = board.participants();
    let (a, b) = (ids[0], ids[1]);
    board.set_act(b, Act::Guard, 100_000);
    let 맞기_전 = board.hits_of(b);
    // a를 b 옆에 붙여 두고 몇 초 돌린다.
    let mut now = 0;
    while now < 6_000 {
        now += 50;
        board.step_brawl(now);
        board.set_act(b, Act::Guard, 100_000); // 계속 가드로 붙들어 둔다
    }
    assert_eq!(
        board.hits_of(b),
        맞기_전,
        "가드 중인데 맞았다"
    );
    let _ = a;
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
                *일정.last().unwrap() < YACHA_BRAWL_MS,
                "{n}마리 시드 {seed}: 마지막 다운이 예산을 넘는다"
            );
        }
    }
}

#[test]
fn 아무리_오래_돌아도_최후의_1인이_나온다() {
    for n in [2usize, 3, 4, 5, 6, 7, 8] {
        for seed in [1u64, 31, 500, 90_210] {
            let mut board = 판(n, seed);
            난투(&mut board, YACHA_BRAWL_MS + 2_000);
            assert_eq!(
                board.standing().len(),
                1,
                "{n}마리 시드 {seed}: {}마리가 남았다",
                board.standing().len()
            );
        }
    }
}

#[test]
fn 난투_길이는_마릿수와_무관하다() {
    // **예산이 판을 끝낸다.** 마지막 다운이 일찍 나도 난투는 예산을 채우고,
    // 그 사이 챔피언은 혼자 `Idle`이다 — 그래야 한 판 길이가 늘 같다.
    for n in [2usize, 4, 8] {
        let board = 판(n, 20_260_903);
        assert!(!board.phase_over(YACHA_BRAWL_MS - 100), "{n}마리가 일찍 끝난다");
        assert!(board.phase_over(YACHA_BRAWL_MS + 100), "{n}마리가 안 끝난다");
    }
}

#[test]
fn 혼자_남으면_아무도_안_친다() {
    // 허공에 주먹을 내지르면 이긴 게 아니라 이상해 보인다.
    let mut board = 판(2, 11);
    난투(&mut board, YACHA_BRAWL_MS + 2_000);
    let 챔프 = board.standing()[0];
    let mut now = YACHA_BRAWL_MS + 2_000;
    let mut 뒤 = 0;
    while 뒤 < 2_000 {
        now += 50;
        뒤 += 50;
        board.step_brawl(now);
        assert!(board.punches().is_empty(), "혼자인데 주먹이 났다");
    }
    assert_eq!(board.act_of(챔프), Some(Act::Idle));
}

#[test]
fn 쓰러진_놈은_안_맞는다() {
    let mut board = 판(4, 13);
    난투(&mut board, YACHA_BRAWL_MS + 2_000);
    let 서있는 = board.standing();
    let mut now = YACHA_BRAWL_MS + 2_000;
    for _ in 0..40 {
        now += 50;
        board.step_brawl(now);
        for p in board.punches() {
            assert!(서있는.contains(&p.to), "쓰러진 놈을 쳤다");
        }
    }
}

#[test]
fn 참여가_하나로_줄면_그대로_챔피언이다() {
    let mut board = 판(2, 2);
    board.leave(board.participants()[0]);
    assert_eq!(board.standing().len(), 1);
}

#[test]
fn 판에서_빠지면_남은_다운도_준다() {
    let mut board = 판(5, 6);
    assert_eq!(board.down_schedule().len(), 4);
    let ids = board.participants();
    board.leave(ids[0]);
    board.leave(ids[1]);
    assert!(board.down_schedule().len() <= 2, "빠진 만큼 다운이 안 줄었다");
}

#[test]
fn 세레모니는_정해진_순서로_간다() {
    let mut board = 판(2, 9);
    let champ = board.participants()[0];
    board.crown(1_000, champ);
    assert_eq!(board.phase(), RingPhase::Victory);

    let mut now = 1_000;
    let mut 본_국면 = vec![RingPhase::Victory];
    while board.phase() != RingPhase::Done && now < 60_000 {
        now += 50;
        board.step_ceremony(now, 0.05);
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
    board.crown(0, board.participants()[0]);
    let 처음 = board.snapshot().queen.expect("승리 국면부터 미녀가 있다");
    assert!(처음.x > 세계.right, "미녀가 화면 안에서 튀어나온다");
    let mut now = 0;
    while board.phase() != RingPhase::Belting && now < 60_000 {
        now += 50;
        board.step_ceremony(now, 0.05);
    }
    let 도착 = board.snapshot().queen.expect("벨트 국면에도 미녀가 있다");
    assert!(도착.x < 세계.right, "미녀가 화면 안으로 안 들어왔다");
}

#[test]
fn 미녀는_챔피언과_같은_높이에_선다() {
    // 판 바닥에 못 박으면 판이 위쪽에서 끝났을 때 미녀가 챔피언보다 몸 두 개
    // 아래에 서서 **허공에 벨트를 채운다.** x와 같은 원천에서 y도 가져와야 한다.
    for seed in [3u64, 11, 20_260_903] {
        let mut board = 판(4, seed);
        난투(&mut board, YACHA_BRAWL_MS);
        let champ = board.standing()[0];
        let (_, champ_y) = board.xy_of(champ).unwrap();
        board.crown(YACHA_BRAWL_MS, champ);
        let queen = board.snapshot().queen.expect("승리 국면부터 미녀가 있다");
        let (champ_at, _, _) = board.pose_of(champ).unwrap();
        assert!(
            (queen.y - champ_at.1).abs() < 1.0,
            "시드 {seed}: 미녀 y {} vs 챔피언 y {} (오프셋 {champ_y})",
            queen.y,
            champ_at.1
        );
    }
}

#[test]
fn 벨트는_채운_뒤에만_챔피언에게_있다() {
    let mut board = 판(2, 9);
    board.crown(0, board.participants()[0]);
    assert!(!board.belt_on_champion(), "승리하자마자 벨트를 차고 있다");
    let mut now = 0;
    while board.phase() != RingPhase::Ceremony && now < 60_000 {
        now += 50;
        board.step_ceremony(now, 0.05);
    }
    assert!(board.belt_on_champion(), "세레모니인데 벨트가 없다");
}

#[test]
fn 깊이는_위에_있는_놈이_뒤다() {
    let mut board = 판(6, 4);
    난투(&mut board, 5_000);
    let 순서 = board.snapshot().depth;
    let mut 앞 = f64::NEG_INFINITY;
    for id in 순서 {
        let (_, y) = board.xy_of(id).unwrap();
        assert!(y >= 앞 - 1e-9, "깊이 순서가 y와 어긋난다");
        앞 = y;
    }
}

#[test]
fn 판은_상한을_넘겨_살지_않는다() {
    let board = 판(3, 1);
    assert!(!board.expired(YACHA_MAX_MS - 1));
    assert!(board.expired(YACHA_MAX_MS + 1));
}
