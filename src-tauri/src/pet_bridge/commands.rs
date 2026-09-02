//! 웹뷰가 부르는 #[tauri::command] 전부.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

use crate::pet::{Behavior, Bounds, Facing, PetId, Pets, Snapshot, Vertical, World, MAX_PETS};

use super::*;

/// 커맨드를 부른 창의 펭귄. 펫 창이 아니면 `None` — 빠따·드래그처럼 **자기
/// 펭귄에게만** 가야 하는 조작은 여기서 걸러진다 (KTD1).
pub fn caller_pet(window: &WebviewWindow) -> Option<PetId> {
    pet_id_from_label(window.label())
}

/// 추가·삭제의 대상. 펫 창이 부르면 자기 자신이고, 팝오버(`main`)가 부르면
/// 마지막으로 우클릭된 펭귄이다 (KTD6).
pub fn target_pet(window: &WebviewWindow, state: &PetState) -> Option<PetId> {
    caller_pet(window).or_else(|| *state.focused.lock().unwrap())
}

/// 빠따 — 왼쪽 클릭 한 번에 펭귄이 한 번 날아간다 (R14).
/// 왼쪽 클릭. `nx`/`ny`는 **맞은 지점**을 펭귄 기준으로 정규화한 값(-0.5~0.5)이다.
#[tauri::command]
pub fn pet_whack(
    nx: f64,
    ny: f64,
    window: WebviewWindow,
    state: State<'_, PetState>,
    app: AppHandle,
) {
    let Some(id) = caller_pet(&window) else { return };
    let world = world_or_flat(&app, id);
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.whack(now_ms(), &world, nx, ny);
    }
    flush(&app, id);
}

/// 오른쪽 클릭 — **펭귄 옆에서** 창을 연다(타이머·설정). 왼쪽 클릭은 빠따가 가져갔다.
/// 메뉴바 밑에서 열면 눌렀는데 화면 반대편에서 뜨는 셈이라 연결이 끊긴다.
#[tauri::command]
pub fn pet_open_popover(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    let Some(snapshot) = state.pets.lock().unwrap().get(id).map(|p| p.snapshot()) else {
        return;
    };
    *state.focused.lock().unwrap() = Some(id);
    let at = popover_anchor(&app, id, snapshot.x, snapshot.y);
    crate::toggle_popover_at(&app, at);
}

/// 드래그 시작 — 자율 이동을 멈춘다 (R6).
#[tauri::command]
pub fn pet_drag_start(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_start(now_ms());
    }
    flush(&app, id);
}

/// 드래그 이동량(논리 px). 창 위치의 소유자는 Rust 하나뿐이라 웹뷰는
/// 이동량만 보내고 직접 `setPosition`을 부르지 않는다 (KTD4).
#[tauri::command]
pub fn pet_drag_by(dx: f64, dy: f64, window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_by(dx, dy);
    }
    flush(&app, id);
}

/// 드래그 놓기 (R6, R12). 웹뷰가 잰 놓는 순간의 속도(논리 px/초)를 그대로 넘긴다 —
/// 세게 던졌으면 포물선을 그리고, 살짝 놓았으면 제자리에서 떨어진다.
#[tauri::command]
pub fn pet_drag_end(vx: f64, vy: f64, window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) {
    let Some(id) = caller_pet(&window) else { return };
    let world = world_or_flat(&app, id);
    if let Some(pet) = state.pets.lock().unwrap().get_mut(id) {
        pet.drag_end(now_ms(), vx, vy, &world);
    }
    flush(&app, id);
}

/// 펭귄을 켜고 끈다 (R8). 끄면 창을 숨기지 않고 닫는다 — 틱 스레드도
/// 창이 없으면 느린 대기로 떨어져 자원을 쓰지 않는다.
/// 저장은 웹뷰가 담당한다 (기존 타이머 설정과 같은 방식).
#[tauri::command]
pub fn pet_set_enabled(enabled: bool, app: AppHandle) -> Result<(), String> {
    if enabled {
        spawn_saved_pets(&app).map_err(|e| e.to_string())
    } else {
        close_all_pet_windows(&app);
        app.state::<PetState>().pets.lock().unwrap().clear();
        Ok(())
    }
}

/// 핀볼 모드를 켜고 끈다 (R8).
#[tauri::command]
pub fn pet_set_pinball(on: bool, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let apply = |on: bool| -> Vec<PetId> {
        let mut pets = state.pets.lock().unwrap();
        let ids = pets.ids();
        for id in &ids {
            if let Some(pet) = pets.get_mut(*id) {
                pet.set_pinball(on);
            }
        }
        ids
    };
    let ids = apply(on);

    if on {
        if let Err(err) = create_field_window(&app) {
            for id in apply(false) {
                flush(&app, id);
            }
            return Err(format!("핀볼 판을 못 깔았어요: {err}"));
        }
    } else {
        close_field_window(&app);
    }

    for id in ids {
        flush(&app, id);
    }
    let _ = app.emit(EVENT_PET_SETTINGS, serde_json::json!({ "pinball": on }));
    Ok(())
}

/// 웹뷰가 처음 뜰 때 현재 상태를 한 번 받아 간다 (첫 틱을 기다리지 않게).
#[tauri::command]
pub fn pet_get_state(window: WebviewWindow, state: State<'_, PetState>) -> Option<Snapshot> {
    let id = caller_pet(&window)?;
    let snapshot = state.pets.lock().unwrap().get(id).map(|p| p.snapshot());
    snapshot
}

/// 팝오버가 버튼 상태를 정하는 데 쓰는 요약 (마릿수·상한·우클릭 대상).
#[tauri::command]
pub fn pet_summary(state: State<'_, PetState>) -> PetSummary {
    let count = state.pets.lock().unwrap().len();
    let focused = *state.focused.lock().unwrap();
    PetSummary {
        count,
        max: MAX_PETS,
        focused,
    }
}

/// 펭귄 한 마리를 **부른 펭귄 옆에** 추가한다.
#[tauri::command]
pub fn pet_add(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<PetId, String> {
    let origin = target_pet(&window, &state);
    let world = origin
        .map(|id| world_or_flat(&app, id))
        .unwrap_or_else(|| world_or_flat_any(&app));
    let bounds = world.first().bounds;
    let start_x = origin
        .and_then(|id| state.pets.lock().unwrap().get(id).map(|p| p.snapshot().x))
        .map(|x| next_to(x, bounds))
        .unwrap_or(bounds.left);

    let now = now_ms();
    let id = state
        .pets
        .lock()
        .unwrap()
        .add(now, now, &world, start_x)
        .ok_or_else(|| format!("펭귄은 {MAX_PETS}마리까지예요"))?;

    apply_saved_settings(&app, id);
    let at = window_origin(start_x, bounds.floor_y);
    if let Err(err) = create_pet_window(&app, id, at) {
        state.pets.lock().unwrap().forget(id);
        return Err(err.to_string());
    }
    save_pet_count(&app, state.pets.lock().unwrap().len());
    Ok(id)
}

/// 설정 창의 "얼음낚시" — 십 분에 한 번짜리 동작을 지금 보게 한다.
#[tauri::command]
pub fn pet_fish(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("낚시할 펭귄을 우클릭해서 열어 주세요")?;
    let started = state
        .pets
        .lock()
        .unwrap()
        .get_mut(id)
        .is_some_and(|pet| pet.start_fishing(now_ms()));
    if !started {
        return Err("이미 낚시하는 중이거나 들고 계세요".into());
    }
    flush(&app, id);
    Ok(())
}

/// 설정 창의 "슬라이딩". 대상 규칙은 [`pet_fish`]와 같다.
#[tauri::command]
pub fn pet_slide(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("미끄러뜨릴 펭귄을 우클릭해서 열어 주세요")?;
    let started = state
        .pets
        .lock()
        .unwrap()
        .get_mut(id)
        .is_some_and(|pet| pet.start_slide(now_ms()));
    if !started {
        return Err("이미 미끄러지는 중이거나, 공중이거나 들고 계세요".into());
    }
    flush(&app, id);
    Ok(())
}

/// 설정 창의 "빽빽거리기". 대상 규칙은 [`pet_fish`]와 같다.
#[tauri::command]
pub fn pet_squawk(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("화나게 할 펭귄을 우클릭해서 열어 주세요")?;
    let started = state
        .pets
        .lock()
        .unwrap()
        .get_mut(id)
        .is_some_and(|pet| pet.start_squawk(now_ms()));
    if !started {
        return Err("이미 빽빽거리는 중이거나 들고 계세요".into());
    }
    flush(&app, id);
    Ok(())
}

/// 설정 창의 "발작". 대상 규칙은 [`pet_fish`]와 같다.
#[tauri::command]
pub fn pet_freakout(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("발작시킬 펭귄을 우클릭해서 열어 주세요")?;
    let started = state
        .pets
        .lock()
        .unwrap()
        .get_mut(id)
        .is_some_and(|pet| pet.start_freakout(now_ms()));
    if !started {
        return Err("이미 발작하는 중이거나 들고 계세요".into());
    }
    flush(&app, id);
    Ok(())
}

/// 우클릭한 펭귄을 삭제한다. **마지막 한 마리는 거부한다** (PRD §5.5).
#[tauri::command]
pub fn pet_remove(window: WebviewWindow, state: State<'_, PetState>, app: AppHandle) -> Result<(), String> {
    let id = target_pet(&window, &state).ok_or("어느 펭귄인지 모르겠어요")?;
    if !state.pets.lock().unwrap().remove(id) {
        return Err("마지막 한 마리는 지울 수 없어요. 전부 없애려면 펭귄을 꺼 주세요".into());
    }
    close_pet_window(&app, id);
    if *state.focused.lock().unwrap() == Some(id) {
        *state.focused.lock().unwrap() = None;
    }
    save_pet_count(&app, state.pets.lock().unwrap().len());
    Ok(())
}
