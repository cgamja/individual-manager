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

/// 저장된 값에서 크기 퍼센트를 꺼낸다. 없거나 깨졌거나 **범위를 벗어나면**
/// [`SIZE_DEFAULT`]다 — 저장 파일이 손으로 고쳐져도 화면을 덮는 펭귄이 뜨지 않는다.
///
/// **조이지 않고 기본값으로 떨어뜨린다.** 프론트의 `sanitizeSize`가 같은 규칙이라야
/// `size: 200`에서 펭귄은 150%인데 슬라이더는 100%를 가리키는 일이 없다.
/// `theme_from`·`sanitizeVolume`이 이미 "깨진 값은 기본값" 규칙이다.
///
/// `pinball_from`·`theme_from`처럼 `AppHandle`이 아니라 값을 받는다 (테스트를 위해서다).
pub fn size_percent_from(stored: Option<&serde_json::Value>) -> u32 {
    stored
        .and_then(|value| value.get("size"))
        .and_then(|v| v.as_u64())
        .filter(|n| (u64::from(SIZE_MIN)..=u64::from(SIZE_MAX)).contains(n))
        .map_or(SIZE_DEFAULT, |n| n as u32)
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
///
/// **`store()`가 아니라 `get_store()`를 먼저 본다.** `store()`는 이미 열린 스토어에도
/// 매번 경로를 다시 해석하고 전역 컬렉션에 **쓰기 락**을 잡는다(`StoreBuilder::build_inner`).
/// 이 함수는 20Hz 틱과 드래그(pointermove)마다 불리므로 그 값이면 프론트의
/// `store.get`/`set`과 계속 경합한다. `get_store()`는 읽기 락만 잡는다.
/// 아직 안 열렸을 때만 `store()`로 떨어진다.
pub fn pet_scale(app: &AppHandle) -> f64 {
    let store = app
        .get_store(SETTINGS_FILE)
        .or_else(|| app.store(SETTINGS_FILE).ok());
    scale_from(store.and_then(|store| store.get(PET_KEY)).as_ref())
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
///
/// **아직 프로덕션 호출자가 없다.** 창의 투명 여백이 클릭을 먹는 것을 고치는 작업
/// (`fix/f4-pet-hit-area-01`)이 히트 영역을 이 값으로 잡기로 해서 이름과 시그니처를
/// 먼저 맞춰 뒀다 — 배율이 곱해지는 자리를 한 곳으로 묶기 위해서다. 그 작업이
/// 들어오지 않으면 지워야 한다.
pub fn pet_box_in_window(scale: f64) -> (f64, f64, f64, f64) {
    let side = pet_render_px(scale);
    (PET_PAD_X * scale, PET_PAD_TOP * scale, side, side)
}

/// 첫 페인트 전에 배율을 웹뷰에 심는 스크립트.
///
/// **저장소를 읽어 올 때까지 기다리면 안 된다** — 창은 이미 배율만큼 작은데 그림은
/// 배율 1로 한 프레임 그려져 잘린 펭귄이 번쩍인다. 창을 만들 때마다 보인다.
pub fn scale_init_script(scale: f64) -> String {
    format!("window.__PG_SCALE = {scale};")
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
