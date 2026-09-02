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
