//! 20Hz 틱 스레드 — 코어를 진행시키고 창을 옮기고 웹뷰에 알린다.

use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, WebviewWindow};

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
pub(super) const BOUNDS_REFRESH_MS: u64 = 2_000;

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
    /// 마지막으로 창에 건 한 변. 배율이 바뀌면 창도 다시 재야 한다.
    side: Option<f64>,
    /// 직전 틱에 **판이** 살아 있었는가. 공과 따로 센다 ([`bowling_over`]).
    board_alive: bool,
    /// 잇달아 창 만들기에 실패한 횟수.
    fails: u32,
}

/// 캐시한 세계를 다시 재야 하는가. 시간뿐 아니라 **배율**도 본다 — 배율만 바뀐
/// 틱에서 안 재면 최대 [`BOUNDS_REFRESH_MS`] 동안 옛 경계로 clamp한다.
pub(super) fn world_is_stale(cached: Option<(u64, f64)>, now: u64, scale: f64) -> bool {
    match cached {
        None => true,
        Some((at, was)) => now.saturating_sub(at) >= BOUNDS_REFRESH_MS || was != scale,
    }
}

/// 이번 틱에 창을 실제로 옮길지. 자는 펭귄은 안 옮긴다.
/// **경계를 못 읽어 주 모니터로 구조된 마리(`rescued`)는 동작과 무관하게 옮긴다** —
/// 안 옮기면 사라진 화면의 좌표에 남아 다시는 안 보인다.
pub(super) fn should_move(moves_window: bool, rescued: bool) -> bool {
    moves_window || rescued
}

/// 다음 틱까지 잘 시간. **구조는 여기에 넣지 않는다** — 한 번 옮기면 제자리를 찾으므로
/// 다음 틱까지 빠르게 돌 이유가 없다.
///
/// **클릭을 통과 중이면 자는 펭귄이어도 빠르게 돈다.** 통과를 되돌리는 유일한
/// 눈이 이 틱이라, 500ms로 늘어지면 커서를 펭귄 위로 옮기고 그 안에 누른 클릭이
/// 아래 앱으로 샌다. 자는 펭귄은 그 구간이 가장 길다.
pub(super) fn tick_interval(any_moves: bool, any_click_through: bool) -> u64 {
    if any_moves || any_click_through {
        TICK_MS
    } else {
        SLEEP_TICK_MS
    }
}

/// 틱이 락을 잡는 유일한 방법 — **중독(poison)돼도 죽지 않는다.**
///
/// 커맨드가 락을 쥔 채 패닉하면 `unwrap()`은 이 스레드를 죽인다. 그러면 클릭
/// 통과를 되돌릴 눈이 사라져 **그때 통과 중이던 창이 영영 안 눌린다** — 이 작업이
/// 막으려던 바로 그 상태다. 값 자체는 한 틱 뒤에 스스로 교정되므로 붙들고 간다.
/// (두 번째 문은 트레이 → 설정에서 펭귄을 껐다 켜는 것이다. 창이 새로 만들어지며
/// 통과가 초기화된다.)
fn 잠금<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// 창 하나의 클릭 통과에 대해 틱이 들고 있는 기억.
#[derive(Clone, Copy, Default)]
pub(super) struct ClickThroughView {
    /// 창에 **성공적으로** 건 값. `None`이면 모르는 상태라 다음 틱에 다시 건다 —
    /// 실패를 성공으로 적으면 되돌리기가 한 번 실패한 뒤 영영 재시도되지 않는다.
    applied: Option<bool>,
    /// 통과를 시작한 순간의 커서 자리 — 드리프트의 기준점.
    anchor: Option<(f64, f64)>,
}

/// 스냅샷의 동작을 [`Pose`]로. 근거가 되는 CSS 변환은 `MOTIONS.md`
/// "클릭의 경계는 창이 아니라 히트 상자다".
///
/// **목록이 뒤집혀 있다** — 상자 안에 머무는 것이 확인된 국면만 적고 나머지는
/// 전부 접는다. 모션 하나는 일곱 자리에 흩어져 있어서(`behavior.rs`) 새 동작을
/// 얹을 때 여기를 빠뜨리기 쉬운데, **빠뜨린 쪽이 안전한 기본값**(= 창이 클릭을
/// 먹는, 고치기 전 동작)이어야 한다.
pub(super) fn pose_of(snapshot: &Snapshot) -> Pose {
    use crate::pet::Behavior::*;
    // 공중에서는 몸이 기울거나(헤엄) 구른다(던져짐·떨어짐).
    if snapshot.air {
        return Pose::OutOfBox;
    }
    match snapshot.behavior {
        Walk | Turn | Sleep | Squawk | Swing => Pose::InBox,
        Idle { .. } | Sassy { .. } | IceFishing { .. } => Pose::InBox,
        _ => Pose::OutOfBox,
    }
}

/// 스냅샷과 커서를 보고 창의 클릭 통과를 맞춘다. 참을 돌려주면 통과 중이다.
///
/// **적용됐는지 읽어서 확인하지 않는다** — 세터가 비동기라 직후에 읽으면 오답이
/// 나온다 (`docs/solutions/best-practices/tauri-ignore-cursor-events-is-async.md`).
/// **`ns_window()`로 안 내려간다** — Tauri API라 틱 스레드에서 안전하다
/// (`docs/solutions/best-practices/appkit-from-tick-thread-kills-the-app.md`).
pub(super) fn apply_click_through(
    window: &WebviewWindow,
    snapshot: &Snapshot,
    requested: bool,
    cursor: Option<(f64, f64)>,
    view: &mut ClickThroughView,
) -> Verdict {
    let verdict = decide_click_through(
        requested,
        pose_of(snapshot),
        (snapshot.x, snapshot.y),
        cursor,
        view.anchor,
    );
    let want = verdict.through();
    view.anchor = if want { view.anchor.or(cursor) } else { None };
    if view.applied != Some(want) {
        view.applied = window.set_ignore_cursor_events(want).ok().map(|()| want);
    }
    verdict
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
        let mut worlds: HashMap<PetId, (World, u64, f64)> = HashMap::new();
        let mut last_look: HashMap<PetId, Look> = HashMap::new();
        let mut ball_view = BallView::default();
        // 비치발리볼의 창 둘(코트·공). 본문은 `pet_bridge/volleyball.rs`에 있다.
        let mut volley_view = VolleyView::default();
        let mut mismatch_since: HashMap<PetId, u64> = HashMap::new();
        let mut click_view: HashMap<PetId, ClickThroughView> = HashMap::new();
        // 화면 배율과 그걸 **읽어 본** 시각. 실패도 시각을 남긴다 — 안 그러면
        // 못 읽는 동안 매 틱 비싼 `current_monitor()`를 두드린다.
        let mut scale: (Option<f64>, u64) = (None, 0);
        loop {
            let ids = 잠금(&app.state::<PetState>().pets).ids();
            let now = now_ms();
            // 배율은 틱마다 한 번만 읽는다 — 창 좌표·창 크기·경계가 전부 이 값을 쓴다.
            let scale = pet_scale(&app);

            // 0) 클릭 통과의 커서 — **락을 하나도 쥐지 않은 채, 통과 중이거나
            //    요청이 있을 때만, 마릿수와 무관하게 한 번 읽는다.**
            //    `cursor_position()`은 메인 스레드를 왕복하는 블로킹 getter라
            //    (`current_monitor()`와 같은 부류, KTD5) 락을 쥔 채 부르면
            //    커맨드와 서로를 붙든다. 아래 `.clone()`이 가드를 그 자리에서 놓는다.
            //
            //    **캐시하지 않는 것은 의도적 예외다.** KTD5가 `current_monitor()`를
            //    캐시하라는 근거는 그 값이 거의 안 변한다는 것인데, 커서는 정반대다 —
            //    낡은 값이 곧 오판이고, 오판의 방향이 "펭귄 위인데 통과 중"이다.
            //    대신 **부르는 횟수 자체를 좁혔다**: 요청이 살아 있는 동안에만 돌고,
            //    위치 판정으로 되돌리는 순간 요청이 지워져 폴이 멎는다([`Verdict`]).
            let requests: HashMap<PetId, bool> =
                잠금(&app.state::<PetState>().click_through).clone();
            let 볼_일이_있다 = requests.values().any(|w| *w)
                || click_view.values().any(|v| v.applied == Some(true));
            let cursor_px = if 볼_일이_있다 {
                app.cursor_position().ok().map(|p| (p.x, p.y))
            } else {
                None
            };

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
                    잠금(&app.state::<PetState>().pets).forget(id);
                }
                mismatch_since.remove(&id);
                worlds.remove(&id);
                last_look.remove(&id);
                click_view.remove(&id);
            }

            // 사라진 마리의 통과 요청은 아무도 안 본다. 남겨 두면 다음에 같은
            // id가 재사용될 때 아무도 요청하지 않은 통과가 켜진 채로 시작한다.
            {
                let state = app.state::<PetState>();
                let mut req = 잠금(&state.click_through);
                req.retain(|id, _| ids.contains(id));
            }
            click_view.retain(|id, _| ids.contains(id));

            if ids.is_empty() {
                // 펭귄을 전부 껐거나 마지막 마리가 사라졌다. 공만 남겨 두면
                // 굴릴 핀이 없는 공이 바탕화면에 영원히 놓여 있다 (R11).
                apply_ball(&app, false, None, &mut ball_view, scale);
                apply_volley(&app, None, &mut volley_view, scale);
                std::thread::sleep(Duration::from_millis(SLEEP_TICK_MS));
                continue;
            }

            // 1) 창을 찾고 경계를 갱신한다. Tauri를 만지는 일은 코어 밖에 남는다.
            let mut ready: HashMap<PetId, (WebviewWindow, bool)> = HashMap::new();
            for id in &ids {
                let Some(window) = pet_window(&app, *id) else {
                    continue;
                };

                let stale = world_is_stale(worlds.get(id).map(|(_, at, s)| (*at, *s)), now, scale);
                let mut rescued = false;
                if stale {
                    let read = current_bounds(&window, scale).map(World::single);
                    rescued = read.is_none();
                    if let Some(world) =
                        world_to_cache(read, || primary_bounds(&window, scale).map(World::single))
                    {
                        worlds.insert(*id, (world, now, scale));
                    } else {
                        rescued = false;
                    }
                }
                if !worlds.contains_key(id) {
                    continue;
                }
                ready.insert(*id, (window, rescued));
            }

            // 커서(물리 px)를 창 좌표(논리 px)로 옮길 배율. 경계와 같은 주기로
            // 캐시한다 — `current_monitor()`가 이벤트 루프를 왕복하는 탓이다.
            // **읽을 일이 있을 때만 읽는다** — 통과가 안 걸린 동안에는 이 값을
            // 아무도 안 본다.
            //
            // **못 읽으면 `None`으로 두고 낡은 값을 안 붙든다.** `None`이면
            // 커서도 `None`이 되어 통과가 통째로 꺼진다 — 안전한 쪽이다 (KTD6).
            // 실패해도 다음 갱신 주기까지는 다시 안 읽는다(비싼 호출이다).
            //
            // **한 마리에서만 읽는다** — 커서는 화면 하나에 있고 세계도 화면
            // 하나다 (PRD §5.2). 배율이 다른 화면이 섞이면 이 변환이 어긋나는데,
            // 그건 이 앱의 범위 밖이다. `ids` 순서로 골라 마리마다 흔들리지
            // 않게 한다 — `HashMap` 순서로 뽑으면 갱신마다 기준이 바뀐다.
            if 볼_일이_있다 && now.saturating_sub(scale.1) >= BOUNDS_REFRESH_MS {
                scale = (
                    ids.iter()
                        .find_map(|id| ready.get(id))
                        .and_then(|(window, _)| current_scale(window)),
                    now,
                );
            }
            let cursor = cursor_px
                .zip(scale.0)
                .map(|((px, py), s)| (px / s, py / s));

            // 2) 코어를 한 번에 진행시킨다. **락을 마리마다 잡지 않는다** — 틱 하나가
            //    전 마리에 대해 원자적이어야 서로를 보는 판정을 여기에 얹을 수 있다.
            let (stepped, ball, board_alive, volley) = {
                let state = app.state::<PetState>();
                let mut pets = 잠금(&state.pets);
                let stepped = pets.step_all(now, |id| {
                    if !ready.contains_key(&id) {
                        return None;
                    }
                    worlds.get(&id).map(|(world, _, _)| world)
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
            let mut any_click_through = false;
            let mut withdraw: Vec<PetId> = Vec::new();
            for (id, snapshot) in stepped {
                let Some((window, rescued)) = ready.get(&id) else {
                    continue;
                };
                let requested = requests.get(&id).copied().unwrap_or(false);
                let verdict = apply_click_through(
                    window,
                    &snapshot,
                    requested,
                    cursor,
                    click_view.entry(id).or_default(),
                );
                any_click_through |= verdict.through();
                // 되돌린 이유가 위치 판정일 때만 요청을 지운다 ([`Verdict`]).
                if requested && verdict.latches() {
                    withdraw.push(id);
                }
                let moves = should_move(snapshot.behavior.moves_window(), *rescued);
                any_moves |= snapshot.behavior.moves_window();
                let look = look_of(&snapshot);
                apply(
                    window,
                    snapshot,
                    moves,
                    should_notify(last_look.get(&id).copied(), look),
                    scale,
                );
                last_look.insert(id, look);
            }
            if !withdraw.is_empty() {
                let state = app.state::<PetState>();
                let mut req = 잠금(&state.click_through);
                for id in withdraw {
                    req.remove(&id);
                }
            }

            any_moves |= ball.is_some_and(|b| b.rolling);
            any_moves |= volley.is_some();
            apply_ball(&app, board_alive, ball, &mut ball_view, scale);
            apply_volley(&app, volley, &mut volley_view, scale);

            std::thread::sleep(Duration::from_millis(tick_interval(
                any_moves,
                any_click_through,
            )));
        }
    });
}

/// 스냅샷을 창 위치와 웹뷰 상태에 반영한다. 창 이동과 상태 통지는 조건이
/// 다르다 — 자는 펭귄은 움직이지 않지만 "잔다"는 사실은 알려야 한다.
pub(super) fn apply(
    window: &WebviewWindow,
    snapshot: Snapshot,
    move_window: bool,
    notify: bool,
    scale: f64,
) {
    if move_window {
        let (wx, wy) = window_origin(snapshot.x, snapshot.y, scale);
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
    scale: f64,
) {
    let Some(ball) = ball else {
        if view.look.take().is_some() {
            close_ball_window(app);
            view.at = None;
            view.side = None;
        }
        if bowling_over(view.board_alive, board_alive) {
            let _ = app.emit(EVENT_BOWLING_OVER, ());
        }
        view.board_alive = board_alive;
        view.fails = 0;
        return;
    };
    view.board_alive = board_alive;

    let at = ball_window_origin(ball.x, ball.y, scale);
    let window = match ball_window(app) {
        Some(window) => window,
        None => match create_ball_window(app, at, scale) {
            Ok(window) => {
                view.fails = 0;
                view.at = Some(at);
                view.side = Some(ball_window_size(scale));
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

    let side = ball_window_size(scale);
    if view.side != Some(side) {
        let _ = window.set_size(LogicalSize::new(side, side));
        view.side = Some(side);
    }
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
    let (wx, wy) = ball_window_origin(ball.x, ball.y, pet_scale(app));
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
    apply(&window, snapshot, true, true, pet_scale(app));
    Some(snapshot)
}
