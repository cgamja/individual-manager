//! 설정 창을 펭귄 옆에 띄울 위치 계산.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

use crate::pet::PET_SIZE;
use crate::pet::{Behavior, Bounds, Facing, PetId, Pets, Snapshot, Vertical, World, MAX_PETS};

use super::*;

/// 펭귄 위치에서 팝오버를 놓을 자리를 구한다. 모니터를 못 읽으면 `None`을
/// 돌려 트레이 밑(기존 동작)으로 떨어진다.
pub fn popover_anchor(app: &AppHandle, id: PetId, pet_x: f64, pet_y: f64) -> Option<(f64, f64)> {
    let popover = app.get_webview_window("main")?;
    let monitor = pet_window(app, id)?.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let popover_scale = popover.scale_factor().unwrap_or(scale);
    let size = popover.inner_size().ok()?;
    Some(popover_position_near(
        (pet_x, pet_y),
        PET_SIZE,
        (
            f64::from(size.width) / popover_scale,
            f64::from(size.height) / popover_scale,
        ),
        (
            f64::from(area.position.x) / scale,
            f64::from(area.position.y) / scale,
            f64::from(area.size.width) / scale,
            f64::from(area.size.height) / scale,
        ),
    ))
}

/// 펭귄과 팝오버 사이 여백 (논리 px).
pub const POPOVER_GAP: f64 = 8.0;

/// 펭귄 옆에 팝오버를 놓을 좌표. 오른쪽을 우선하되 넘치면 왼쪽으로 접고,
/// 그래도 안 되면 영역 안으로 자른다. 세로는 펭귄 높이에 맞추되 화면을 넘지 않는다.
pub fn popover_position_near(
    pet: (f64, f64),
    pet_size: f64,
    popover: (f64, f64),
    area: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (pet_x, pet_y) = pet;
    let (pop_w, pop_h) = popover;
    let (area_x, area_y, area_w, area_h) = area;
    let (right_edge, bottom_edge) = (area_x + area_w, area_y + area_h);

    let to_right = pet_x + pet_size + POPOVER_GAP;
    let x = if to_right + pop_w <= right_edge {
        to_right
    } else {
        pet_x - pop_w - POPOVER_GAP
    };
    let x = x.clamp(area_x, (right_edge - pop_w).max(area_x));
    let y = pet_y.clamp(area_y, (bottom_edge - pop_h).max(area_y));
    (x, y)
}

/// 부른 펭귄 옆자리. 오른쪽을 우선하되 넘치면 왼쪽으로 접는다.
pub fn next_to(x: f64, bounds: Bounds) -> f64 {
    let gap = PET_SIZE * 0.75;
    let right = x + gap;
    let candidate = if right <= bounds.right { right } else { x - gap };
    candidate.clamp(bounds.left, bounds.right.max(bounds.left))
}
