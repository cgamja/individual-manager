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
