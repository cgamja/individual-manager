use super::*;
use crate::pet::{Bounds, World, PET_SIZE};
use std::collections::HashMap;

fn 경계(right: f64) -> Bounds {
    Bounds {
        left: 0.0,
        right,
        top: 0.0,
        floor_y: 800.0,
    }
}

/// 자세 판정만 보는 최소 스냅샷.
fn 스냅샷(behavior: crate::pet::Behavior, air: bool) -> Snapshot {
    Snapshot {
        x: 0.0,
        y: 0.0,
        facing: crate::pet::Facing::Right,
        vertical: crate::pet::Vertical::Level,
        air,
        speech: None,
        whack_seq: 0,
        pinball: false,
        behavior,
    }
}

#[test]
fn 서_있는_국면은_상자_안이다() {
    use crate::pet::Behavior::*;
    for b in [Walk, Sleep, Swing, Squawk] {
        assert!(pose_of(&스냅샷(b, false)).in_box(), "{b:?}");
    }
}

#[test]
fn 그림이_상자를_넘는_국면은_통과를_접는다() {
    // 이 목록이 비면 슬라이딩·굴러떨어지기 중에 그려진 펭귄을 눌렀는데 클릭이
    // 아래 앱으로 샌다 — 이 설계가 없애려던 갈래다.
    //
    // **판·경기 동작도 여기 든다.** 코어 좌표는 바닥에 두고 CSS로만 그리므로
    // 스파이크(`translateY(-26px)`)나 핀 자세(`rotate(48deg)`)가 되돌리기
    // 여유(`PET_SIZE * 0.1`)를 넘는다.
    use crate::pet::Behavior::*;
    for b in [
        Slide,
        Tumble,
        Splat,
        Sprawl,
        Thrown,
        Dragged,
        Land,
        Falling,
        Volleyball {
            volley: crate::pet::VolleyPhase::Bump,
        },
        Volleyball {
            volley: crate::pet::VolleyPhase::Chase,
        },
        Bowling {
            bowling: crate::pet::BowlingPhase::Ready,
        },
    ] {
        assert!(!pose_of(&스냅샷(b, false)).in_box(), "{b:?}");
    }
}

#[test]
fn 모르는_동작은_통과를_접는다() {
    // 목록을 **뒤집어** 뒀다: 상자 안이 확인된 국면만 통과를 허락한다.
    // 모션은 일곱 자리에 흩어져 있어서(`behavior.rs`) 새 동작을 얹을 때 이
    // 목록을 빠뜨리기 쉬운데, 빠뜨린 쪽이 안전한 기본값(= 오늘까지의 동작)이
    // 되어야 한다. 이 테스트가 그 방향을 못 박는다.
    use crate::pet::{Behavior::*, FreakoutPhase};
    assert!(!pose_of(&스냅샷(
        Freakout {
            freakout: FreakoutPhase::Dash
        },
        false
    ))
    .in_box());
    assert!(!pose_of(&스냅샷(Swim, false)).in_box());
}

#[test]
fn 공중에_있으면_동작과_무관하게_통과를_접는다() {
    // 헤엄은 오르내릴 때 SVG 루트가 통째로 기운다(`air.css`).
    assert!(!pose_of(&스냅샷(crate::pet::Behavior::Swim, true)).in_box());
}

#[test]
fn 시작할_때는_아무도_클릭을_통과시키지_않는다() {
    // 통과는 근거가 있을 때만 켜지는 상태다. 기본이 "클릭을 먹는다"여야
    // 어떤 실패에서든 펭귄을 누를 수 있다 (R6).
    let state = PetState::new(crate::pet::Pets::new());
    assert!(state.click_through.lock().unwrap().is_empty());
}

#[test]
fn 경계를_못_읽으면_주_모니터로_떨어진다() {
    let 주 = World::single(경계(1_440.0));
    let got = world_to_cache(None, || Some(주.clone()));
    assert_eq!(
        got.map(|w| w.first().bounds.right),
        Some(1_440.0),
        "읽기에 실패했으면 주 모니터로 떨어져야 한다"
    );
}

#[test]
fn 경계를_읽었으면_그대로_쓴다() {
    let 지금 = World::single(경계(3_008.0));
    let 주 = World::single(경계(1_440.0));
    let got = world_to_cache(Some(지금), || Some(주));
    assert_eq!(got.map(|w| w.first().bounds.right), Some(3_008.0));
}

#[test]
fn 둘_다_못_읽으면_캐시를_건드리지_않는다() {
    assert!(world_to_cache(None, || None).is_none());
}

#[test]
fn 크기가_0인_작업_영역은_모니터로_치지_않는다() {
    assert!(bounds_of_work_area((0, 0), (0, 0), 1.0, 1.0).is_none());
    assert!(bounds_of_work_area((0, 0), (1_440, 0), 2.0, 1.0).is_none());
    assert!(bounds_of_work_area((0, 0), (0, 900), 2.0, 1.0).is_none());
    assert!(
        bounds_of_work_area((0, 0), (2_880, 1_800), 2.0, 1.0).is_some(),
        "멀쩡한 모니터는 통과해야 한다"
    );
}

#[test]
fn 덮개는_화면_전체를_배율로_나눈다() {
    let rects = pinball_rects_of(&[((0, 0), (2_880, 1_800), 2.0)]);
    assert_eq!(rects, vec![(0.0, 0.0, 1_440.0, 900.0)]);
}

#[test]
fn 화면마다_판을_하나씩_깐다() {
    let rects = pinball_rects_of(&[
        ((0, 0), (2_880, 1_800), 2.0),
        ((1_800, -333), (3_008, 1_692), 1.0),
    ]);
    assert_eq!(
        rects,
        vec![
            (0.0, 0.0, 1_440.0, 900.0),
            (1_800.0, -333.0, 3_008.0, 1_692.0),
        ]
    );
}

#[test]
fn 크기가_0인_화면은_건너뛴다() {
    assert!(pinball_rects_of(&[]).is_empty());
    assert!(pinball_rects_of(&[((0, 0), (0, 900), 2.0)]).is_empty());
    assert!(pinball_rects_of(&[((0, 0), (1_440, 900), 0.0)]).is_empty());
    assert_eq!(
        pinball_rects_of(&[((0, 0), (0, 0), 1.0), ((100, 0), (200, 200), 1.0)]),
        vec![(100.0, 0.0, 200.0, 200.0)],
        "멀쩡한 화면 하나는 남는다"
    );
}

/// 새 창의 라벨을 capabilities에 안 넣으면 그 창이 부르는 커맨드가
/// **런타임에서만 조용히 reject된다** — 아래 커맨드 등록 누락과 같은 부류다.
/// 덮개는 Esc로 `pet_set_pinball`을 부르므로 여기 걸리면 나가는 문 하나가 죽는다.
#[test]
fn 덮개_라벨이_capabilities에_등록되어_있다() {
    let capabilities = include_str!("../../capabilities/default.json");
    assert!(
        capabilities.contains(&format!("{PINBALL_LABEL_PREFIX}*")),
        "`{PINBALL_LABEL_PREFIX}*` 글롭이 capabilities의 windows 목록에 없다"
    );
    assert_eq!(pinball_label(0), "pinball-board-0");
}

/// 공 창도 같은 부류다 — 라벨이 capabilities에 없으면 공을 집는 순간
/// `ball_drag_start`가 조용히 reject되고, 공은 끌리지 않는데 오류도 안 난다.
#[test]
fn 공_창_라벨이_capabilities에_등록되어_있다() {
    let capabilities = include_str!("../../capabilities/default.json");
    assert!(
        capabilities.contains(&format!("\"{BALL_LABEL}\"")),
        "`{BALL_LABEL}`이 capabilities의 windows 목록에 없다"
    );
}

/// 공 창의 창 좌표는 **중심**에서 나온다. 좌상단으로 착각하면 공이 반 칸씩
/// 어긋나 눈으로는 보이지만 히트 판정과 어긋난다.
#[test]
fn 공_창은_중심을_기준으로_놓인다() {
    let (x, y) = ball_window_origin(500.0, 800.0, 1.0);
    assert_eq!(x, 500.0 - BALL_WINDOW_SIZE / 2.0);
    assert_eq!(y, 800.0 - BALL_WINDOW_SIZE / 2.0);
}

/// 공 창은 크기와 자리가 **함께** 배율을 타야 한다. 한쪽만 타면 공이 손에서
/// 반 칸 어긋난 채로 굴러간다.
#[test]
fn 공_창의_크기와_자리가_함께_배율을_탄다() {
    assert_eq!(ball_window_size(0.5), BALL_WINDOW_SIZE / 2.0);
    let (x, y) = ball_window_origin(500.0, 800.0, 0.5);
    assert_eq!(x, 250.0 - BALL_WINDOW_SIZE / 4.0);
    assert_eq!(y, 400.0 - BALL_WINDOW_SIZE / 4.0);
}

/// 위치는 `BallLook`에 안 들어간다 — 넣으면 굴러가는 내내 20Hz로 리렌더한다.
#[test]
fn 공은_구르기_시작할_때만_웹뷰에_알린다() {
    use crate::pet::BallSnapshot;
    let 멈춤 = BallSnapshot {
        x: 0.0,
        y: 0.0,
        rolling: false,
        held: false,
    };
    let 옮김 = BallSnapshot { x: 900.0, ..멈춤 };
    let 구름 = BallSnapshot {
        rolling: true,
        ..멈춤
    };
    assert_eq!(ball_look_of(&멈춤), ball_look_of(&옮김));
    assert_ne!(ball_look_of(&멈춤), ball_look_of(&구름));
}

/// 판이 끝났다는 신호는 **공이 아니라 판**을 보고 낸다. 공은 전부 서기 전에는
/// 없으므로, 모으는 중에 참여 마리가 전부 빠지면 공 쪽 기억만으로는 끝난 줄을
/// 모르고 "볼링 한 판" 버튼이 비활성인 채로 남는다.
#[test]
fn 공이_나오기_전에_끝난_판도_끝났다고_알린다() {
    assert!(bowling_over(true, false), "모으는 중에 끝난 판도 알려야 한다");
    assert!(!bowling_over(false, false), "없던 판은 끝날 것도 없다");
    assert!(!bowling_over(true, true), "도는 중에는 알리지 않는다");
    assert!(!bowling_over(false, true), "막 열린 판은 끝난 게 아니다");
}

/// 창 만들기 재시도에 끝이 없으면 20Hz로 영원히 두드리면서 사용자에게는
/// 아무 신호도 안 간다.
#[test]
fn 공_창_실패_재시도에_상한이_있다() {
    assert!(BALL_WINDOW_MAX_FAILS > 0, "한 번은 시도해야 한다");
    assert!(
        BALL_WINDOW_MAX_FAILS < 20,
        "20Hz 틱에서 20번이면 1초 넘게 굳은 펭귄을 보게 된다"
    );
}

/// 등록을 빠뜨리면 컴파일도 되고 테스트도 통과하는데 런타임에서 모든 IPC가
/// reject된다 — 커맨드는 `pub`이라 dead_code 경고도 안 뜬다. 실제로 한 번
/// 놓쳤던 사각지대라 소스를 직접 대조한다.
#[test]
fn 모든_펫_커맨드가_invoke_handler에_등록되어_있다() {
    // 디렉터리를 실행 시점에 훑는다. 파일 목록을 손으로 들고 있으면 새 모듈에
    // 커맨드를 넣었을 때 이 테스트가 그 파일을 안 읽어 등록 누락을 놓친다 —
    // 막으려던 사각지대가 그대로 되살아난다.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pet_bridge");
    let mut bridge = String::new();
    for entry in std::fs::read_dir(&dir).expect("pet_bridge 디렉터리를 못 읽었다") {
        let path = entry.expect("디렉터리 항목을 못 읽었다").path();
        if path.extension().is_some_and(|e| e == "rs") {
            bridge.push_str(&std::fs::read_to_string(&path).expect("소스를 못 읽었다"));
            bridge.push('\n');
        }
    }
    let lib = include_str!("../lib.rs");

    let mut commands = Vec::new();
    let mut lines = bridge.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() != "#[tauri::command]" {
            continue;
        }
        let signature = lines.peek().expect("커맨드 속성 뒤에 함수가 없다");
        let name = signature
            .trim()
            .strip_prefix("pub fn ")
            .and_then(|rest| rest.split('(').next())
            .expect("`pub fn 이름(` 형태를 기대한다");
        commands.push(name.to_string());
    }

    assert!(
        !commands.is_empty(),
        "커맨드를 하나도 찾지 못했다 — 탐지가 깨졌다"
    );
    for name in commands {
        assert!(
            lib.contains(&format!("pet_bridge::commands::{name},")),
            "`{name}`이 lib.rs의 invoke_handler 목록에 없다"
        );
    }
}

/// 배율이 바뀐 틱은 시간이 남아 있어도 경계를 다시 재야 한다. 안 그러면 최대
/// 2초 동안 옛 경계로 clamp되어 작아진 펭귄이 벽에서 떨어져 선다.
#[test]
fn 배율이_바뀌면_세계_캐시를_다시_잰다() {
    // 방금 잰 캐시는 배율이 같으면 그대로 쓴다.
    assert!(!world_is_stale(Some((10_000, 1.0)), 10_100, 1.0));
    // 배율만 달라도 다시 잰다.
    assert!(world_is_stale(Some((10_000, 1.0)), 10_100, 0.6));
    // 시간이 지나면 배율이 같아도 다시 잰다.
    assert!(world_is_stale(Some((10_000, 1.0)), 10_000 + BOUNDS_REFRESH_MS, 1.0));
    // 캐시가 없으면 당연히 잰다.
    assert!(world_is_stale(None, 0, 1.0));
}

/// 배율이 바뀐 틱에는 **자는 마리도** 옮겨야 한다. 안 그러면 새 경계로 다시
/// clamp된 좌표가 창에 안 걸려 최대 25초 동안 허공에 뜨거나 화면 밖에 남는다.
#[test]
fn 배율이_바뀌면_자는_마리도_옮긴다() {
    // 평소에는 안 옮긴다.
    assert!(!should_move(false, false));
    // 구조됐거나 배율이 바뀌면 동작과 무관하게 옮긴다.
    assert!(should_move(false, true));
    assert!(should_move(true, false));
}

#[test]
fn 방금_어긋난_것은_정리하지_않는다() {
    let mut seen = HashMap::new();
    seen.insert(2, 10_000);
    assert!(
        due_for_cleanup(&seen, 10_050).is_empty(),
        "50ms만에 지우면 안 된다"
    );
    assert!(due_for_cleanup(&seen, 10_999).is_empty());
}

#[test]
fn 유예를_다_쓰면_정리한다() {
    let mut seen = HashMap::new();
    seen.insert(2, 10_000);
    assert_eq!(due_for_cleanup(&seen, 11_000), vec![2]);
    assert_eq!(due_for_cleanup(&seen, 30_000), vec![2]);
}

#[test]
fn 시계가_뒤로_가도_정리가_앞당겨지지_않는다() {
    let mut seen = HashMap::new();
    seen.insert(2, 10_000);
    assert!(
        due_for_cleanup(&seen, 9_000).is_empty(),
        "음수 대신 0으로 본다"
    );
}

#[test]
fn 라벨에서_펭귄_id를_뽑는다() {
    assert_eq!(pet_label(3), "pet-3");
    assert_eq!(pet_id_from_label("pet-3"), Some(3));
    assert_eq!(pet_id_from_label(&pet_label(42)), Some(42));
}

#[test]
fn 펫이_아닌_라벨은_id가_없다() {
    for label in ["main", "pet", "pet-", "pet-x", "pets-1", "", "PET-1"] {
        assert_eq!(pet_id_from_label(label), None, "`{label}`은 펫 창이 아니다");
    }
}

#[test]
fn 옆자리는_영역_안에_들어온다() {
    let b = Bounds {
        left: 0.0,
        right: 1_000.0,
        top: 0.0,
        floor_y: 800.0,
    };
    assert!(next_to(100.0, b) > 100.0);
    assert!(next_to(b.right, b) < b.right);
    for x in [b.left, 500.0, b.right] {
        let n = next_to(x, b);
        assert!(n >= b.left && n <= b.right, "{n}이 영역을 벗어났다");
    }
}

#[test]
fn 영역이_없어도_옆자리_계산이_패닉하지_않는다() {
    let flat = Bounds {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        floor_y: 0.0,
    };
    assert_eq!(next_to(0.0, flat), 0.0);
}

use crate::pet::{IdleKind, SassyKind, Vertical};

/// 1440x900 작업 영역, 360x540 팝오버, 140px 펭귄.
const AREA: (f64, f64, f64, f64) = (0.0, 25.0, 1440.0, 875.0);
const POP: (f64, f64) = (360.0, 540.0);

#[test]
fn 팝오버는_펭귄_오른쪽에_붙는다() {
    let (x, y) = popover_position_near((200.0, 120.0), PET_SIZE, POP, AREA);
    assert_eq!(x, 200.0 + PET_SIZE + POPOVER_GAP);
    assert_eq!(y, 120.0);
}

#[test]
fn 아래쪽_펭귄에서는_팝오버가_화면_안으로_올라온다() {
    let (_, y) = popover_position_near((200.0, 760.0), PET_SIZE, POP, AREA);
    assert_eq!(y, AREA.1 + AREA.3 - POP.1);
    assert!(y < 760.0, "펭귄보다 위로 올라와야 한다");
}

#[test]
fn 오른쪽이_모자라면_펭귄_왼쪽에_붙는다() {
    let (x, _) = popover_position_near((1200.0, 400.0), PET_SIZE, POP, AREA);
    assert_eq!(x, 1200.0 - POP.0 - POPOVER_GAP);
}

#[test]
fn 어느_위치에서도_화면을_벗어나지_않는다() {
    for px in [0.0, 300.0, 700.0, 1100.0, 1300.0] {
        for py in [25.0, 300.0, 700.0, 760.0] {
            let (x, y) = popover_position_near((px, py), PET_SIZE, POP, AREA);
            assert!(x >= AREA.0, "왼쪽으로 벗어남: {x}");
            assert!(
                x + POP.0 <= AREA.0 + AREA.2 + 0.001,
                "오른쪽으로 벗어남: {x}"
            );
            assert!(y >= AREA.1, "위로 벗어남: {y}");
            assert!(y + POP.1 <= AREA.1 + AREA.3 + 0.001, "아래로 벗어남: {y}");
        }
    }
}

#[test]
fn 팝오버가_영역보다_커도_패닉하지_않는다() {
    let tiny = (0.0, 0.0, 200.0, 200.0);
    let (x, y) = popover_position_near((10.0, 10.0), PET_SIZE, POP, tiny);
    assert_eq!((x, y), (0.0, 0.0));
}

#[test]
fn 겉모습이_그대로면_다시_알리지_않는다() {
    let look = (
        Behavior::Walk,
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    assert!(!should_notify(Some(look), look));
    assert!(should_notify(None, look), "처음에는 알려야 한다");
}

#[test]
fn 겉모습_비교에_핀볼이_들어간다() {
    let 꺼짐 = (
        Behavior::Walk,
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    let 켜짐 = (
        Behavior::Walk,
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        true,
    );
    assert!(should_notify(Some(꺼짐), 켜짐));
}

#[test]
fn 핀볼_설정이_없으면_꺼짐이다() {
    assert!(!pinball_from(None));
    assert!(!pinball_from(Some(&serde_json::json!({}))));
    assert!(!pinball_from(Some(
        &serde_json::json!({ "enabled": true, "count": 3 })
    )));
    assert!(
        !pinball_from(Some(&serde_json::json!({ "pinball": "true" }))),
        "문자열은 켜짐으로 읽지 않는다"
    );
    assert!(pinball_from(Some(&serde_json::json!({ "pinball": true }))));
}

#[test]
fn 테마_설정이_없으면_시스템이다() {
    assert_eq!(theme_from(None), Theme::System);
    assert_eq!(theme_from(Some(&serde_json::json!({}))), Theme::System);
}

#[test]
fn 저장된_테마를_그대로_읽는다() {
    assert_eq!(
        theme_from(Some(&serde_json::json!({ "theme": "light" }))),
        Theme::Light
    );
    assert_eq!(
        theme_from(Some(&serde_json::json!({ "theme": "dark" }))),
        Theme::Dark
    );
    assert_eq!(
        theme_from(Some(&serde_json::json!({ "theme": "system" }))),
        Theme::System
    );
}

#[test]
fn 깨진_테마는_시스템으로_수렴한다() {
    assert_eq!(
        theme_from(Some(&serde_json::json!({ "theme": "어둡게" }))),
        Theme::System
    );
    assert_eq!(
        theme_from(Some(&serde_json::json!({ "theme": 2 }))),
        Theme::System
    );
}

#[test]
fn 세로_방향만_바뀌어도_웹뷰에_알린다() {
    let up = (
        Behavior::Swim,
        Facing::Right,
        Vertical::Up,
        true,
        None,
        0,
        false,
    );
    let down = (
        Behavior::Swim,
        Facing::Right,
        Vertical::Down,
        true,
        None,
        0,
        false,
    );
    assert!(should_notify(Some(up), down));
}

#[test]
fn 좌우_방향만_바뀌어도_웹뷰에_알린다() {
    let right = (
        Behavior::Walk,
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    let left = (
        Behavior::Walk,
        Facing::Left,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    assert!(should_notify(Some(right), left));
}

#[test]
fn 공중_여부만_바뀌어도_웹뷰에_알린다() {
    let ground = (
        Behavior::Sassy {
            sassy: SassyKind::EyeRoll,
        },
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    let air = (
        Behavior::Sassy {
            sassy: SassyKind::EyeRoll,
        },
        Facing::Right,
        Vertical::Level,
        true,
        None,
        0,
        false,
    );
    assert!(should_notify(Some(ground), air));
}

#[test]
fn 유휴_종류가_바뀌면_웹뷰에_알린다() {
    let a = (
        Behavior::Idle {
            idle: IdleKind::LookAround,
        },
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    let b = (
        Behavior::Idle {
            idle: IdleKind::Shake,
        },
        Facing::Right,
        Vertical::Level,
        false,
        None,
        0,
        false,
    );
    assert!(should_notify(Some(a), b));
}

#[test]
fn 경계는_창_여백까지_화면_안에_들어오게_잡는다() {
    let b = bounds_from_work_area((0, 25), (1440, 875), 1.0, 140.0);
    assert_eq!(b.left, PET_PAD_X, "왼쪽 여백만큼 안으로 들어와야 한다");
    assert_eq!(b.right, 1440.0 - 140.0 - PET_PAD_X);
    assert_eq!(
        b.top,
        25.0 + PET_PAD_TOP,
        "말풍선이 메뉴바 뒤로 숨으면 안 된다"
    );
    assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
}

#[test]
fn 어느_경계에_서도_창_전체가_화면_안이다() {
    // **어느 배율에서도** 성립해야 한다 — 경계는 코어 단위로 넓어지고 창은
    // 화면 단위로 줄어드는데, 둘이 어긋나면 작은 펭귄이 화면 밖에 선다.
    let area = (0.0, 25.0, 1440.0, 875.0);
    for percent in (SIZE_MIN..=SIZE_MAX).step_by(SIZE_STEP as usize) {
        let s = scale_of(percent);
        let b = bounds_of_work_area((0, 25), (1440, 875), 1.0, s).expect("멀쩡한 작업 영역");
        let (ww, wh) = pet_window_size(s);
        for (px, py) in [
            (b.left, b.top),
            (b.right, b.top),
            (b.left, b.floor_y),
            (b.right, b.floor_y),
        ] {
            let (wx, wy) = window_origin(px, py, s);
            assert!(wx >= area.0 - 0.001, "{percent}%: 창이 왼쪽으로 벗어남: {wx}");
            assert!(
                wx + ww <= area.0 + area.2 + 0.001,
                "{percent}%: 오른쪽으로 벗어남"
            );
            assert!(wy >= area.1 - 0.001, "{percent}%: 창이 위로 벗어남: {wy}");
            assert!(wy + wh <= area.1 + area.3 + 0.001, "{percent}%: 아래로 벗어남");
        }
    }
}

/// 배율이 작으면 코어가 보는 세계가 넓어진다 — 작은 펭귄은 더 오른쪽까지 간다.
#[test]
fn 배율이_작으면_세계가_넓어진다() {
    let 크게 = bounds_of_work_area((0, 25), (1440, 875), 1.0, 1.0).unwrap();
    let 작게 = bounds_of_work_area((0, 25), (1440, 875), 1.0, 0.5).unwrap();
    assert!(작게.right > 크게.right, "세계가 안 넓어졌다");
    assert!(작게.floor_y > 크게.floor_y, "바닥이 안 내려갔다");
    // 화면으로 되돌리면 같은 자리다 — 오른쪽 끝의 창 오른쪽 변이 일치한다.
    let 오른쪽 = |b: Bounds, s: f64| window_origin(b.right, b.floor_y, s).0 + pet_window_size(s).0;
    assert!((오른쪽(크게, 1.0) - 오른쪽(작게, 0.5)).abs() < 1e-9);
}

#[test]
fn 레티나_배율에서도_논리_좌표로_환산한다() {
    let b = bounds_from_work_area((0, 50), (2880, 1750), 2.0, 140.0);
    assert_eq!(b.left, PET_PAD_X);
    assert_eq!(b.right, 1440.0 - 140.0 - PET_PAD_X);
    assert_eq!(b.floor_y, 25.0 + 875.0 - 140.0);
}

#[test]
fn 보조_모니터처럼_원점이_음수여도_경계가_밀린다() {
    let b = bounds_from_work_area((-1920, 0), (1920, 1080), 1.0, 140.0);
    assert_eq!(b.left, -1920.0 + PET_PAD_X);
    assert_eq!(b.right, -1920.0 + 1920.0 - 140.0 - PET_PAD_X);
}

#[test]
fn 영역이_펭귄보다_좁아도_경계가_뒤집히지_않는다() {
    let b = bounds_from_work_area((0, 0), (100, 100), 1.0, 140.0);
    assert!(
        b.right >= b.left,
        "right가 left보다 작아지면 clamp가 패닉한다"
    );
    assert!(b.floor_y >= 0.0);
}

// ── 틱 반영 결정 ──

#[test]
fn 경계를_못_읽어_구조된_마리는_자고_있어도_창을_옮긴다() {
    assert!(
        should_move(false, true),
        "안 옮기면 사라진 화면의 좌표에 남아 다시는 안 보인다"
    );
}

#[test]
fn 아무_일도_없으면_창을_옮기지_않는다() {
    assert!(!should_move(false, false));
}

#[test]
fn 움직이는_동작은_구조가_아니어도_창을_옮긴다() {
    assert!(should_move(true, false));
}

#[test]
fn 한_마리라도_움직이면_틱이_빨라진다() {
    assert_eq!(tick_interval(true, false), TICK_MS);
}

#[test]
fn 전부_멈춰_있으면_틱이_느려진다() {
    assert_eq!(tick_interval(false, false), SLEEP_TICK_MS);
}

#[test]
fn 클릭을_통과_중이면_자는_펭귄도_빠르게_돈다() {
    // 통과를 되돌리는 유일한 눈이 이 틱이다. 500ms로 늘어지면 커서를 펭귄
    // 위로 옮기고 그 안에 누른 클릭이 아래 앱으로 샌다.
    assert_eq!(tick_interval(false, true), TICK_MS);
}

// ── 여럿 만들기: 전부 아니면 하나도 ────────────────────────────

#[test]
fn 전부_성공하면_되돌리지_않는다() {
    let mut 만든 = Vec::new();
    let mut 되돌린 = None;
    let r: Result<(), ()> = build_all_or_none(
        3,
        |i| {
            만든.push(i);
            Ok(Some(i))
        },
        |x| 되돌린 = Some(x),
    );
    assert!(r.is_ok());
    assert_eq!(만든, vec![0, 1, 2]);
    assert_eq!(되돌린, None);
}

/// 둘째 화면에서 실패하면 첫째 판 창이 그대로 남는다 — 커맨드는 모드를 "꺼짐"으로
/// 되돌리므로 **화면은 판이고 상태는 꺼짐**이 되어 Esc도 트레이도 그 판을 못 닫는다.
/// "나가는 문이 둘"이 이 경로 하나에서만 무너진다.
#[test]
fn 중간에_실패하면_앞서_만든_것을_되돌린다() {
    let mut 시도 = Vec::new();
    let mut 되돌린 = None;
    let r = build_all_or_none(
        3,
        |i| {
            시도.push(i);
            if i == 1 {
                Err("둘째 화면에서 실패")
            } else {
                Ok(Some(i))
            }
        },
        |x| 되돌린 = Some(x),
    );
    assert_eq!(r, Err("둘째 화면에서 실패"));
    assert_eq!(시도, vec![0, 1], "실패한 뒤로는 더 시도하지 않는다");
    assert_eq!(되돌린, Some(vec![0]), "앞서 만든 것만 되돌려야 한다");
}

/// **이미 있어서 건너뛴 것은 되돌리지 않는다.** 라벨 접두어로 싹 닫으면
/// 펭귄을 껐다 켤 때 멀쩡히 돌던 판까지 사라진다 — 그때 판은 살아 있는 채로
/// `create_pinball_window`가 다시 불린다.
#[test]
fn 이미_있던_것은_되돌리지_않는다() {
    let mut 되돌린 = None;
    let r = build_all_or_none(
        3,
        |i| match i {
            0 => Ok(None), // 이미 있어서 건너뜀
            1 => Ok(Some(1)),
            _ => Err("셋째에서 실패"),
        },
        |x| 되돌린 = Some(x),
    );
    assert_eq!(r, Err("셋째에서 실패"));
    assert_eq!(
        되돌린,
        Some(vec![1]),
        "이번에 만든 1만 닫는다 — 원래 있던 0은 그대로 둔다"
    );
}

#[test]
fn 첫_번째에서_실패하면_되돌릴_것이_없다() {
    let mut 되돌린 = None;
    let r: Result<(), &str> = build_all_or_none(
        2,
        |_| Err("첫 화면부터 실패"),
        |x: Vec<u32>| 되돌린 = Some(x),
    );
    assert_eq!(r, Err("첫 화면부터 실패"));
    assert_eq!(되돌린, Some(vec![]), "만든 게 없으면 빈 목록이다");
}

#[test]
fn 만들_것이_없으면_아무것도_안_한다() {
    let mut 되돌린 = None;
    let r: Result<(), ()> = build_all_or_none(
        0,
        |_| -> Result<Option<u32>, ()> { unreachable!() },
        |x| 되돌린 = Some(x),
    );
    assert!(r.is_ok());
    assert_eq!(되돌린, None);
}
