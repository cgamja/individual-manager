use super::*;

/// `settings.json`의 `pet` 객체 한 조각.
fn 저장(json: serde_json::Value) -> serde_json::Value {
    json
}

#[test]
fn 저장이_없으면_배율은_1이다() {
    assert_eq!(scale_from(None), 1.0);
    assert_eq!(size_percent_from(None), SIZE_DEFAULT);
}

#[test]
fn 크기_퍼센트를_배율로_바꾼다() {
    let v = 저장(serde_json::json!({ "size": 60 }));
    assert_eq!(size_percent_from(Some(&v)), 60);
    assert_eq!(scale_from(Some(&v)), 0.6);
}

#[test]
fn 범위를_벗어난_크기는_조인다() {
    // 손으로 고친 저장 파일이 화면을 덮는 펭귄을 만들면 안 된다.
    let 작다 = 저장(serde_json::json!({ "size": 5 }));
    let 크다 = 저장(serde_json::json!({ "size": 5000 }));
    assert_eq!(scale_from(Some(&작다)), f64::from(SIZE_MIN) / 100.0);
    assert_eq!(scale_from(Some(&크다)), f64::from(SIZE_MAX) / 100.0);
}

#[test]
fn 크기가_숫자가_아니면_기본값이다() {
    for 깨진 in [
        serde_json::json!({ "size": "크다" }),
        serde_json::json!({ "size": null }),
        serde_json::json!({ "size": -30 }),
        serde_json::json!({ "pinball": true }),
    ] {
        assert_eq!(scale_from(Some(&깨진)), 1.0, "{깨진} 에서 기본값으로 안 떨어졌다");
    }
}

#[test]
fn 다른_설정이_있어도_크기만_읽는다() {
    let v = 저장(serde_json::json!({ "enabled": true, "count": 3, "size": 80 }));
    assert_eq!(size_percent_from(Some(&v)), 80);
}

#[test]
fn 렌더_크기는_펭귄_한_변에_배율을_곱한_값이다() {
    assert_eq!(pet_render_px(1.0), crate::pet::PET_SIZE);
    assert_eq!(pet_render_px(0.5), crate::pet::PET_SIZE / 2.0);
}

#[test]
fn 배율이_1이면_창_크기가_예전과_같다() {
    assert_eq!(pet_window_size(1.0), (PET_WINDOW_W, PET_WINDOW_H));
}

#[test]
fn 배율이_반이면_창도_절반이다() {
    let (w, h) = pet_window_size(0.5);
    assert_eq!((w, h), (PET_WINDOW_W / 2.0, PET_WINDOW_H / 2.0));
}

#[test]
fn 창_안의_펭귄_사각형이_여백까지_배율을_탄다() {
    let (x, y, w, h) = pet_box_in_window(0.5);
    assert_eq!((x, y), (PET_PAD_X / 2.0, PET_PAD_TOP / 2.0));
    assert_eq!((w, h), (pet_render_px(0.5), pet_render_px(0.5)));
}

#[test]
fn 펭귄_사각형은_언제나_창_안에_있다() {
    // 여백과 몸통이 같은 배율을 타므로 어느 배율에서도 창을 넘지 않는다.
    for percent in (SIZE_MIN..=SIZE_MAX).step_by(SIZE_STEP as usize) {
        let s = scale_of(percent);
        let (x, y, w, h) = pet_box_in_window(s);
        let (ww, wh) = pet_window_size(s);
        assert!(x >= 0.0 && y >= 0.0, "{percent}%에서 사각형이 창 밖으로 나갔다");
        assert!(x + w <= ww + 1e-9, "{percent}%에서 가로가 창을 넘었다");
        assert!(y + h <= wh + 1e-9, "{percent}%에서 세로가 창을 넘었다");
    }
}

#[test]
fn 화면과_코어를_왕복하면_제자리다() {
    for percent in [SIZE_MIN, SIZE_DEFAULT, SIZE_MAX] {
        let s = scale_of(percent);
        let 원본 = 337.5;
        assert!((to_core(to_screen(원본, s), s) - 원본).abs() < 1e-9);
    }
}

#[test]
fn 배율은_언제나_0보다_크다() {
    // `to_core`가 나누기라 0이면 무한대가 된다. 정화가 그걸 막는 유일한 장치다.
    for percent in [0, 1, SIZE_MIN, SIZE_MAX, u32::MAX] {
        assert!(scale_of(percent) > 0.0, "{percent}%에서 배율이 0 이하다");
    }
}

#[test]
fn 배율을_첫_페인트_전에_심는_스크립트가_수를_담는다() {
    // 저장소 왕복을 기다리면 창은 작은데 그림은 배율 1로 한 프레임 그려진다.
    let script = scale_init_script(0.6);
    assert!(script.contains("0.6"), "배율이 안 들어갔다: {script}");
    assert!(script.contains("__PG_SCALE"), "웹뷰가 읽는 이름이 없다: {script}");
    // 웹뷰가 `Number.isFinite`로 거르므로 수여야 한다 — 따옴표가 붙으면 안 된다.
    assert!(!script.contains('"'), "값이 문자열이 됐다: {script}");
}
