use super::*;

/// 1440×900쯤의 화면을 흉내 낸 경계.
fn 넓은_코트() -> Bounds {
    Bounds {
        left: 52.0,
        right: 1_248.0,
        top: 80.0,
        floor_y: 700.0,
    }
}

fn 코트() -> Court {
    Court::new(넓은_코트()).expect("넓은 화면에서는 코트가 선다")
}

// ── 코트 기하 ──────────────────────────────────────────────────

#[test]
fn 좁은_화면에서는_판을_열_수_없다() {
    let 좁은 = Bounds {
        left: 0.0,
        right: 200.0,
        top: 0.0,
        floor_y: 400.0,
    };
    assert!(Court::new(좁은).is_none());
}

#[test]
fn 납작한_경계에서_패닉하지_않는다() {
    let 납작 = Bounds {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        floor_y: 0.0,
    };
    assert!(Court::new(납작).is_none());
}

#[test]
fn 타점은_네트_꼭대기보다_높다() {
    // KTD6 — 이게 성립하는 동안 공은 네트에 걸릴 수가 없다. `tuning.rs`의
    // `const assert!`와 같은 주장을 실제 기하로 한 번 더 확인한다.
    let c = 코트();
    assert!(
        c.contact_y() < c.net_top_y(),
        "타점 {} 이 네트 꼭대기 {} 보다 아래다",
        c.contact_y(),
        c.net_top_y()
    );
}

#[test]
fn 모래는_발밑에_있다() {
    let b = 넓은_코트();
    let c = Court::new(b).unwrap();
    assert_eq!(c.sand_y(), b.floor_y + PET_SIZE);
}

#[test]
fn 양_팀은_네트를_사이에_두고_선다() {
    let c = 코트();
    let (l0, l1) = c.span_of(Side::Left);
    let (r0, r1) = c.span_of(Side::Right);
    assert!(l0 < l1, "왼쪽 폭이 뒤집혔다");
    assert!(r0 < r1, "오른쪽 폭이 뒤집혔다");
    assert!(l1 < c.net_cx(), "왼쪽 코트가 네트를 넘었다");
    assert!(r0 > c.net_cx(), "오른쪽 코트가 네트를 넘었다");
    // 네트에 딱 붙어 서지 않는다.
    assert!(c.net_cx() - l1 >= VOLLEY_NET_GAP * 0.5);
    assert!(r0 - c.net_cx() >= VOLLEY_NET_GAP * 0.5);
}

#[test]
fn 코트는_네트를_사이에_두고_대칭이다() {
    let c = 코트();
    let (l0, l1) = c.span_of(Side::Left);
    let (r0, r1) = c.span_of(Side::Right);
    assert!((c.net_cx() - l1 - (r0 - c.net_cx())).abs() < 1e-9);
    assert!((c.net_cx() - l0 - (r1 - c.net_cx())).abs() < 1e-9);
}

#[test]
fn 자리는_세계_경계_안에_있다() {
    let b = 넓은_코트();
    let c = Court::new(b).unwrap();
    for n in 1..=4 {
        for k in 0..n {
            for side in [Side::Left, Side::Right] {
                let (x, y) = c.spot_of(side, k, n);
                assert!(
                    x >= b.left && x <= b.right,
                    "{side:?} {k}/{n} 의 x={x} 가 경계 밖이다"
                );
                assert_eq!(y, b.floor_y, "펭귄은 모래 위에 선다");
            }
        }
    }
}

#[test]
fn 한_마리면_자기_코트_가운데에_선다() {
    let c = 코트();
    let (x, _) = c.spot_of(Side::Left, 0, 1);
    let (lo, hi) = c.span_of(Side::Left);
    assert!(((x + PET_SIZE / 2.0) - (lo + hi) / 2.0).abs() < 1e-9);
}

#[test]
fn 여럿이면_코트_폭에_고르게_퍼진다() {
    let c = 코트();
    let (lo, hi) = c.span_of(Side::Left);
    let 자리: Vec<f64> = (0..3)
        .map(|k| c.spot_of(Side::Left, k, 3).0 + PET_SIZE / 2.0)
        .collect();
    assert!((자리[0] - lo).abs() < 1e-9);
    assert!((자리[2] - hi).abs() < 1e-9);
    // 간격이 같다.
    assert!(((자리[1] - 자리[0]) - (자리[2] - 자리[1])).abs() < 1e-9);
}

#[test]
fn 네트를_기준으로_어느_쪽인지_안다() {
    let c = 코트();
    assert_eq!(c.side_of_cx(c.net_cx() - 10.0), Side::Left);
    assert_eq!(c.side_of_cx(c.net_cx() + 10.0), Side::Right);
}

#[test]
fn 코트_사각형은_네트_꼭대기부터_모래_아래까지다() {
    let c = 코트();
    let (_, y, _, h) = c.rect();
    assert_eq!(y, c.net_top_y());
    assert_eq!(h, VOLLEY_NET_HEIGHT + VOLLEY_SAND_DEPTH);
}

// ── 팀 배정 ────────────────────────────────────────────────────

#[test]
fn 홀수면_왼쪽_팀이_한_마리_많다() {
    let sides = assign_sides(3);
    assert_eq!(sides.iter().filter(|s| **s == Side::Left).count(), 2);
    assert_eq!(sides.iter().filter(|s| **s == Side::Right).count(), 1);
}

#[test]
fn 팀은_id_오름차순으로_번갈아_배정된다() {
    assert_eq!(
        assign_sides(4),
        vec![Side::Left, Side::Right, Side::Left, Side::Right]
    );
    // 같은 마릿수는 항상 같은 배치를 낳는다 (난수를 안 쓴다).
    assert_eq!(assign_sides(8), assign_sides(8));
}

#[test]
fn 두_마리면_한_명씩_나뉜다() {
    assert_eq!(assign_sides(2), vec![Side::Left, Side::Right]);
}

#[test]
fn 반대편은_서로다() {
    assert_eq!(Side::Left.other(), Side::Right);
    assert_eq!(Side::Right.other(), Side::Left);
}

// ── 공 물리와 랠리 계획 ────────────────────────────────────────

/// 코트에 마리 `n`을 세우고 판을 연다. 반환은 (판, (id, 팀, 몸통 가운데 x) 목록).
fn 판(n: usize, seed: u64) -> (Volleyball, Vec<(PetId, Side, f64)>) {
    let court = 코트();
    let sides = assign_sides(n);
    let 좌수 = sides.iter().filter(|s| **s == Side::Left).count();
    let 우수 = n - 좌수;
    let (mut 좌, mut 우) = (0usize, 0usize);
    let mut players = BTreeMap::new();
    let mut 위치 = Vec::new();
    for (i, side) in sides.iter().enumerate() {
        let id = i as PetId + 1;
        let (k, total) = if *side == Side::Left {
            좌 += 1;
            (좌 - 1, 좌수)
        } else {
            우 += 1;
            (우 - 1, 우수)
        };
        let spot = court.spot_of(*side, k, total);
        players.insert(id, (*side, spot));
        위치.push((id, *side, spot.0 + PET_SIZE / 2.0));
    }
    (Volleyball::new(players, court, 0, seed), 위치)
}

struct 랠리결과 {
    왕복: usize,
    끝난_ms: u64,
    체공들: Vec<u64>,
    목적지들: Vec<f64>,
}

/// 판을 열고 서브까지 넣은 뒤 끝날 때까지 굴린다. 받을 마리는 목적지로 곧장
/// 이동시킨다 — 코어의 랠리 계획만 떼어 보기 위한 이상적 모형이고, 실제 이동은
/// `motion/volleyball.rs`가 한다.
fn 랠리를_굴린다(n: usize, seed: u64) -> 랠리결과 {
    const 틱: u64 = 50;
    let dt = 틱 as f64 / 1_000.0;
    let (mut board, mut 위치) = 판(n, seed);
    let mut now = 0u64;
    board.serve(now, &위치);
    let mut 왕복 = 0;
    let mut 체공들 = Vec::new();
    let mut 목적지들 = Vec::new();

    while now < 200_000 {
        now += 틱;
        board.step_ball(dt);

        if let Some((rid, tcx)) = board.receiver() {
            if let Some(p) = 위치.iter_mut().find(|(id, _, _)| *id == rid) {
                let 남은 = tcx - p.2;
                let 한걸음 = VOLLEY_CHASE_SPEED * dt;
                p.2 += 남은.clamp(-한걸음, 한걸음);
            }
            let cx = 위치.iter().find(|(id, _, _)| *id == rid).map(|p| p.2);
            if let Some(cx) = cx {
                if board.contact_at(cx) {
                    let to = board.next_side();
                    let 상대: Vec<(PetId, f64)> = 위치
                        .iter()
                        .filter(|(_, s, _)| *s == to)
                        .map(|(id, _, cx)| (*id, *cx))
                        .collect();
                    board.hit(now, &상대);
                    왕복 += 1;
                    체공들.push(board.last_flight_ms());
                    if let Some((_, tcx)) = board.receiver() {
                        목적지들.push(tcx);
                    }
                }
            }
        }

        if board.landed() {
            return 랠리결과 {
                왕복,
                끝난_ms: now,
                체공들,
                목적지들,
            };
        }
    }
    panic!("판이 200초 안에 안 끝났다");
}

#[test]
fn 같은_시드는_같은_랠리를_낳는다() {
    // **매 틱 전체를 대조한다** — 끝에서 한 번만 보면 중간에 갈렸다가 우연히
    // 붙은 경우를 놓친다.
    let (mut a, 위치a) = 판(4, 0xBEEF);
    let (mut b, 위치b) = 판(4, 0xBEEF);
    a.serve(0, &위치a);
    b.serve(0, &위치b);
    for t in 1..=400u64 {
        a.step_ball(0.05);
        b.step_ball(0.05);
        let now = t * 50;
        assert_eq!(a.ball(), b.ball(), "{now}ms 에서 공이 갈렸다");
        assert_eq!(a.receiver(), b.receiver(), "{now}ms 에서 받을 마리가 갈렸다");
        assert_eq!(a.phase(), b.phase(), "{now}ms 에서 국면이 갈렸다");
    }
}

#[test]
fn 다른_시드는_다른_랠리를_낳는다() {
    let 결과들: Vec<Vec<f64>> = (1u64..=5).map(|s| 랠리를_굴린다(4, s).목적지들).collect();
    assert!(
        결과들.iter().any(|r| *r != 결과들[0]),
        "시드 다섯 개가 전부 같은 목적지 열을 냈다 — 난수가 안 걸렸다"
    );
}

#[test]
fn 공은_반드시_네트를_넘는다() {
    // KTD6 — 네트 근처를 지날 때 공은 언제나 네트 꼭대기보다 위다.
    let court = 코트();
    for seed in 1u64..=8 {
        let (mut board, mut 위치) = 판(4, seed);
        board.serve(0, &위치);
        let mut now = 0u64;
        while now < 40_000 && !board.landed() {
            now += 50;
            board.step_ball(0.05);
            if let Some(ball) = board.ball() {
                if (ball.x - court.net_cx()).abs() < 40.0 {
                    assert!(
                        ball.y <= court.net_top_y(),
                        "시드 {seed}: 공이 네트에 걸렸다 (y={}, 네트 꼭대기={})",
                        ball.y,
                        court.net_top_y()
                    );
                }
            }
            if let Some((rid, tcx)) = board.receiver() {
                if let Some(p) = 위치.iter_mut().find(|(id, _, _)| *id == rid) {
                    let 남은 = tcx - p.2;
                    p.2 += 남은.clamp(-VOLLEY_CHASE_SPEED * 0.05, VOLLEY_CHASE_SPEED * 0.05);
                }
                let cx = 위치.iter().find(|(id, _, _)| *id == rid).unwrap().2;
                if board.contact_at(cx) {
                    let to = board.next_side();
                    let 상대: Vec<(PetId, f64)> = 위치
                        .iter()
                        .filter(|(_, s, _)| *s == to)
                        .map(|(id, _, cx)| (*id, *cx))
                        .collect();
                    board.hit(now, &상대);
                }
            }
        }
    }
}

#[test]
fn 공이_모래에_닿으면_판이_끝난다() {
    let (mut board, 위치) = 판(2, 7);
    board.serve(0, &위치);
    let mut now = 0;
    while now < 10_000 && !board.landed() {
        now += 50;
        board.step_ball(0.05);
    }
    assert!(board.landed(), "받는 사람이 없어도 공은 반드시 떨어진다");
    board.settle(now);
    assert_eq!(board.phase(), CourtPhase::Point);
}

#[test]
fn 예산이_다_되면_반드시_끝난다() {
    // 여덟 마리가 코트를 빽빽하게 덮어도 끝난다 — KTD7의 종료 증명.
    for seed in 1u64..=10 {
        let r = 랠리를_굴린다(8, seed);
        assert!(
            r.끝난_ms <= VOLLEY_SESSION_MS.1 + 4_000,
            "시드 {seed}: {}ms 나 걸렸다",
            r.끝난_ms
        );
    }
}

#[test]
fn 한_판은_스무_초쯤_걸린다() {
    for seed in 1u64..=20 {
        let r = 랠리를_굴린다(4, seed);
        assert!(
            (VOLLEY_SESSION_MS.0..=VOLLEY_SESSION_MS.1 + 3_000).contains(&r.끝난_ms),
            "시드 {seed}: {}ms — 18~25초 밖이다",
            r.끝난_ms
        );
    }
}

#[test]
fn 랠리는_열_번_넘게_오간다() {
    // 밀도가 곧 이 동작의 재미다. 왕복이 몇 번 안 되면 20초가 통째로 빈다.
    let mut 왕복들: Vec<usize> = (1u64..=20).map(|s| 랠리를_굴린다(4, s).왕복).collect();
    왕복들.sort_unstable();
    let 중앙값 = 왕복들[왕복들.len() / 2];
    assert!(중앙값 >= 12, "왕복 중앙값이 {중앙값}회뿐이다 — 랠리로 안 보인다");
}

#[test]
fn 체공_등급이_세_가지_다_나온다() {
    let mut 본_것 = std::collections::BTreeSet::new();
    for seed in 1u64..=20 {
        for ms in 랠리를_굴린다(4, seed).체공들 {
            if VOLLEY_FLIGHT_MS.contains(&ms) {
                본_것.insert(ms);
            }
        }
    }
    assert_eq!(
        본_것.len(),
        3,
        "체공 등급 셋 중 {}가지만 나왔다 — 리듬이 안 갈린다",
        본_것.len()
    );
}

#[test]
fn 멀리_보낸_공은_체공이_길다() {
    // 체공은 "받을 마리가 도착할 수 있는 최소값"으로 아래를 눌러 잡는다. 그래서
    // 먼 곳을 노린 공은 저절로 토스가 된다 — 주사위 하나가 변화 둘을 만든다.
    let court = 코트();
    let 가까이 = flight_ms_for(&court, 0.0, VOLLEY_FLIGHT_MS[0]);
    let 멀리 = flight_ms_for(&court, 340.0, VOLLEY_FLIGHT_MS[0]);
    assert!(멀리 > 가까이, "먼 곳({멀리}ms)이 가까운 곳({가까이}ms)보다 안 길다");
}

#[test]
fn 목적지에서_가장_가까운_마리가_받는다() {
    // 난수가 아니라 거리로 정한다 — 그래야 "저쪽으로 갔으니 쟤가 뛰겠구나"가 읽힌다.
    let 상대 = [(10u32, 0.0), (11, 100.0), (12, 300.0)];
    assert_eq!(nearest_to(&상대, 90.0), Some(11));
    assert_eq!(nearest_to(&상대, 5.0), Some(10));
    assert_eq!(nearest_to(&상대, 999.0), Some(12));
    assert_eq!(nearest_to(&[], 0.0), None);
}

#[test]
fn 킬샷은_아무도_못_받는다() {
    // 예산이 지난 뒤에는 접촉 판정을 통째로 건너뛴다 — 이게 종료의 증명이다.
    let (mut board, 위치) = 판(4, 3);
    board.serve(0, &위치);
    board.force_rally_over();
    board.step_ball(0.05);
    for (_, _, cx) in &위치 {
        assert!(!board.contact_at(*cx), "예산이 지났는데 접촉이 잡혔다");
    }
}

#[test]
fn 틱이_밀려도_타점을_건너뛰지_않는다() {
    // 한 틱이 250ms를 정산하면 공이 한 번에 300px 넘게 내려온다 — 지금 위치만
    // 보면 타점을 통째로 뛰어넘는다. 구간으로 봐야 잡힌다.
    let (mut board, 위치) = 판(2, 11);
    board.serve(0, &위치);
    let (_, cx) = board.receiver().unwrap();
    let _ = 위치;
    let mut 잡혔나 = false;
    for _ in 0..8 {
        board.step_ball(0.25);
        if board.contact_at(cx) {
            잡혔나 = true;
            break;
        }
    }
    assert!(잡혔나, "250ms 틱에서 타점을 놓쳤다");
}

#[test]
fn 서브는_자기_쪽으로_띄웠다_때린다() {
    // 서브는 국면이 아니라 "자기에게 보내는 왕복 0번"이다 (KTD4).
    let (mut board, 위치) = 판(4, 5);
    board.serve(0, &위치);
    let (rid, tcx) = board.receiver().expect("서브하면 받을 마리가 정해진다");
    let 서버_cx = 위치.iter().find(|(id, _, _)| *id == rid).unwrap().2;
    assert!((tcx - 서버_cx).abs() < 1e-9, "서브의 목적지가 자기 자리가 아니다");
    let ball = board.ball().unwrap();
    assert!((ball.x - 서버_cx).abs() < 1e-9, "공이 서버 위에서 안 뜬다");
    assert_eq!(board.phase(), CourtPhase::Rally);
}

#[test]
fn 공은_올라가는_동안_안_맞는다() {
    let (mut board, 위치) = 판(2, 13);
    board.serve(0, &위치);
    let (_, cx) = board.receiver().unwrap();
    let _ = 위치;
    board.step_ball(0.05);
    assert!(!board.contact_at(cx), "올라가는 공을 때렸다");
}

#[test]
fn 정점은_천장을_넘지_않는다() {
    let court = 코트();
    let 높이 = court.contact_y() - court.ceiling_y();
    for grade in VOLLEY_FLIGHT_MS {
        let t = flight_ms_for(&court, 0.0, grade) as f64 / 1_000.0;
        let 정점 = VOLLEY_GRAVITY * t * t / 8.0;
        assert!(
            정점 <= 높이 + 1e-6,
            "체공 {t}s 의 정점 {정점}이 천장 높이 {높이}를 넘는다"
        );
    }
}

#[test]
fn 진_팀은_공이_떨어진_쪽이다() {
    let (mut board, 위치) = 판(2, 17);
    board.serve(0, &위치);
    let 서버 = board.receiver().unwrap().0;
    let 서버_팀 = board.side_of(서버).unwrap();
    while !board.landed() {
        board.step_ball(0.05);
    }
    // 아무도 안 받았으므로 서브한 쪽 코트에 떨어진다.
    assert_eq!(board.loser(), 서버_팀);
}

#[test]
fn 안전_상한이_지나면_판이_만료된다() {
    let (board, _) = 판(2, 1);
    assert!(!board.expired(VOLLEY_MAX_MS - 1));
    assert!(board.expired(VOLLEY_MAX_MS));
}

#[test]
fn 마리가_빠지면_참여_목록에서_사라진다() {
    let (mut board, _) = 판(4, 1);
    assert_eq!(board.participants().len(), 4);
    board.leave(2);
    assert_eq!(board.participants().len(), 3);
    assert_eq!(board.side_of(2), None);
}

#[test]
fn 틱은_밀려도_상한을_넘지_않는다() {
    let (mut board, _) = 판(2, 1);
    // 10초가 밀려도 한 번에 `MAX_STEP_MS`만 정산한다 — 아니면 공이 순간이동한다.
    let dt = board.tick(10_000);
    assert!((dt - MAX_STEP_MS as f64 / 1_000.0).abs() < 1e-9);
}
