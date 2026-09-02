//! 20Hz 틱 스레드 — 코어를 진행시키고 창을 옮기고 웹뷰에 알린다.

use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, Manager, WebviewWindow};

use crate::pet::{BallSnapshot, PetId, Snapshot, VolleySnapshot, World};

use super::*;

/// 위치·동작 갱신 주기. 스프라이트 프레임은 CSS가 담당하므로 이 주기는
/// "얼마나 부드럽게 이동하느냐"만 정한다. set_position은 매 호출이 IPC라
/// 60Hz로 때리지 않는다 (KTD2).
pub(super) const TICK_MS: u64 = 50;

/// 졸고 있을 때의 틱 간격 — 깨어날 시각만 확인하면 되므로 길게 잡는다 (R10).
pub(super) const SLEEP_TICK_MS: u64 = 500;

/// 쉬는 틱이 더 짧으면 자는 펭귄이 깨어 있는 펭귄보다 더 많은 일을 시킨다.
const _: () = assert!(SLEEP_TICK_MS > TICK_MS);

/// 모니터 작업 영역을 다시 읽는 주기.
const BOUNDS_REFRESH_MS: u64 = 2_000;

/// 상태와 창이 어긋난 것을 보고 **정리하기까지 기다리는 시간**.
const RECONCILE_GRACE_MS: u64 = 1_000;

/// 공 창 만들기를 이만큼 잇달아 실패하면 판을 접는다. **재시도에 끝이 없으면**
/// 20Hz로 영원히 두드리면서 사용자에게는 아무 신호도 안 가고, 버튼은 비활성인
/// 채로 남는다. 핀볼이 판을 못 깔았을 때 모드가 스스로 되돌아가는 것과 같은 규칙이다.
pub(super) const BALL_WINDOW_MAX_FAILS: u32 = 5;

/// 틱 스레드가 공에 대해 들고 있는 유일한 기억.
#[derive(Default)]
pub(super) struct BallView {
    /// 웹뷰에 마지막으로 알린 겉모습. `Some`이면 창이 떠 있다는 뜻이다.
    look: Option<BallLook>,
    /// 마지막으로 창에 건 위치. 같으면 `set_position`을 부르지 않는다 —
    /// 서 있는 공을 20Hz로 옮기면 IPC만 낭비한다.
    at: Option<(f64, f64)>,
    /// 직전 틱에 **판이** 살아 있었는가. 공과 따로 센다 ([`bowling_over`]).
    board_alive: bool,
    /// 잇달아 창 만들기에 실패한 횟수.
    fails: u32,
}

/// 이번 틱에 창을 실제로 옮길지. 자는 펭귄은 안 옮긴다.
/// **경계를 못 읽어 주 모니터로 구조된 마리(`rescued`)는 동작과 무관하게 옮긴다** —
/// 안 옮기면 사라진 화면의 좌표에 남아 다시는 안 보인다.
pub(super) fn should_move(moves_window: bool, rescued: bool) -> bool {
    moves_window || rescued
}

/// 다음 틱까지 잘 시간. **구조는 여기에 넣지 않는다** — 한 번 옮기면 제자리를 찾으므로
/// 다음 틱까지 빠르게 돌 이유가 없다.
pub(super) fn tick_interval(any_moves: bool) -> u64 {
    if any_moves {
        TICK_MS
    } else {
        SLEEP_TICK_MS
    }
}

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
        let mut ball_view = BallView::default();
        // 비치발리볼의 창 둘(코트·공). 본문은 `pet_bridge/volleyball.rs`에 있다.
        let mut volley_view = VolleyView::default();
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
                // 펭귄을 전부 껐거나 마지막 마리가 사라졌다. 공만 남겨 두면
                // 굴릴 핀이 없는 공이 바탕화면에 영원히 놓여 있다 (R11).
                apply_ball(&app, false, None, &mut ball_view);
                apply_volley(&app, None, &mut volley_view);
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            }

            // 1) 창을 찾고 경계를 갱신한다. Tauri를 만지는 일은 코어 밖에 남는다.
            let mut ready: HashMap<PetId, (WebviewWindow, bool)> = HashMap::new();
            for id in &ids {
                let Some(window) = pet_window(&app, *id) else {
                    continue;
                };

                let stale = worlds
                    .get(id)
                    .is_none_or(|(_, at)| now.saturating_sub(*at) >= BOUNDS_REFRESH_MS);
                let mut rescued = false;
                if stale {
                    let read = current_bounds(&window).map(World::single);
                    rescued = read.is_none();
                    if let Some(world) =
                        world_to_cache(read, || primary_bounds(&window).map(World::single))
                    {
                        worlds.insert(*id, (world, now));
                    } else {
                        rescued = false;
                    }
                }
                if !worlds.contains_key(id) {
                    continue;
                }
                ready.insert(*id, (window, rescued));
            }

            // 2) 코어를 한 번에 진행시킨다. **락을 마리마다 잡지 않는다** — 틱 하나가
            //    전 마리에 대해 원자적이어야 서로를 보는 판정을 여기에 얹을 수 있다.
            let (stepped, ball, board_alive, volley) = {
                let state = app.state::<PetState>();
                let mut pets = state.pets.lock().unwrap();
                let stepped = pets.step_all(now, |id| {
                    if !ready.contains_key(&id) {
                        return None;
                    }
                    worlds.get(&id).map(|(world, _)| world)
                });
                // 공은 **같은 락 안에서** 읽는다. 밖에서 다시 잡으면 그 사이에
                // 커맨드가 판을 끝내 공과 펭귄이 다른 틱을 보게 된다.
                let ball = pets.bowling().and_then(|b| b.ball());
                let board_alive = pets.bowling().is_some();
                let volley: Option<VolleySnapshot> = pets.volleyball().map(|v| v.snapshot());
                (stepped, ball, board_alive, volley)
            };

            // 3) 창 위치와 웹뷰에 반영한다. **락 밖에서** 한다 — 창 IPC는 이벤트 루프를
            //    왕복하므로 락을 쥔 채 하면 커맨드가 그만큼 기다린다.
            //    대가: 반영이 밀리는 동안 커맨드의 `flush`가 쓴 새 스냅샷을 여기 낡은
            //    스냅샷이 덮을 수 있다. 노출은 앞선 마리들의 IPC 길이만큼이고 다음 틱에
            //    자가 교정된다.
            //
            //    **회수 조건은 확인했고 발동하지 않았다** (2026-09-02). 마리 간 판정
            //    (핀볼 부딪히기, `Pets::collide_pinball`)이 들어왔지만 **구조상 붙을
            //    자리가 없다**: 상한 8마리라 쌍이 28개뿐이고, 쌍마다 드는 것은 부동소수
            //    산술 수십 번이다 — IPC도 할당도 시스템 호출도 없다. 8마리 1,000틱을
            //    재 보면 틱당 0.5µs → 1.1µs로 50ms 틱의 0.002%다(release, 1회 측정).
            //    판정에 IPC나 마릿수를 넘는 순회가 붙으면 그때 다시 본다.
            let mut any_moves = false;
            for (id, snapshot) in stepped {
                let Some((window, rescued)) = ready.get(&id) else {
                    continue;
                };
                let moves = should_move(snapshot.behavior.moves_window(), *rescued);
                any_moves |= snapshot.behavior.moves_window();
                let look = look_of(&snapshot);
                apply(
                    window,
                    snapshot,
                    moves,
                    should_notify(last_look.get(&id).copied(), look),
                );
                last_look.insert(id, look);
            }
            any_moves |= ball.is_some_and(|b| b.rolling);
            any_moves |= volley.is_some();
            apply_ball(&app, board_alive, ball, &mut ball_view);
            apply_volley(&app, volley, &mut volley_view);

            std::thread::sleep(Duration::from_millis(tick_interval(any_moves)));
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

/// 공을 창에 반영한다. 판이 끝나면(`None`) 창을 닫는다 — **`app.hide()`가
/// 아니라 `window.close()`다.**
///
/// 창을 만드는 것도 여기다. 공은 전부 서기 전에는 없으므로(R4) 창의 생성
/// 시점이 곧 "다 섰다"는 신호가 된다.
pub(super) fn apply_ball(
    app: &AppHandle,
    board_alive: bool,
    ball: Option<BallSnapshot>,
    view: &mut BallView,
) {
    let Some(ball) = ball else {
        if view.look.take().is_some() {
            close_ball_window(app);
            view.at = None;
        }
        if bowling_over(view.board_alive, board_alive) {
            let _ = app.emit(EVENT_BOWLING_OVER, ());
        }
        view.board_alive = board_alive;
        view.fails = 0;
        return;
    };
    view.board_alive = board_alive;

    let at = ball_window_origin(ball.x, ball.y);
    let window = match ball_window(app) {
        Some(window) => window,
        None => match create_ball_window(app, at) {
            Ok(window) => {
                view.fails = 0;
                view.at = Some(at);
                window
            }
            Err(err) => {
                view.fails += 1;
                eprintln!(
                    "[penguin] 공 창을 못 만들었다 ({}/{BALL_WINDOW_MAX_FAILS}): {err}",
                    view.fails
                );
                if view.fails >= BALL_WINDOW_MAX_FAILS {
                    // 포기한다 — 펭귄을 핀 자세에서 풀어 주고 버튼을 되살린다.
                    // 조용히 계속 두드리면 사용자는 굳은 펭귄만 보게 된다.
                    app.state::<PetState>()
                        .pets
                        .lock()
                        .unwrap()
                        .end_bowling(now_ms());
                    view.fails = 0;
                    view.board_alive = false;
                    let _ = app.emit(EVENT_BOWLING_OVER, ());
                }
                return;
            }
        },
    };

    if view.at != Some(at) {
        let _ = window.set_position(LogicalPosition::new(at.0, at.1));
        view.at = Some(at);
    }
    let look = ball_look_of(&ball);
    if view.look != Some(look) {
        let _ = window.emit_to(EventTarget::webview_window(BALL_LABEL), EVENT_BALL_STATE, ball);
        view.look = Some(look);
    }
}

/// 공을 끄는 동안 다음 틱(최대 50ms)을 기다리면 손을 따라오지 못한다.
/// 펭귄의 [`flush`]와 같은 이유다.
pub(super) fn flush_ball(app: &AppHandle) {
    let ball = app
        .state::<PetState>()
        .pets
        .lock()
        .unwrap()
        .bowling()
        .and_then(|b| b.ball());
    let (Some(ball), Some(window)) = (ball, ball_window(app)) else {
        return;
    };
    let (wx, wy) = ball_window_origin(ball.x, ball.y);
    let _ = window.set_position(LogicalPosition::new(wx, wy));
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
