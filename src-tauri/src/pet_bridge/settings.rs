//! 저장된 설정 읽고 쓰기 — 펭귄 on/off·마릿수·핀볼·테마.

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::pet::{PetId, MAX_PETS};

use super::*;

/// 프론트의 `settings.ts`와 공유하는 저장 위치. 웹뷰가 저장하고 Rust가 읽는다 —
/// 시작 시점에 펭귄을 띄울지는 웹뷰가 뜨기 전에 정해져야 깜빡임이 없다.
pub(super) const SETTINGS_FILE: &str = "settings.json";

pub(super) const PET_KEY: &str = "pet";

/// 저장된 켜짐 여부. 값이 없으면 켜짐이 기본이다 — 사용자가 직접 요청한
/// 기능이라 opt-in으로 숨기지 않는다 (A2).
pub fn pet_enabled(app: &AppHandle) -> bool {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|store| store.get(PET_KEY))
        .and_then(|value| value.get("enabled").and_then(|v| v.as_bool()))
        .unwrap_or(true)
}

/// 저장된 값에서 핀볼 여부를 꺼낸다. **없으면 꺼짐이다** — `enabled`가 켜짐으로
/// 떨어지는 것과 반대다. 새 모드는 사용자가 켜기 전에는 아무것도 바꾸지 않아야 한다.
pub fn pinball_from(stored: Option<&serde_json::Value>) -> bool {
    stored
        .and_then(|value| value.get("pinball"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// 겉모습 테마 — 설정 창과 트레이 아이콘이 함께 따른다 (2026-09-01 사용자 지시).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    /// OS를 따른다 — 창은 테마 강제 없음, 트레이는 템플릿 이미지(자동 적응).
    System,
    Light,
    Dark,
}

/// 저장된 값에서 테마를 꺼낸다. 없거나 깨졌으면 **시스템**이다 — 사용자가
/// 고르기 전에는 아무것도 강제하지 않는다. `pinball_from`처럼 값을 받는
/// 이유도 같다: `AppHandle` 없이 테스트하기 위해서다.
pub fn theme_from(stored: Option<&serde_json::Value>) -> Theme {
    match stored
        .and_then(|value| value.get("theme"))
        .and_then(|v| v.as_str())
    {
        Some("light") => Theme::Light,
        Some("dark") => Theme::Dark,
        _ => Theme::System,
    }
}

/// 저장된 테마.
pub fn pet_theme(app: &AppHandle) -> Theme {
    theme_from(
        app.store(SETTINGS_FILE)
            .ok()
            .and_then(|store| store.get(PET_KEY))
            .as_ref(),
    )
}

/// 저장된 핀볼 모드 여부.
pub fn pet_pinball(app: &AppHandle) -> bool {
    pinball_from(
        app.store(SETTINGS_FILE)
            .ok()
            .and_then(|store| store.get(PET_KEY))
            .as_ref(),
    )
}

/// 새로 만든 펭귄에 저장된 설정을 건다.
pub(super) fn apply_saved_settings(app: &AppHandle, id: PetId) {
    let pinball = pet_pinball(app);
    if let Some(pet) = app.state::<PetState>().pets.lock().unwrap().get_mut(id) {
        pet.set_pinball(pinball);
    }
}

/// 저장된 마릿수. 없으면 한 마리, 범위를 벗어나면 조인다 —
/// 저장 파일이 손으로 고쳐져도 0마리나 100마리로 뜨지 않게.
pub fn pet_count(app: &AppHandle) -> usize {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|store| store.get(PET_KEY))
        .and_then(|value| value.get("count").and_then(|v| v.as_u64()))
        .map_or(1, |n| (n as usize).clamp(1, MAX_PETS))
}

/// 마릿수를 저장한다. **읽고-고쳐-쓰기**여야 한다 — `pet` 키 아래 `enabled`가
/// 함께 살아서, 객체를 통째로 덮어쓰면 켜짐/꺼짐 설정이 날아간다 (KTD3).
pub(super) fn save_pet_count(app: &AppHandle, count: usize) {
    let Ok(store) = app.store(SETTINGS_FILE) else {
        return;
    };
    let mut value = store.get(PET_KEY).unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        obj.insert("count".into(), serde_json::json!(count));
        store.set(PET_KEY, value);
        let _ = store.save();
    }
}
