//! 볼링 판 — 국면·핀 자리·공 물리.

use super::test_support::*;
use super::*;

/// 화면 하나짜리 레인. 대부분의 테스트가 이걸 쓴다.
fn 레인() -> Bounds {
    BOUNDS
}

/// `n`마리가 있는 `Pets`.
fn 펭귄들(n: usize) -> Pets {
    let w = world();
    let mut pets = Pets::new();
    for i in 0..n {
        pets.add(7, 0, &w, BOUNDS.left + i as f64 * 10.0)
            .expect("상한 안에서는 추가된다");
    }
    pets
}

// ── 핀 자리 ────────────────────────────────────────────────────

#[test]
fn 핀_자리는_오른쪽부터_왼쪽으로_배정된다() {
    let xs = pin_positions(3, 레인());
    assert!(xs[0] > xs[1] && xs[1] > xs[2], "왼쪽으로 가며 줄어야 한다: {xs:?}");
    assert_eq!(xs[0], 레인().right - BOWLING_PIN_MARGIN);
    assert_eq!(xs[1], xs[0] - BOWLING_PIN_GAP);
}

#[test]
fn 한_마리면_핀이_하나다() {
    let xs = pin_positions(1, 레인());
    assert_eq!(xs.len(), 1);
    assert_eq!(xs[0], 레인().right - BOWLING_PIN_MARGIN);
}

#[test]
fn 마리가_없으면_핀도_없다() {
    assert!(pin_positions(0, 레인()).is_empty());
}

#[test]
fn 여덟_마리도_화면_안에_들어간다() {
    let lane = 레인();
    let xs = pin_positions(MAX_PETS, lane);
    assert_eq!(xs.len(), MAX_PETS);
    for x in xs {
        assert!(x >= lane.left && x <= lane.right, "{x}가 레인을 벗어났다");
    }
}

#[test]
fn 좁은_화면에서는_간격을_줄여_공_자리를_침범하지_않는다() {
    let lane = Bounds {
        left: 0.0,
        right: 400.0,
        top: 0.0,
        floor_y: 800.0,
    };
    let xs = pin_positions(MAX_PETS, lane);
    let 가장_왼쪽 = xs.last().copied().expect("여덟 자리가 나온다");
    assert!(
        가장_왼쪽 >= lane.left + BOWLING_LANE_MIN - 0.001,
        "공이 굴러올 길이 없다: 가장 왼쪽 핀이 {가장_왼쪽}"
    );
    assert!(
        xs[0] - xs[1] < BOWLING_PIN_GAP,
        "좁으면 간격이 줄어야 한다"
    );
}

#[test]
fn 레인이_아예_없어도_패닉하지_않는다() {
    let flat = Bounds {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        floor_y: 0.0,
    };
    let xs = pin_positions(MAX_PETS, flat);
    for x in xs {
        assert_eq!(x, 0.0);
    }
}

#[test]
fn 같은_마릿수는_항상_같은_자리_배정을_낳는다() {
    assert_eq!(pin_positions(5, 레인()), pin_positions(5, 레인()));
}

#[test]
fn 공은_레인_왼쪽_바닥에_놓인다() {
    let lane = 레인();
    let (x, y) = ball_home(lane);
    assert_eq!(x, lane.left + BOWLING_BALL_SIZE / 2.0, "공 왼쪽이 레인 왼쪽에 닿는다");
    assert_eq!(
        y + BOWLING_BALL_SIZE / 2.0,
        lane.floor_y + PET_SIZE,
        "공 밑면이 펭귄 발밑과 같은 바닥에 있어야 한다"
    );
    assert!(x < pin_positions(1, lane)[0], "공은 핀보다 왼쪽이다");
}

// ── 판의 수명 ──────────────────────────────────────────────────

#[test]
fn 볼링을_시작하면_전_마리가_참여한다() {
    let mut pets = 펭귄들(3);
    assert!(pets.start_bowling(1_000, 레인()));
    let board = pets.bowling().expect("판이 생긴다");
    assert_eq!(board.participants().len(), 3, "우클릭한 한 마리가 아니라 전부다");
    assert_eq!(board.phase(), BoardPhase::Gathering);
}

#[test]
fn 다_서기_전에는_공이_없다() {
    let mut pets = 펭귄들(3);
    pets.start_bowling(1_000, 레인());
    assert!(
        pets.bowling().and_then(|b| b.ball()).is_none(),
        "모으는 중에 공이 보이면 안 된다 (R4)"
    );
}

#[test]
fn 이미_볼링_중이면_다시_시작되지_않는다() {
    let mut pets = 펭귄들(2);
    assert!(pets.start_bowling(1_000, 레인()));
    assert!(
        !pets.start_bowling(2_000, 레인()),
        "판이 도는 중에 또 누르면 무시한다 (A3)"
    );
}

#[test]
fn 펭귄이_없으면_판이_열리지_않는다() {
    let mut pets = Pets::new();
    assert!(!pets.start_bowling(1_000, 레인()));
    assert!(pets.bowling().is_none());
}

#[test]
fn 판_도중_지운_마리의_자리는_비워진다() {
    let mut pets = 펭귄들(3);
    pets.start_bowling(1_000, 레인());
    let 지울 = pets.ids()[1];
    assert!(pets.remove(지울));
    let board = pets.bowling().expect("남은 둘로 판은 계속 돈다");
    assert!(!board.participants().contains(&지울));
    assert_eq!(board.participants().len(), 2);
}

#[test]
fn 참여_마리가_모두_사라지면_판이_끝난다() {
    let mut pets = 펭귄들(2);
    pets.start_bowling(1_000, 레인());
    for id in pets.ids() {
        pets.forget(id);
    }
    assert!(pets.bowling().is_none(), "참여 마리가 0이면 판을 접는다");
}

#[test]
fn 펭귄을_전부_끄면_판도_사라진다() {
    let mut pets = 펭귄들(3);
    pets.start_bowling(1_000, 레인());
    pets.clear();
    assert!(pets.bowling().is_none());
}

// ── 공 물리와 히트 판정 ────────────────────────────────────────

/// 판을 열고 **전부 설 때까지** 돌린다. 반환은 (마리들, 세계, 지금 시각).
fn 다_선_판(n: usize) -> (Pets, World, u64) {
    let w = world();
    let mut pets = 펭귄들(n);
    assert!(pets.start_bowling(0, 레인()));
    let mut t = 0;
    while t < 60_000 {
        t += 50;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_some_and(|b| b.phase() == BoardPhase::Ready) {
            return (pets, w, t);
        }
    }
    panic!("전부 서는 데 실패했다");
}

/// 판이 끝날 때까지 돌리고, 지나온 공 중심 x들을 돌려준다.
fn 굴린다(pets: &mut Pets, w: &World, from_ms: u64, vx: f64) -> Vec<f64> {
    pets.ball_drag_start();
    pets.ball_drag_end(from_ms, vx);
    let mut 궤적 = Vec::new();
    let mut t = from_ms;
    while t < from_ms + 60_000 {
        t += 50;
        pets.step_all(t, |_| Some(w));
        match pets.bowling().and_then(|b| b.ball()) {
            Some(ball) => 궤적.push(ball.x),
            None => break,
        }
        if pets.bowling().is_none() {
            break;
        }
    }
    궤적
}

fn 맞은_마리(pets: &Pets) -> Vec<PetId> {
    pets.ids()
        .into_iter()
        .filter(|id| {
            matches!(
                pets.get(*id).map(Pet::behavior),
                Some(Behavior::Bowling {
                    bowling: BowlingPhase::Struck
                })
            )
        })
        .collect()
}

#[test]
fn 전부_서면_공이_나타난다() {
    let (pets, _, _) = 다_선_판(3);
    let ball = pets.bowling().and_then(|b| b.ball()).expect("공이 놓인다");
    let (hx, hy) = ball_home(레인());
    assert_eq!((ball.x, ball.y), (hx, hy));
    assert!(!ball.rolling);
}

#[test]
fn 공은_수평으로만_굴러간다() {
    // 조준 각도가 없다 (R6) — 웹뷰가 세로 속도를 버리는지가 아니라, 코어가
    // 애초에 세로를 움직일 방법을 갖고 있지 않은지를 본다.
    let (mut pets, w, t) = 다_선_판(3);
    let y0 = pets.bowling().and_then(|b| b.ball()).unwrap().y;
    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_200.0);
    let mut tt = t;
    let mut 굴렀다 = false;
    while tt < t + 30_000 {
        tt += 50;
        pets.step_all(tt, |_| Some(&w));
        let Some(ball) = pets.bowling().and_then(|b| b.ball()) else {
            break;
        };
        assert_eq!(ball.y, y0, "공이 위아래로 움직였다");
        굴렀다 |= ball.x > 0.0;
    }
    assert!(굴렀다);
}

#[test]
fn 드래그가_빠를수록_멀리_간다() {
    let 거리 = |vx: f64| {
        let (mut pets, w, t) = 다_선_판(1);
        let 시작 = pets.bowling().and_then(|b| b.ball()).unwrap().x;
        let 궤적 = 굴린다(&mut pets, &w, t, vx);
        궤적.last().copied().unwrap_or(시작) - 시작
    };
    assert!(거리(1_000.0) > 거리(400.0), "세기가 거리를 정한다 (R5)");
    assert!(거리(400.0) > 0.0);
}

#[test]
fn 속도_상한은_세계_폭에_비례한다() {
    let 넓게 = clamp_roll(1_000_000.0, 3_000.0);
    let 좁게 = clamp_roll(1_000_000.0, 1_000.0);
    assert!(넓게 > 좁게, "화면이 넓으면 같은 손짓이 더 멀리 간다");
    assert_eq!(넓게, 3_000.0 * BOWLING_MAX_WORLDS_PER_SEC);
    assert_eq!(
        clamp_roll(-1_000_000.0, 3_000.0),
        -넓게,
        "방향은 유지한 채 자른다"
    );
    assert!(
        clamp_roll(1_000_000.0, 0.0) > 0.0,
        "세계 폭을 못 구해도 굴러는 간다"
    );
}

#[test]
fn 공은_반드시_멎는다() {
    // 정지 문턱이 없으면 20Hz 틱이 영영 안 쉰다.
    let (mut pets, w, t) = 다_선_판(1);
    let 궤적 = 굴린다(&mut pets, &w, t, 300.0);
    assert!(!궤적.is_empty());
    assert!(
        pets.bowling().is_none() || pets.bowling().unwrap().phase() != BoardPhase::Rolling,
        "60초 안에 판이 끝나야 한다"
    );
}

#[test]
fn 공이_지나간_펭귄만_맞는다() {
    let (mut pets, w, t) = 다_선_판(3);
    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_200.0);
    let mut tt = t;
    let mut 본_적 = Vec::new();
    while tt < t + 20_000 {
        tt += 50;
        pets.step_all(tt, |_| Some(&w));
        for id in 맞은_마리(&pets) {
            if !본_적.contains(&id) {
                본_적.push(id);
            }
        }
        if pets.bowling().is_none() {
            break;
        }
    }
    assert_eq!(본_적.len(), 3, "세게 굴리면 세 마리를 다 지나간다");
}

#[test]
fn 살살_굴리면_아무도_못_맞히고_판이_끝난다() {
    // AE3 — 재시도 버튼은 없다. 다시 하려면 "볼링 한 판"을 다시 누른다.
    let (mut pets, w, t) = 다_선_판(3);
    let mut tt = t;
    pets.ball_drag_start();
    pets.ball_drag_end(tt, BOWLING_MIN_ROLL_SPEED + 10.0);
    while tt < t + 30_000 {
        tt += 50;
        pets.step_all(tt, |_| Some(&w));
        assert!(맞은_마리(&pets).is_empty(), "공이 닿지도 않았는데 맞았다");
        if pets.bowling().is_none() {
            return;
        }
    }
    panic!("판이 안 끝났다");
}

#[test]
fn 펭귄을_맞혀도_공은_계속_간다() {
    // 첫 펭귄에서 멈추면 마릿수가 무의미해진다 (A2).
    let (mut pets, w, t) = 다_선_판(3);
    let 궤적 = 굴린다(&mut pets, &w, t, 1_200.0);
    let 마지막 = 궤적.last().copied().unwrap();
    let 첫_핀 = pin_positions(3, 레인())[2] + PET_SIZE / 2.0;
    assert!(마지막 > 첫_핀, "첫 핀({첫_핀})에서 멈췄다 — {마지막}");
}

#[test]
fn 같은_펭귄을_두_번_맞히지_않는다() {
    // 히트 반경 안에 여러 틱 머무는 것이 정상이므로, "이미 맞았다"를 판이
    // 기억하지 않으면 한 마리가 매 틱 다시 넘어지고 공도 계속 느려진다.
    let mut pins = std::collections::BTreeMap::new();
    pins.insert(1, 500.0);
    let mut board = Bowling::new(pins, 레인(), 0);
    board.open_ball();
    let 중심 = 500.0 + PET_SIZE / 2.0;
    assert!(board.grab());
    board.drag(중심 - ball_home(레인()).0);
    board.release(0, 100_000.0);
    assert!(board.hit(1, 중심), "반경 안에 들어오면 맞는다");
    assert!(!board.hit(1, 중심), "두 번째부터는 그냥 지나간다");
}

#[test]
fn 반경_밖의_펭귄은_맞지_않는다() {
    let mut pins = std::collections::BTreeMap::new();
    pins.insert(1, 500.0);
    let mut board = Bowling::new(pins, 레인(), 0);
    board.open_ball();
    let 중심 = 500.0 + PET_SIZE / 2.0;
    assert!(!board.hit(1, 중심 + BOWLING_HIT_RADIUS + 1.0));
    assert!(!board.hit(1, 중심 - BOWLING_HIT_RADIUS - 1.0));
}

#[test]
fn 공이_오른쪽_경계를_벗어나면_판이_끝난다() {
    let (mut pets, w, t) = 다_선_판(1);
    굴린다(&mut pets, &w, t, 100_000.0);
    assert!(pets.bowling().is_none(), "레인을 벗어나면 판이 끝난다 (A1)");
}

#[test]
fn 같은_초기_속도는_같은_결과를_낳는다() {
    // 공 물리에는 난수가 없다 — R12가 자동으로 만족된다.
    let 한_판 = || {
        let (mut pets, w, t) = 다_선_판(3);
        굴린다(&mut pets, &w, t, 1_100.0)
    };
    assert_eq!(한_판(), 한_판());
}

#[test]
fn 살살_놓으면_공이_그_자리에_남는다() {
    let (mut pets, w, t) = 다_선_판(2);
    let 시작 = pets.bowling().and_then(|b| b.ball()).unwrap().x;
    pets.ball_drag_start();
    pets.ball_drag_end(t, BOWLING_MIN_ROLL_SPEED - 1.0);
    pets.step_all(t + 50, |_| Some(&w));
    let ball = pets.bowling().and_then(|b| b.ball()).expect("공이 남아 있다");
    assert_eq!(ball.x, 시작);
    assert!(!ball.rolling, "굴러가지 않는다 — 다시 집을 수 있다");
    assert_eq!(pets.bowling().unwrap().phase(), BoardPhase::Ready);
}

#[test]
fn 공은_레인_밖으로_끌려_나가지_않는다() {
    let (mut pets, _, _) = 다_선_판(2);
    pets.ball_drag_start();
    pets.ball_drag_by(-100_000.0);
    let ball = pets.bowling().and_then(|b| b.ball()).unwrap();
    assert!(ball.x >= 레인().left, "끌고 나가면 다시 못 집는다");
    pets.ball_drag_by(100_000.0);
    let ball = pets.bowling().and_then(|b| b.ball()).unwrap();
    assert!(ball.x <= 레인().right + PET_SIZE);
}

#[test]
fn 굴러가는_공은_집을_수_없다() {
    let (mut pets, _, t) = 다_선_판(2);
    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_200.0);
    assert!(!pets.ball_drag_start(), "굴러가는 중에는 손이 안 닿는다");
}

#[test]
fn 판이_끝나면_전_마리가_흩어진다() {
    let (mut pets, w, t) = 다_선_판(3);
    굴린다(&mut pets, &w, t, 1_200.0);
    assert!(pets.bowling().is_none());
    for id in pets.ids() {
        assert!(
            matches!(
                pets.get(id).map(Pet::behavior),
                Some(Behavior::Bowling {
                    bowling: BowlingPhase::Scatter
                })
            ),
            "{id}번이 흩어지기를 건너뛰었다"
        );
    }
}

#[test]
fn 판_도중_드래그로_빼낸_마리는_판에서_빠진다() {
    // A4 — 억지로 자리로 되돌려 보내면 사용자와 앱이 싸운다.
    let (mut pets, w, t) = 다_선_판(3);
    let 뺄 = pets.ids()[0];
    pets.get_mut(뺄).unwrap().drag_start(t);
    pets.step_all(t + 50, |_| Some(&w));
    let board = pets.bowling().expect("남은 둘로 판은 계속 돈다");
    assert!(!board.participants().contains(&뺄));
}
