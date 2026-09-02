use crate::pet::test_support::*;
use crate::pet::*;

/// 오른쪽 끝 근처의 핀 자리.
const 핀: f64 = 900.0;

fn 핀이_된_펭귄() -> Pet {
    let mut p = Pet::new(7, 0, &world());
    p.x = 100.0;
    assert!(p.start_bowling(0, 핀, BOUNDS.floor_y));
    p
}

/// 볼링 국면만 순서대로 모은다 (연속 중복은 접는다).
fn 국면들(p: &mut Pet, 끝ms: u64) -> Vec<BowlingPhase> {
    let w = world();
    let mut out: Vec<BowlingPhase> = Vec::new();
    let mut t = 0;
    while t <= 끝ms {
        if let Behavior::Bowling { bowling } = p.step(t, &w).behavior {
            if out.last() != Some(&bowling) {
                out.push(bowling);
            }
        }
        t += 50;
    }
    out
}

#[test]
fn 볼링을_시작하면_자기_자리로_걸어간다() {
    let mut p = 핀이_된_펭귄();
    let 시작 = p.snapshot().x;
    let s = p.step(200, &world());
    assert!(matches!(
        s.behavior,
        Behavior::Bowling {
            bowling: BowlingPhase::Gather
        }
    ));
    assert!(s.x > 시작, "핀 자리 쪽으로 움직여야 한다");
    assert!(s.x < 핀, "한 틱에 순간이동하면 안 된다 (R2)");
}

#[test]
fn 자리에_도착하면_서서_기다린다() {
    let mut p = 핀이_된_펭귄();
    let 국면 = 국면들(&mut p, 20_000);
    assert_eq!(국면.first(), Some(&BowlingPhase::Gather));
    assert!(
        국면.contains(&BowlingPhase::Ready),
        "도착하면 Ready로 넘어가야 한다: {국면:?}"
    );
    assert!((p.snapshot().x - 핀).abs() <= ARRIVE_EPSILON);
}

#[test]
fn 핀_자리가_왼쪽이면_왼쪽을_보고_걷는다() {
    let mut p = Pet::new(7, 0, &world());
    p.x = 800.0;
    assert!(p.start_bowling(0, 100.0, BOUNDS.floor_y));
    let s = p.step(200, &world());
    assert_eq!(s.facing, Facing::Left, "뒷걸음질로 보이면 안 된다");
    assert!(s.x < 800.0);
}

#[test]
fn 다_서면_공이_오는_쪽을_본다() {
    let mut p = 핀이_된_펭귄();
    국면들(&mut p, 20_000);
    assert_eq!(
        p.snapshot().facing,
        Facing::Left,
        "핀은 공이 굴러오는 왼쪽을 본다"
    );
}

#[test]
fn 공중에_있어도_바닥까지_걸어_내려온다() {
    let w = world();
    let mut p = Pet::new(7, 0, &w);
    p.enter_swim(0, BOUNDS);
    p.y = BOUNDS.top;
    p.air = true;
    assert!(p.start_bowling(0, 핀, BOUNDS.floor_y));

    let 첫틱 = p.step(50, &w);
    assert!(
        첫틱.y < BOUNDS.floor_y,
        "바닥으로 순간이동하면 R2를 그 자리에서 어긴다"
    );
    국면들(&mut p, 20_000);
    let s = p.snapshot();
    assert_eq!(s.y, BOUNDS.floor_y, "결국은 바닥에 선다");
    assert!(!s.air);
}

#[test]
fn 공에_맞으면_빙글빙글_돈다() {
    let mut p = 핀이_된_펭귄();
    국면들(&mut p, 20_000);
    p.bowling_struck(20_000);
    assert!(matches!(
        p.snapshot().behavior,
        Behavior::Bowling {
            bowling: BowlingPhase::Struck
        }
    ));
}

#[test]
fn 맞은_뒤에는_다시_맞지_않는다() {
    let mut p = 핀이_된_펭귄();
    국면들(&mut p, 20_000);
    p.bowling_struck(20_000);
    p.bowling_scatter(20_100);
    p.bowling_struck(20_200);
    assert!(
        matches!(
            p.snapshot().behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Scatter
            }
        ),
        "흩어지는 중에 다시 맞으면 판이 되감긴다"
    );
}

#[test]
fn 모든_국면은_흩어지기로_끝난다() {
    // 판이 몰아 주는 경우
    let mut p = 핀이_된_펭귄();
    국면들(&mut p, 20_000);
    p.bowling_scatter(20_000);
    let mut 국면 = vec![BowlingPhase::Scatter];
    국면.extend(국면들(&mut p, 20_000 + BOWLING_SCATTER_MS + 500));
    assert_eq!(국면.last(), Some(&BowlingPhase::Scatter), "{국면:?}");
    assert!(
        !matches!(p.snapshot().behavior, Behavior::Bowling { .. }),
        "흩어지기가 끝나면 평소로 돌아간다"
    );
}

#[test]
fn 판이_사라져도_영원히_서_있지_않는다() {
    // 판(`Pets::bowling`)이 어떤 이유로 없어져도 마리가 스스로 빠져나온다 (R11).
    let mut p = 핀이_된_펭귄();
    // 상한은 국면에 **들어간 시각**부터 센다. 걸어가는 시간까지 넉넉히 넘긴다.
    let 국면 = 국면들(&mut p, 2 * BOWLING_MAX_MS);
    assert_eq!(국면.last(), Some(&BowlingPhase::Scatter), "{국면:?}");
    assert!(!matches!(p.snapshot().behavior, Behavior::Bowling { .. }));
}

#[test]
fn 볼링_중_드래그하면_판에서_빠진다() {
    let mut p = 핀이_된_펭귄();
    p.step(200, &world());
    p.drag_start(300);
    assert_eq!(p.snapshot().behavior, Behavior::Dragged, "A4");
}

#[test]
fn 들려_있으면_볼링이_시작되지_않는다() {
    let mut p = pet();
    p.drag_start(0);
    assert!(!p.start_bowling(100, 핀, BOUNDS.floor_y));
}

#[test]
fn 이미_볼링_중이면_start_bowling이_거짓을_준다() {
    let mut p = 핀이_된_펭귄();
    assert!(!p.start_bowling(1_000, 핀, BOUNDS.floor_y));
}

#[test]
fn 볼링하는_동안_경계를_넘지_않는다() {
    let w = world();
    for 목표 in [BOUNDS.left - 500.0, BOUNDS.right + 500.0] {
        let mut p = Pet::new(7, 0, &w);
        p.x = 500.0;
        assert!(p.start_bowling(0, 목표, BOUNDS.floor_y));
        for s in drive(&mut p, 0, 20_000, 50, &w) {
            assert!(
                s.x >= BOUNDS.left && s.x <= BOUNDS.right,
                "목표 {목표}에서 {}가 경계를 넘었다",
                s.x
            );
        }
    }
}

#[test]
fn 볼링은_pick_next에서_뽑히지_않는다() {
    // 확률 사다리에 끼우면 튜닝된 빈도표가 통째로 밀린다 (KTD1).
    for seed in 1u64..8 {
        for s in 삼십분(seed) {
            assert!(
                !matches!(s.behavior, Behavior::Bowling { .. }),
                "시드 {seed}: 저절로 볼링이 나왔다"
            );
        }
    }
}

#[test]
fn 흩어지는_중에는_새_판에_낄_수_있다() {
    // 판이 끝나면 곧바로 버튼이 살아나는데, 흩어지는 0.6초 동안 새 판을
    // 거절하면 눌리는데 아무 일도 안 일어나는 구간이 생긴다.
    let mut p = 핀이_된_펭귄();
    국면들(&mut p, 20_000);
    p.bowling_scatter(20_000);
    assert!(
        p.start_bowling(20_100, 핀, BOUNDS.floor_y),
        "흩어지는 중에는 다시 낄 수 있어야 한다"
    );
    assert!(matches!(
        p.snapshot().behavior,
        Behavior::Bowling {
            bowling: BowlingPhase::Gather
        }
    ));
}
