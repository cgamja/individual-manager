//! 20Hz 틱 스레드 — 코어를 진행시키고 창을 옮기고 웹뷰에 알린다.

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

/// 위치·동작 갱신 주기. 스프라이트 프레임은 CSS가 담당하므로 이 주기는
/// "얼마나 부드럽게 이동하느냐"만 정한다. set_position은 매 호출이 IPC라
/// 60Hz로 때리지 않는다 (KTD2).
const TICK_MS: u64 = 50;

/// 졸고 있을 때의 틱 간격 — 깨어날 시각만 확인하면 되므로 길게 잡는다 (R10).
const SLEEP_TICK_MS: u64 = 500;

/// 모니터 작업 영역을 다시 읽는 주기.
const BOUNDS_REFRESH_MS: u64 = 2_000;

/// 상태와 창이 어긋난 것을 보고 **정리하기까지 기다리는 시간**.
const RECONCILE_GRACE_MS: u64 = 1_000;

/// 어긋남을 처음 본 시각들 중, 유예를 다 쓴 것들.
pub(super) fn due_for_cleanup(mismatch_since: &HashMap<PetId, u64>, now_ms: u64) -> Vec<PetId> {
    mismatch_since
        .iter()
        .filter(|(_, since)| now_ms.saturating_sub(**since) >= RECONCILE_GRACE_MS)
        .map(|(id, _)| *id)
        .collect()
}

/// 위치·동작 틱. 트레이(`set_title`)와 달리 `set_position`은 어느 스레드에서
/// 불러도 안전하다 — tauri-runtime-wry가 메인 스레드가 아니면 이벤트 루프로
/// 넘기고, tao의 macOS 구현이 다시 메인 스레드로 디스패치한다 (KTD5).
/// 그러니 여기서는 `run_on_main_thread`로 감싸지 않는다.
pub fn spawn_pet_tick_thread(app: AppHandle) {
    std::thread::spawn(move || {
        let mut worlds: HashMap<PetId, (World, u64)> = HashMap::new();
        let mut last_look: HashMap<PetId, Look> = HashMap::new();
        let mut mismatch_since: HashMap<PetId, u64> = HashMap::new();
        loop {
            let ids = app.state::<PetState>().pets.lock().unwrap().ids();
            let now = now_ms();

            let mut orphan_windows: HashMap<PetId, WebviewWindow> = HashMap::new();
            for (label, window) in app.webview_windows() {
                if let Some(orphan) = pet_id_from_label(&label) {
                    if !ids.contains(&orphan) {
                        orphan_windows.insert(orphan, window);
                    }
                }
            }

            let mut mismatched: Vec<PetId> = orphan_windows.keys().copied().collect();
            for id in &ids {
                if pet_window(&app, *id).is_none() {
                    mismatched.push(*id);
                }
            }
            mismatch_since.retain(|id, _| mismatched.contains(id));
            for id in &mismatched {
                mismatch_since.entry(*id).or_insert(now);
            }

            for id in due_for_cleanup(&mismatch_since, now) {
                if let Some(window) = orphan_windows.get(&id) {
                    let _ = window.close();
                } else {
                    app.state::<PetState>().pets.lock().unwrap().forget(id);
                }
                mismatch_since.remove(&id);
                worlds.remove(&id);
                last_look.remove(&id);
            }

            if ids.is_empty() {
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            }

            let mut any_moves = false;

            for id in ids {
                let Some(window) = pet_window(&app, id) else {
                    continue;
                };

                let stale = worlds
                    .get(&id)
                    .is_none_or(|(_, at)| now.saturating_sub(*at) >= BOUNDS_REFRESH_MS);
                let mut rescued = false;
                if stale {
                    let read = current_bounds(&window).map(World::single);
                    rescued = read.is_none();
                    if let Some(world) =
                        world_to_cache(read, || primary_bounds(&window).map(World::single))
                    {
                        worlds.insert(id, (world, now));
                    } else {
                        rescued = false;
                    }
                }
                let Some((world, _)) = worlds.get(&id) else {
                    continue;
                };

                let snapshot = {
                    let state = app.state::<PetState>();
                    let mut pets = state.pets.lock().unwrap();
                    let Some(pet) = pets.get_mut(id) else {
                        continue;
                    };
                    pet.step(now, world)
                };

                let moves = snapshot.behavior.moves_window() || rescued;
                any_moves |= snapshot.behavior.moves_window();
                let look = look_of(&snapshot);
                apply(
                    &window,
                    snapshot,
                    moves,
                    should_notify(last_look.get(&id).copied(), look),
                );
                last_look.insert(id, look);
            }

            let interval = if any_moves { TICK_MS } else { SLEEP_TICK_MS };
            std::thread::sleep(Duration::from_millis(interval));
        }
    });
}

/// 스냅샷을 창 위치와 웹뷰 상태에 반영한다. 창 이동과 상태 통지는 조건이
/// 다르다 — 자는 펭귄은 움직이지 않지만 "잔다"는 사실은 알려야 한다.
pub(super) fn apply(window: &WebviewWindow, snapshot: Snapshot, move_window: bool, notify: bool) {
    if move_window {
        let (wx, wy) = window_origin(snapshot.x, snapshot.y);
        let _ = window.set_position(LogicalPosition::new(wx, wy));
    }
    if notify {
        let _ = window.emit_to(
            EventTarget::webview_window(window.label()),
            EVENT_PET_STATE,
            snapshot,
        );
    }
}

/// 커맨드가 상태를 바꾼 뒤 즉시 화면에 반영한다 — 다음 틱(최대 500ms)을
/// 기다리면 클릭·드래그 반응이 굼떠 보인다. 커맨드는 항상 동작을 바꾸므로
/// 이동과 통지를 모두 한다.
pub(super) fn flush(app: &AppHandle, id: PetId) -> Option<Snapshot> {
    let window = pet_window(app, id)?;
    let snapshot = app
        .state::<PetState>()
        .pets
        .lock()
        .unwrap()
        .get(id)?
        .snapshot();
    apply(&window, snapshot, true, true);
    Some(snapshot)
}
