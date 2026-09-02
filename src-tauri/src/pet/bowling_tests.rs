//! 볼링 판 — 삼각 대형·국면·공 물리·연쇄.

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

// ── 삼각 대형 ──────────────────────────────────────────────────

#[test]
fn 줄마다_한_마리씩_늘어난다() {
    assert_eq!(triangle_rows(1), vec![1]);
    assert_eq!(triangle_rows(3), vec![1, 2]);
    assert_eq!(triangle_rows(6), vec![1, 2, 3]);
    assert_eq!(triangle_rows(MAX_PETS), vec![1, 2, 3, 2], "남은 줄은 모자란 채로 둔다");
    assert!(triangle_rows(0).is_empty());
}

#[test]
fn 핀은_삼각형으로_선다() {
    // 여섯 마리면 1·2·3의 완전한 삼각형이다.
    let xs: Vec<f64> = pin_positions(6, 레인()).into_iter().map(|(x, _)| x).collect();
    assert_eq!(xs.len(), 6);
    // 줄마다 x가 같고, 줄이 뒤로 갈수록 x가 커진다.
    assert!(xs[0] < xs[1], "꼭짓점이 둘째 줄보다 앞이다");
    assert_eq!(xs[1], xs[2], "둘째 줄 둘은 같은 x다");
    assert!(xs[1] < xs[3]);
    assert_eq!(xs[3], xs[4]);
    assert_eq!(xs[4], xs[5], "셋째 줄 셋은 같은 x다");
}

#[test]
fn 꼭짓점이_공이_오는_쪽을_향한다() {
    let pins = pin_positions(6, 레인());
    let 꼭짓점 = pins[0].0;
    assert!(
        pins.iter().skip(1).all(|(x, _)| *x > 꼭짓점),
        "꼭짓점이 가장 왼쪽이어야 공이 먼저 닿는다"
    );
    assert!(꼭짓점 > ball_home(레인()).0, "그래도 공보다는 오른쪽이다");
}

#[test]
fn 한_줄_안에서는_가운데를_기준으로_퍼진다() {
    let pins = pin_positions(3, 레인());
    let 중앙 = lane_center_y(레인());
    assert_eq!(pins[0].1, 중앙, "꼭짓점은 정확히 가운데다");
    let (위, 아래) = (pins[1].1, pins[2].1);
    assert!(위 < 중앙 && 아래 > 중앙);
    assert!(
        ((중앙 - 위) - (아래 - 중앙)).abs() < 0.001,
        "위아래가 대칭이어야 한다"
    );
}

#[test]
fn 판은_화면_세로_중앙에_선다() {
    // 바닥이 아니다 — 2차원 바닥은 선이라 삼각형을 만들 수 없다.
    let lane = 레인();
    let 중앙 = lane_center_y(lane);
    assert!(중앙 > lane.top && 중앙 < lane.floor_y);
    assert_eq!(중앙, (lane.top + lane.floor_y) / 2.0);
    assert!(
        pin_positions(6, lane).iter().all(|(_, y)| *y != lane.floor_y),
        "핀이 바닥에 서면 안 된다"
    );
}

#[test]
fn 한_마리면_꼭짓점_하나다() {
    let pins = pin_positions(1, 레인());
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].1, lane_center_y(레인()));
}

#[test]
fn 마리가_없으면_핀도_없다() {
    assert!(pin_positions(0, 레인()).is_empty());
}

#[test]
fn 여덟_마리도_화면_안에_들어간다() {
    let lane = 레인();
    let pins = pin_positions(MAX_PETS, lane);
    assert_eq!(pins.len(), MAX_PETS);
    for (x, y) in pins {
        assert!(x >= lane.left && x <= lane.right, "{x}가 레인을 벗어났다");
        assert!(y >= lane.top && y <= lane.floor_y, "{y}가 레인을 벗어났다");
    }
}

#[test]
fn 좁은_화면에서는_간격을_줄여_공_자리를_침범하지_않는다() {
    let lane = Bounds {
        left: 0.0,
        right: 400.0,
        top: 0.0,
        floor_y: 300.0,
    };
    let pins = pin_positions(MAX_PETS, lane);
    let 꼭짓점 = pins[0].0;
    assert!(
        꼭짓점 >= lane.left + BOWLING_LANE_MIN - 0.001,
        "공이 굴러올 길이 없다: 꼭짓점이 {꼭짓점}"
    );
    for (x, y) in pins {
        assert!(x >= lane.left && x <= lane.right);
        assert!(y >= lane.top && y <= lane.floor_y, "{y}가 위아래로 넘쳤다");
    }
}

#[test]
fn 레인이_아예_없어도_패닉하지_않는다() {
    let flat = Bounds {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        floor_y: 0.0,
    };
    for (x, y) in pin_positions(MAX_PETS, flat) {
        assert_eq!((x, y), (0.0, 0.0));
    }
}

#[test]
fn 같은_마릿수는_항상_같은_자리_배정을_낳는다() {
    assert_eq!(pin_positions(5, 레인()), pin_positions(5, 레인()));
}

#[test]
fn 공은_레인_왼쪽_판과_같은_높이에_놓인다() {
    let lane = 레인();
    let (x, y) = ball_home(lane);
    assert_eq!(x, lane.left + BOWLING_BALL_SIZE / 2.0, "공 왼쪽이 레인 왼쪽에 닿는다");
    assert_eq!(
        y,
        lane_center_y(lane) + PET_SIZE / 2.0,
        "공 중심이 핀 몸통 가운데와 같은 줄에 있어야 정면으로 맞힌다"
    );
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

/// 판을 열고 **전부 뜰 때까지** 돌린다. 반환은 (마리들, 세계, 지금 시각).
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
    panic!("전부 뜨는 데 실패했다");
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
    }
    궤적
}

/// 아직 판에 떠 있는 마리들.
fn 서_있는(pets: &Pets) -> Vec<PetId> {
    pets.ids()
        .into_iter()
        .filter(|id| {
            matches!(
                pets.get(*id).map(Pet::behavior),
                Some(Behavior::Bowling {
                    bowling: BowlingPhase::Ready
                })
            )
        })
        .collect()
}

#[test]
fn 전부_뜨면_공이_나타난다() {
    let (pets, _, _) = 다_선_판(3);
    let ball = pets.bowling().and_then(|b| b.ball()).expect("공이 놓인다");
    let (hx, hy) = ball_home(레인());
    assert_eq!((ball.x, ball.y), (hx, hy));
    assert!(!ball.rolling);
}

#[test]
fn 핀은_바닥이_아니라_공중에_뜬다() {
    let (pets, _, _) = 다_선_판(3);
    for id in pets.ids() {
        let s = pets.get(id).unwrap().snapshot();
        assert!(s.air, "{id}번이 떠 있지 않다");
        assert!(s.y < BOUNDS.floor_y, "{id}번이 바닥에 있다");
    }
}

#[test]
fn 공은_수평으로만_굴러간다() {
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
        assert_eq!(ball.y, y0, "공이 위아래로 움직였다 — 조준 각도가 없다 (R6)");
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
    assert_eq!(clamp_roll(-1_000_000.0, 3_000.0), -넓게, "방향은 유지한 채 자른다");
}

#[test]
fn 속도_상한의_바닥은_볼링_자기_상수다() {
    // 던지기의 `THROW_MIN_SPEED`를 빌려 쓰면 던지기를 튜닝할 때 볼링이 따라 바뀐다.
    assert_eq!(clamp_roll(1_000_000.0, 1.0), BOWLING_MIN_MAX_SPEED);
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
fn 맞은_핀은_튕겨_나간다() {
    // 제자리에서 도는 게 아니라 평소 던져졌을 때와 같은 물리를 탄다.
    let (mut pets, w, t) = 다_선_판(1);
    let id = pets.ids()[0];
    let 자리 = pets.get(id).unwrap().snapshot().x;
    pets.ball_drag_start();
    pets.ball_drag_end(t, 100_000.0);
    let mut tt = t;
    let mut 날았다 = false;
    while tt < t + 20_000 {
        tt += 50;
        pets.step_all(tt, |_| Some(&w));
        if pets.get(id).map(Pet::behavior) == Some(Behavior::Thrown) {
            날았다 = true;
            break;
        }
    }
    assert!(날았다, "맞은 핀이 Thrown이 되어야 한다");
    // 한 틱 더 진행하면 실제로 밀려나 있다.
    pets.step_all(tt + 50, |_| Some(&w));
    assert!(pets.get(id).unwrap().snapshot().x > 자리, "공이 온 쪽 반대로 밀린다");
}

#[test]
fn 연쇄로_옆_핀도_쓰러진다() {
    // 공이 지나는 줄에서 먼 핀은 직접 안 맞는다. 튕겨 나간 핀이 쳐야 쓰러진다 —
    // 이게 없으면 삼각형을 세운 보람이 없다.
    let (mut pets, w, t) = 다_선_판(6);
    let 처음 = 서_있는(&pets).len();
    assert_eq!(처음, 6);
    굴린다(&mut pets, &w, t, 100_000.0);
    let 남은 = pets
        .ids()
        .into_iter()
        .filter(|id| matches!(pets.get(*id).map(Pet::behavior), Some(Behavior::Bowling { .. })))
        .count();
    assert!(
        남은 < 5,
        "가운데 줄 하나만 쓰러졌다 — 연쇄가 안 걸렸다 (남은 {남은})"
    );
}

#[test]
fn 공이_지나는_줄에서_먼_핀은_직접_안_맞는다() {
    let mut pins = std::collections::BTreeMap::new();
    pins.insert(1, (500.0, lane_center_y(레인())));
    let mut board = Bowling::new(pins, 레인(), 0);
    board.open_ball();
    let 공 = board.ball().unwrap();
    assert!(
        board.ball_hit(공.x, 공.y + BOWLING_HIT_RADIUS + 1.0).is_none(),
        "세로로 멀면 공이 그냥 지나간다"
    );
    assert!(board.ball_hit(공.x, 공.y).is_some(), "같은 줄이면 맞는다");
}

#[test]
fn 틱이_밀려도_핀을_지나치지_않는다() {
    // 20Hz 틱이 밀리면 한 step이 최대 `MAX_STEP_MS`(250ms)를 정산한다. 그때
    // **지금 위치만** 보고 판정하면 히트 반경(52px)보다 좁은 핀을 통째로 뛰어넘는다.
    let mut pins = std::collections::BTreeMap::new();
    pins.insert(1, (500.0, lane_center_y(레인())));
    let mut board = Bowling::new(pins, 레인(), 0);
    board.open_ball();
    let 중심_y = board.ball().unwrap().y;
    let 중심_x = 500.0 + PET_SIZE / 2.0;
    let 출발 = 중심_x - BOWLING_HIT_RADIUS * 2.0;
    assert!(board.grab());
    board.drag(출발 - ball_home(레인()).0);
    board.release(0, 100_000.0);

    board.roll(MAX_STEP_MS as f64 / 1000.0);
    let 도착 = board.ball().expect("공이 있다").x;
    assert!(
        (도착 - 중심_x).abs() > BOWLING_HIT_RADIUS,
        "한 틱에 핀을 건너뛰는 상황이어야 의미 있는 테스트다 (도착 {도착})"
    );
    assert!(
        board.ball_hit(중심_x, 중심_y).is_some(),
        "지나온 구간 안에 있던 핀을 놓쳤다 — 점이 아니라 구간으로 재야 한다"
    );
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
        assert!(
            pets.ids()
                .into_iter()
                .all(|id| pets.get(id).map(Pet::behavior) != Some(Behavior::Thrown)),
            "공이 닿지도 않았는데 튕겨 나갔다"
        );
        if pets.bowling().is_none() {
            return;
        }
    }
    panic!("판이 안 끝났다");
}

#[test]
fn 스트라이크가_나도_공이_끝까지_굴러간다() {
    // 핀이 하나도 안 남았다고 판을 바로 접으면 공 창이 날아가는 중에 닫힌다.
    let (mut pets, w, t) = 다_선_판(1);
    let 궤적 = 굴린다(&mut pets, &w, t, 100_000.0);
    assert!(궤적.len() > 3, "공이 몇 틱은 더 굴러야 한다: {궤적:?}");
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
fn 집지_않은_공은_놓아도_굴러가지_않는다() {
    // 빠르게 튕기면 `pointerup`이 `ball_drag_start`의 왕복보다 먼저 도착해,
    // 집기가 거절됐는데도 놓기가 온다. 웹뷰가 보내는 것을 그대로 믿으면 안 된다.
    let (mut pets, w, t) = 다_선_판(2);
    let 시작 = pets.bowling().and_then(|b| b.ball()).unwrap().x;
    pets.ball_drag_end(t, 100_000.0); // 집지 않고 놓기만
    pets.step_all(t + 50, |_| Some(&w));
    let board = pets.bowling().expect("판이 살아 있어야 한다");
    assert_eq!(board.phase(), BoardPhase::Ready, "굴러가기 시작하면 안 된다");
    assert_eq!(board.ball().unwrap().x, 시작);
}

#[test]
fn 굴러가는_공에_놓기가_와도_속도가_안_바뀐다() {
    let (mut pets, w, t) = 다_선_판(3);
    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_200.0);
    let mut tt = t;
    for _ in 0..4 {
        tt += 50;
        pets.step_all(tt, |_| Some(&w));
    }
    let 중간 = pets.bowling().and_then(|b| b.ball()).unwrap().x;
    // 늦게 도착한 놓기 — 아주 살살 놓은 값이라 그대로 반영되면 공이 멎는다.
    pets.ball_drag_end(tt, 1.0);
    tt += 50;
    pets.step_all(tt, |_| Some(&w));
    let board = pets.bowling().expect("판이 살아 있어야 한다");
    assert_eq!(board.phase(), BoardPhase::Rolling, "판이 끊기면 안 된다");
    assert!(board.ball().unwrap().x > 중간, "공이 계속 굴러가야 한다");
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
fn 판이_끝나면_아무도_볼링_중이_아니다() {
    let (mut pets, w, t) = 다_선_판(3);
    굴린다(&mut pets, &w, t, 1_200.0);
    assert!(pets.bowling().is_none());
    // 흩어지기는 한 국면이라 조금 더 돌려 평소로 돌아가는 것까지 본다.
    let mut tt = t + 60_000;
    for _ in 0..40 {
        tt += 100;
        pets.step_all(tt, |_| Some(&w));
    }
    for id in pets.ids() {
        assert!(
            !matches!(pets.get(id).map(Pet::behavior), Some(Behavior::Bowling { .. })),
            "{id}번이 판이 끝났는데도 볼링 중이다"
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

#[test]
fn 판을_방치하면_스스로_접힌다() {
    // 마리별 안전 상한만으로는 부족하다 — 서로 다른 시각에 만료되면 마지막
    // 한 마리가 빠질 때까지 판과 공 창이 화면에 남는다 (R11).
    let (mut pets, w, t) = 다_선_판(3);
    let mut tt = t;
    while tt < t + BOWLING_MAX_MS + 5_000 {
        tt += 200;
        pets.step_all(tt, |_| Some(&w));
        if pets.bowling().is_none() {
            return;
        }
    }
    panic!("아무도 굴리지 않은 판이 스스로 접히지 않았다");
}

#[test]
fn 판을_강제로_접으면_전_마리가_흩어진다() {
    // 브릿지가 공 창을 못 만들었을 때 쓰는 길이다.
    let (mut pets, _, t) = 다_선_판(3);
    pets.end_bowling(t);
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

// ── 연쇄 (넓은 화면 포함) ──────────────────────────────────────

/// 확장 모니터만 한 레인.
fn 넓은_레인() -> Bounds {
    Bounds {
        left: 52.0,
        right: 2_800.0,
        top: 105.0,
        floor_y: 1_550.0,
    }
}

#[test]
fn 이웃한_핀은_연쇄_반경_안에_있다() {
    // 이게 깨지면 나란히 선 핀들이 서로 안 닿아 공이 지나는 한 줄만 쓰러진다.
    // 실제로 반경이 96일 때 그랬다 (2026-09-02 사용자 보고).
    for lane in [레인(), 넓은_레인()] {
        let pins = pin_positions(6, lane);
        for (i, a) in pins.iter().enumerate() {
            let 이웃_있나 = pins.iter().enumerate().any(|(j, b)| {
                i != j
                    && (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
                        <= BOWLING_KNOCK_RADIUS * BOWLING_KNOCK_RADIUS
            });
            assert!(이웃_있나, "{i}번 핀에 닿는 이웃이 하나도 없다");
        }
    }
}

#[test]
fn 선분_거리는_지나온_구간_전체를_잰다() {
    let (d2, 가까운) = dist2_to_segment((100.0, 50.0), (0.0, 0.0), (200.0, 0.0));
    assert_eq!(d2, 2_500.0, "구간 한가운데를 스쳤다");
    assert_eq!(가까운, (100.0, 0.0));

    // 끝점 밖은 끝점까지의 거리다 — 선을 무한히 늘려 잡으면 안 된다.
    let (d2, 가까운) = dist2_to_segment((300.0, 0.0), (0.0, 0.0), (200.0, 0.0));
    assert_eq!(d2, 10_000.0);
    assert_eq!(가까운, (200.0, 0.0));

    // 제자리에 있었으면 선분이 아니라 점이다 (0으로 나누지 않는다).
    let (d2, _) = dist2_to_segment((3.0, 4.0), (0.0, 0.0), (0.0, 0.0));
    assert_eq!(d2, 25.0);
}

#[test]
fn 넓은_화면에서도_연쇄가_걸린다() {
    // 튕기는 속도가 세계 폭에 비례하므로 확장 모니터에서는 한 틱에 140px 넘게
    // 난다. 판정이 점이면 이웃 옆을 스쳐 지나가면서도 안 잡힌다.
    let lane = 넓은_레인();
    let w = World::single(lane);
    let mut pets = Pets::new();
    for i in 0..6 {
        pets.add(7, 0, &w, lane.left + i as f64 * 10.0).unwrap();
    }
    assert!(pets.start_bowling(0, lane));

    let mut t = 0;
    while t < 60_000 {
        t += 50;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_some_and(|b| b.phase() == BoardPhase::Ready) {
            break;
        }
    }
    assert_eq!(pets.bowling().map(|b| b.phase()), Some(BoardPhase::Ready));

    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_000_000.0);
    while t < 120_000 {
        t += 50;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_none() {
            break;
        }
    }

    let 남은 = pets
        .ids()
        .into_iter()
        .filter(|id| {
            matches!(
                pets.get(*id).map(Pet::behavior),
                Some(Behavior::Bowling { .. })
            )
        })
        .count();
    assert_eq!(남은, 0, "확장 모니터에서 {남은}마리가 안 쓰러졌다");
}

#[test]
fn 틱이_밀려도_연쇄가_이웃을_뛰어넘지_않는다() {
    // 20Hz 틱이 밀리면 한 step이 최대 `MAX_STEP_MS`(250ms)를 정산한다. 확장
    // 모니터의 튕기는 속도(≈2600px/s)면 한 틱에 650px을 나는데, 연쇄 반경은
    // 126px이다 — 지금 위치만 보면 이웃을 통째로 뛰어넘는다.
    let lane = 넓은_레인();
    let w = World::single(lane);
    let mut pets = Pets::new();
    for i in 0..6 {
        pets.add(7, 0, &w, lane.left + i as f64 * 10.0).unwrap();
    }
    assert!(pets.start_bowling(0, lane));

    let mut t = 0;
    while t < 60_000 {
        t += MAX_STEP_MS;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_some_and(|b| b.phase() == BoardPhase::Ready) {
            break;
        }
    }
    pets.ball_drag_start();
    pets.ball_drag_end(t, 1_000_000.0);
    while t < 180_000 {
        t += MAX_STEP_MS;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_none() {
            break;
        }
    }

    let 남은 = pets
        .ids()
        .into_iter()
        .filter(|id| {
            matches!(
                pets.get(*id).map(Pet::behavior),
                Some(Behavior::Bowling { .. })
            )
        })
        .count();
    assert_eq!(남은, 0, "밀린 틱에서 {남은}마리가 연쇄를 피했다");
}

// ── 마릿수 1~8 전수 ────────────────────────────────────────────

/// `count`마리로 한 판을 끝까지 돌리고 (다 서기까지 걸린 ms, 안 쓰러진 수)를 준다.
fn 한_판(count: usize, lane: Bounds, dt: u64, vx: f64) -> (u64, usize) {
    let w = World::single(lane);
    let mut pets = Pets::new();
    for i in 0..count {
        pets.add(7, 0, &w, lane.left + i as f64 * 10.0)
            .expect("상한 안에서는 추가된다");
    }
    assert!(pets.start_bowling(0, lane), "{count}마리: 판이 안 열렸다");

    let mut t = 0;
    let mut 다_선_때 = None;
    while t < 60_000 {
        t += dt;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_some_and(|b| b.phase() == BoardPhase::Ready) {
            다_선_때 = Some(t);
            break;
        }
    }
    let 다_선_때 = 다_선_때.unwrap_or_else(|| panic!("{count}마리: 전부 뜨지 못했다"));

    assert!(pets.ball_drag_start(), "{count}마리: 공을 못 집었다");
    pets.ball_drag_end(t, vx);
    while t < 180_000 {
        t += dt;
        pets.step_all(t, |_| Some(&w));
        if pets.bowling().is_none() {
            break;
        }
    }
    assert!(pets.bowling().is_none(), "{count}마리: 판이 안 끝났다");

    let 남은 = pets
        .ids()
        .into_iter()
        .filter(|id| {
            matches!(
                pets.get(*id).map(Pet::behavior),
                Some(Behavior::Bowling { .. })
            )
        })
        .count();
    (다_선_때, 남은)
}

#[test]
fn 한_마리부터_여덟_마리까지_대형이_성립한다() {
    for lane in [레인(), 넓은_레인()] {
        for count in 1..=MAX_PETS {
            let pins = pin_positions(count, lane);
            assert_eq!(pins.len(), count, "{count}마리: 자리 수가 안 맞는다");
            // 자리가 겹치지 않는다 — 겹치면 한 마리로 보인다.
            for (i, a) in pins.iter().enumerate() {
                for b in pins.iter().skip(i + 1) {
                    assert!(
                        (a.0 - b.0).abs() > 0.001 || (a.1 - b.1).abs() > 0.001,
                        "{count}마리: 자리가 겹쳤다 {a:?}"
                    );
                }
                assert!(
                    a.0 >= lane.left && a.0 <= lane.right,
                    "{count}마리: {a:?}가 좌우를 벗어났다"
                );
                assert!(
                    a.1 >= lane.top && a.1 <= lane.floor_y,
                    "{count}마리: {a:?}가 위아래를 벗어났다"
                );
            }
        }
    }
}

#[test]
fn 한_마리부터_여덟_마리까지_세게_굴리면_다_쓰러진다() {
    for lane in [레인(), 넓은_레인()] {
        for count in 1..=MAX_PETS {
            let (_, 남은) = 한_판(count, lane, 50, 1_000_000.0);
            assert_eq!(남은, 0, "{count}마리: {남은}마리가 안 쓰러졌다");
        }
    }
}

#[test]
fn 한_마리부터_여덟_마리까지_틱이_밀려도_다_쓰러진다() {
    for lane in [레인(), 넓은_레인()] {
        for count in 1..=MAX_PETS {
            let (_, 남은) = 한_판(count, lane, MAX_STEP_MS, 1_000_000.0);
            assert_eq!(남은, 0, "{count}마리(밀린 틱): {남은}마리가 안 쓰러졌다");
        }
    }
}

#[test]
fn 한_마리부터_여덟_마리까지_살살_굴리면_아무도_안_쓰러진다() {
    // 반대쪽 극단도 본다 — 세기가 아무 의미 없이 다 쓰러지면 그건 게임이 아니다.
    for lane in [레인(), 넓은_레인()] {
        for count in 1..=MAX_PETS {
            let (_, 남은) = 한_판(count, lane, 50, BOWLING_MIN_ROLL_SPEED + 10.0);
            assert_eq!(남은, count, "{count}마리: 닿지도 않았는데 쓰러졌다");
        }
    }
}

#[test]
fn 한_마리부터_여덟_마리까지_금방_모인다() {
    // 다 서는 데 오래 걸리면 판이 시작되기 전에 지친다 (AE1은 2~4초를 말한다).
    for lane in [레인(), 넓은_레인()] {
        for count in 1..=MAX_PETS {
            let (다_선_때, _) = 한_판(count, lane, 50, 1_000_000.0);
            assert!(
                다_선_때 <= 8_000,
                "{count}마리: 다 서는 데 {다_선_때}ms나 걸렸다"
            );
        }
    }
}
