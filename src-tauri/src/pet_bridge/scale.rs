//! 크기 배율 — 저장된 퍼센트를 배율로 바꾸고 화면 ↔ 코어 좌표를 환산한다.
//!
//! 코어(`pet/`)는 펭귄이 한 변 [`PET_SIZE`]인 좌표계에서만 산다. **화면에 보이는 크기는
//! 여기서 배율을 곱해 만든다** — 배율이 곱해지는 자리는 이 파일 하나다.

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::pet::PET_SIZE;

use super::*;

/// 크기 슬라이더의 범위·단계·기본값 (퍼센트).
pub const SIZE_MIN: u32 = 50;
pub const SIZE_MAX: u32 = 150;
pub const SIZE_STEP: u32 = 10;
pub const SIZE_DEFAULT: u32 = 100;

const _: () = assert!(SIZE_MIN < SIZE_DEFAULT && SIZE_DEFAULT < SIZE_MAX);
const _: () = assert!((SIZE_MAX - SIZE_MIN) % SIZE_STEP == 0);
const _: () = assert!(SIZE_DEFAULT % SIZE_STEP == 0, "기본값이 슬라이더 눈금에 없다");

/// 저장된 값에서 크기 퍼센트를 꺼낸다. 없거나 깨졌으면 [`SIZE_DEFAULT`]이고,
/// 범위를 벗어나면 조인다 — 저장 파일이 손으로 고쳐져도 화면을 덮는 펭귄이 뜨지 않는다.
///
/// `pinball_from`·`theme_from`처럼 `AppHandle`이 아니라 값을 받는다 (테스트를 위해서다).
pub fn size_percent_from(stored: Option<&serde_json::Value>) -> u32 {
    stored
        .and_then(|value| value.get("size"))
        .and_then(|v| v.as_u64())
        .map_or(SIZE_DEFAULT, |n| {
            n.clamp(u64::from(SIZE_MIN), u64::from(SIZE_MAX)) as u32
        })
}

/// 퍼센트를 배율로. 저장·UI는 퍼센트(`size`), 코드 안은 배율(`scale`)이다 —
/// 한 이름이 두 단위를 가리키면 반드시 어딘가에서 100을 곱하거나 나눈다.
pub fn scale_of(percent: u32) -> f64 {
    f64::from(percent.clamp(SIZE_MIN, SIZE_MAX)) / 100.0
}

/// 저장된 값에서 배율을 꺼낸다.
pub fn scale_from(stored: Option<&serde_json::Value>) -> f64 {
    scale_of(size_percent_from(stored))
}

/// 저장된 배율. 창을 만들거나 옮기기 전에 이걸 읽는다.
pub fn pet_scale(app: &AppHandle) -> f64 {
    scale_from(
        app.store(SETTINGS_FILE)
            .ok()
            .and_then(|store| store.get(PET_KEY))
            .as_ref(),
    )
}

/// **화면에 그려지는 펭귄의 한 변 (논리 px).** 창 크기·좌표 변환·클릭 판정이 전부
/// 여기서 나온다 — 렌더 크기를 내는 함수는 이것 하나뿐이다.
pub fn pet_render_px(scale: f64) -> f64 {
    PET_SIZE * scale
}

/// 펫 창의 바깥 크기. 여백(말풍선·방망이 자리)도 함께 배율을 탄다.
pub fn pet_window_size(scale: f64) -> (f64, f64) {
    (PET_WINDOW_W * scale, PET_WINDOW_H * scale)
}

/// 창 안에서 펭귄이 차지하는 사각형 `(x, y, w, h)` — 창 좌상단 기준 논리 px.
/// 클릭 판정이 이걸 쓴다.
pub fn pet_box_in_window(scale: f64) -> (f64, f64, f64, f64) {
    let side = pet_render_px(scale);
    (PET_PAD_X * scale, PET_PAD_TOP * scale, side, side)
}

/// 코어 좌표 → 화면 논리 px.
pub fn to_screen(v: f64, scale: f64) -> f64 {
    v * scale
}

/// 화면 논리 px → 코어 좌표. 웹뷰가 잰 드래그 델타·던진 속도가 이걸 지난다.
pub fn to_core(v: f64, scale: f64) -> f64 {
    v / scale
}

#[cfg(test)]
#[path = "scale_tests.rs"]
mod tests;
