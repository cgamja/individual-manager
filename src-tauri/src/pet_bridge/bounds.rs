//! 모니터 경계를 읽어 코어가 쓸 World로 바꾼다. 못 읽으면 주 모니터로 떨어진다.

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::pet::PET_SIZE;
use crate::pet::{Bounds, PetId, World};

use super::*;

/// 모니터의 작업 영역(물리 px)을 펭귄이 걸어다닐 논리 좌표 영역으로 바꾼다.
pub fn bounds_from_work_area(
    origin: (i32, i32),
    size: (u32, u32),
    scale: f64,
    pet_size: f64,
) -> Bounds {
    let left = f64::from(origin.0) / scale;
    let top = f64::from(origin.1) / scale;
    let width = f64::from(size.0) / scale;
    let height = f64::from(size.1) / scale;
    let min_x = left + PET_PAD_X;
    let max_x = left + width - pet_size - PET_PAD_X;
    let min_y = top + PET_PAD_TOP;
    let max_y = top + height - pet_size;
    Bounds {
        left: min_x,
        right: max_x.max(min_x),
        top: min_y.min(max_y),
        floor_y: max_y.max(min_y),
    }
}

/// 지금 펭귄이 놓인 모니터의 이동 영역. **창이 어떤 화면에도 안 걸치면 `None`이다** —
/// 모니터를 뽑은 순간이 그 경우이고, 부르는 쪽이 주 모니터로 떨어진다
/// ([`current_world_or_primary`]).
pub(super) fn current_bounds(window: &WebviewWindow, pet_scale: f64) -> Option<Bounds> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .and_then(|m| monitor_bounds(&m, pet_scale))
}

/// **주 모니터**의 경계. 창이 어떤 모니터에도 안 걸칠 때 돌아갈 곳이다.
pub(super) fn primary_bounds(window: &WebviewWindow, pet_scale: f64) -> Option<Bounds> {
    window
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| monitor_bounds(&m, pet_scale))
}

/// 이 창이 선 화면의 배율. 커서는 **물리 px**, 창 좌표는 **논리 px**이라
/// 둘을 견주려면 필요하다. **못 읽으면 `None`이고 부르는 쪽은 클릭 통과를
/// 아예 안 한다** — 어림한 배율로 판정하면 통과가 안 풀린다.
pub(super) fn current_scale(window: &WebviewWindow) -> Option<f64> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .map(|m| m.scale_factor())
}

/// 모니터 하나에서 펭귄이 다닐 수 있는 범위를 낸다.
pub(super) fn monitor_bounds(monitor: &tauri::Monitor, pet_scale: f64) -> Option<Bounds> {
    let area = monitor.work_area();
    bounds_of_work_area(
        (area.position.x, area.position.y),
        (area.size.width, area.size.height),
        monitor.scale_factor(),
        pet_scale,
    )
}

/// 작업 영역 하나를 경계로. **크기가 0이면 `None`이다.**
///
/// **배율이 둘이다.** `dpi_scale`은 모니터의 물리→논리 배율이고, `pet_scale`은 사용자가
/// 고른 크기다. 코어는 펭귄이 `PET_SIZE`인 좌표계에 살므로 화면을 **둘의 곱**으로 나눈다 —
/// 그러면 배율이 작을수록 코어가 보는 세계가 넓어진다.
pub(super) fn bounds_of_work_area(
    pos: (i32, i32),
    size: (u32, u32),
    dpi_scale: f64,
    pet_scale: f64,
) -> Option<Bounds> {
    if size.0 == 0 || size.1 == 0 {
        return None;
    }
    Some(bounds_from_work_area(
        pos,
        size,
        dpi_scale * pet_scale,
        PET_SIZE,
    ))
}

/// 캐시에 넣을 세계를 고른다 — 못 읽었으면 **주 모니터로 떨어진다.**
pub(super) fn world_to_cache(
    current: Option<World>,
    primary: impl FnOnce() -> Option<World>,
) -> Option<World> {
    current.or_else(primary)
}

/// 창이 선 화면의 세계. 못 읽으면 주 모니터로 떨어진다 ([`world_to_cache`]).
pub(super) fn current_world_or_primary(window: &WebviewWindow, pet_scale: f64) -> Option<World> {
    world_to_cache(current_bounds(window, pet_scale).map(World::single), || {
        primary_bounds(window, pet_scale).map(World::single)
    })
}

/// 모니터를 못 읽었을 때 쓰는 납작한 경계. 보수적으로 동작한다 —
/// 폭이 0이라 펭귄이 제자리에 서고, 던지기 상한은 코어의 기본 폭으로 떨어진다.
pub(super) const FLAT_BOUNDS: Bounds = Bounds {
    left: 0.0,
    right: 0.0,
    top: 0.0,
    floor_y: 0.0,
};

/// 현재 세계. 모니터를 못 읽으면 납작한 경계 하나짜리를 쓴다.
pub(super) fn world_or_flat(app: &AppHandle, id: PetId) -> World {
    let scale = pet_scale(app);
    pet_window(app, id)
        .and_then(|w| current_world_or_primary(&w, scale))
        .unwrap_or_else(|| World::single(FLAT_BOUNDS))
}

/// 아무 펭귄이나 기준으로 본 세계.
pub(super) fn world_or_flat_any(app: &AppHandle) -> World {
    let scale = pet_scale(app);
    any_pet_window(app)
        .and_then(|w| current_world_or_primary(&w, scale))
        .or_else(|| {
            app.get_webview_window("main")
                .and_then(|w| current_world_or_primary(&w, scale))
        })
        .unwrap_or_else(|| World::single(FLAT_BOUNDS))
}
