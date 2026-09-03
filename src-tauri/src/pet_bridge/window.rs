//! 펭귄 창의 생성·수명·좌표. 창 플래그는 전부 여기서 정한다.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::pet::PetId;
use crate::pet::PET_SIZE;

use super::*;

/// 저장된 마릿수만큼 펭귄을 만든다. 이미 있으면 모자란 만큼만 채운다.
pub fn spawn_saved_pets(app: &AppHandle) -> tauri::Result<()> {
    let wanted = pet_count(app);
    let scale = pet_scale(app);
    let now = now_ms();
    let world = world_or_flat_any(app);
    let bounds = world.first().bounds;
    while app.state::<PetState>().pets.lock().unwrap().len() < wanted {
        let start_x = {
            let state = app.state::<PetState>();
            let pets = state.pets.lock().unwrap();
            pets.ids()
                .last()
                .and_then(|id| pets.get(*id))
                .map_or(bounds.left, |p| next_to(p.snapshot().x, bounds))
        };
        let Some(id) = app
            .state::<PetState>()
            .pets
            .lock()
            .unwrap()
            .add(now, now, &world, start_x)
        else {
            break;
        };
        apply_saved_settings(app, id);
        if let Err(err) = create_pet_window(
            app,
            id,
            window_origin(start_x, bounds.floor_y, scale),
            scale,
        ) {
            app.state::<PetState>().pets.lock().unwrap().forget(id);
            return Err(err);
        }
    }
    if pet_pinball(app) {
        if let Err(err) = create_pinball_window(app) {
            // **모드도 함께 되돌린다.** 로그만 남기면 상태는 "켜짐"인데 판은
            // 0개가 되어, 클릭이 채가 되는 범위와 화면이 어긋난 채로 시작한다.
            // `pet_set_pinball`이 실패했을 때 하는 것과 같은 처리다.
            eprintln!("[penguin] 시작 시 핀볼 판을 못 깔았다 — 모드를 끈다: {err}");
            let ids = {
                let state = app.state::<PetState>();
                let mut pets = state.pets.lock().unwrap();
                let ids = pets.ids();
                for id in &ids {
                    if let Some(pet) = pets.get_mut(*id) {
                        pet.set_pinball(false);
                    }
                }
                ids
            };
            for id in ids {
                flush(app, id);
            }
            let _ = app.emit(EVENT_PET_SETTINGS, serde_json::json!({ "pinball": false }));
        }
    }
    Ok(())
}

/// 펫 창 라벨 접두어. 마리마다 `pet-<id>`가 된다.
pub const PET_LABEL_PREFIX: &str = "pet-";

/// 펭귄 id → 창 라벨.
pub fn pet_label(id: PetId) -> String {
    format!("{PET_LABEL_PREFIX}{id}")
}

/// 창 라벨 → 펭귄 id. 펫 창이 아니면 `None`.
pub fn pet_id_from_label(label: &str) -> Option<PetId> {
    label.strip_prefix(PET_LABEL_PREFIX)?.parse().ok()
}

/// 펭귄 좌우로 비워 두는 여백 — 방망이가 휘둘러질 자리.
pub const PET_PAD_X: f64 = 52.0;

/// 펭귄 위로 비워 두는 여백 — 말풍선이 뜰 자리.
pub const PET_PAD_TOP: f64 = 80.0;

/// 창은 펭귄보다 크다. 창을 펭귄 크기에 딱 맞추면 말풍선과 방망이가 잘린다.
pub const PET_WINDOW_W: f64 = PET_SIZE + PET_PAD_X * 2.0;

pub const PET_WINDOW_H: f64 = PET_SIZE + PET_PAD_TOP;

/// 펭귄의 **코어 좌표** → **화면 논리 좌표**의 창 좌상단. 펭귄이 창 안에서
/// (PAD_X, PAD_TOP)에 놓이므로 창은 그만큼 왼쪽·위로 물러나 있고, 그 여백도
/// 몸통과 함께 배율을 탄다.
pub fn window_origin(pet_x: f64, pet_y: f64, scale: f64) -> (f64, f64) {
    (
        to_screen(pet_x - PET_PAD_X, scale),
        to_screen(pet_y - PET_PAD_TOP, scale),
    )
}

pub fn pet_window(app: &AppHandle, id: PetId) -> Option<WebviewWindow> {
    app.get_webview_window(&pet_label(id))
}

/// 살아 있는 아무 펫 창 하나. 화면 경계처럼 "어느 펭귄이든 상관없는" 조회에 쓴다.
pub(super) fn any_pet_window(app: &AppHandle) -> Option<WebviewWindow> {
    let ids = app.state::<PetState>().pets.lock().unwrap().ids();
    ids.into_iter().find_map(|id| pet_window(app, id))
}

/// 펫 창을 만든다. 이미 있으면 그것을 돌려준다 (중복 생성 방지).
pub fn create_pet_window(
    app: &AppHandle,
    id: PetId,
    at: (f64, f64),
    scale: f64,
) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = pet_window(app, id) {
        return Ok(existing);
    }

    let (w, h) = pet_window_size(scale);
    WebviewWindowBuilder::new(app, pet_label(id), WebviewUrl::App("pet.html".into()))
        .title("Penguin Pet")
        .inner_size(w, h)
        .position(at.0, at.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .accept_first_mouse(true)
        .focused(false)
        .focusable(false)
        .visible(true)
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()
        .inspect(|window| {
            let _ = window.show();
        })
}

/// 펫 창 하나를 닫는다. 없으면 아무것도 하지 않는다.
pub fn close_pet_window(app: &AppHandle, id: PetId) {
    if let Some(window) = pet_window(app, id) {
        let _ = window.close();
    }
}

/// 모든 펫 창을 닫는다 (설정에서 껐을 때).
pub fn close_all_pet_windows(app: &AppHandle) {
    let ids = app.state::<PetState>().pets.lock().unwrap().ids();
    for id in ids {
        close_pet_window(app, id);
    }
}
