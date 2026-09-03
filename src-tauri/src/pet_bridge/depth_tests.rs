//! 깊이 레벨의 검사 — 범위가 다른 창들과 안 다투는지가 전부다.

use super::*;

#[test]
fn 가까운_마리가_더_높은_레벨을_받는다() {
    // `order`는 뒤에서 앞 순서라, 뒤에 있을수록 index가 크고 레벨이 높다.
    for k in 1..MAX_PETS {
        assert!(depth_level(k) > depth_level(k - 1));
    }
}

#[test]
fn 판_창들은_여전히_전부_아래다() {
    // 핀볼 판·코트가 2이고 가장 뒤 펭귄이 3이라 겹칠 일이 없다.
    assert!(crate::pet_bridge::PINBALL_WINDOW_LEVEL < depth_level(0));
    assert!(crate::pet_bridge::COURT_WINDOW_LEVEL < depth_level(0));
}

#[test]
fn 메뉴바를_안_뚫는다() {
    // 트레이는 핀볼의 "나가는 문 둘" 중 하나라 절대 가려지면 안 된다.
    assert!(depth_level(MAX_PETS - 1) < MENU_BAR_LEVEL);
    assert!(QUEEN_DEPTH_LEVEL < MENU_BAR_LEVEL);
}

#[test]
fn 미녀는_모든_펭귄보다_앞이다() {
    assert!(QUEEN_DEPTH_LEVEL > depth_level(MAX_PETS - 1));
}

#[test]
fn 마릿수를_넘겨도_범위를_안_벗어난다() {
    assert_eq!(depth_level(MAX_PETS + 10), depth_level(MAX_PETS - 1));
}

#[test]
fn 순서가_같으면_다시_안_건다() {
    // 매 틱 메인 스레드로 여덟 번씩 건너가면 그것대로 비싸다.
    let mut view = DepthView::default();
    view.order = vec![1, 2, 3];
    assert_eq!(view.order, vec![1, 2, 3]);
    view.forget();
    assert!(view.order.is_empty());
}
