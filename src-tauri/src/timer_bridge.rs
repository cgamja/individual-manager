//! 타이머 브릿지 — 코어(pomodoro)를 앱에 연결한다: commands, 1Hz 틱, 트레이 타이틀, 이벤트.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::pomodoro::{Config, Phase, Pomodoro, Snapshot};

pub const EVENT_TICK: &str = "pomodoro://tick";
pub const EVENT_FINISHED: &str = "pomodoro://finished";
pub const TRAY_ID: &str = "main-tray";

pub struct TimerState(pub Mutex<Pomodoro>);

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("시스템 시계가 UNIX epoch 이전입니다")
        .as_millis() as u64
}

/// 남은 ms → "MM:SS". 올림 처리로 시작 직후에도 설정 분 그대로 보이게 한다.
/// 60분 이상은 분 단위를 그대로 늘린다 (예: "90:00").
pub fn format_mmss(ms: u64) -> String {
    let total_secs = ms.div_ceil(1000);
    format!("{:02}:{:02}", total_secs / 60, total_secs % 60)
}

/// 트레이 타이틀: Running/Paused는 남은 시간(Paused는 고정 표시), 유휴(Idle/Finished)는 비움 (R7).
pub fn tray_title(snapshot: &Snapshot) -> Option<String> {
    match snapshot {
        Snapshot::Running { remaining_ms, .. } | Snapshot::Paused { remaining_ms, .. } => {
            Some(format_mmss(*remaining_ms))
        }
        Snapshot::Idle | Snapshot::Finished { .. } => None,
    }
}

fn refresh_tray(app: &AppHandle) {
    let snapshot = current_snapshot(app);
    let title = tray_title(&snapshot);
    let app2 = app.clone();
    // 트레이 조작은 메인 스레드에서 — 틱 스레드에서 직접 호출하지 않는다
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app2.tray_by_id(TRAY_ID) {
            let _ = tray.set_title(title.as_deref());
        }
    });
}

fn current_snapshot(app: &AppHandle) -> Snapshot {
    let state = app.state::<TimerState>();
    let snapshot = state.0.lock().unwrap().snapshot(now_ms());
    snapshot
}

/// 커맨드 처리 후 공통 마무리: 트레이 갱신 + 최신 스냅샷 반환.
fn after_command(app: &AppHandle) -> Snapshot {
    refresh_tray(app);
    current_snapshot(app)
}

#[tauri::command]
pub fn timer_start(phase: Phase, state: State<'_, TimerState>, app: AppHandle) -> Snapshot {
    state.0.lock().unwrap().start(phase, now_ms());
    after_command(&app)
}

#[tauri::command]
pub fn timer_pause(state: State<'_, TimerState>, app: AppHandle) -> Snapshot {
    // 틱(1Hz) 사이에 만료된 세션을 먼저 정산한다 — 정산 없이 pause하면
    // "Paused 00:00"으로 얼어붙고 finished 이벤트/알림이 영영 나가지 않는다.
    let now = now_ms();
    let finished = {
        let mut pomodoro = state.0.lock().unwrap();
        let finished = pomodoro.poll(now);
        if finished.is_none() {
            pomodoro.pause(now);
        }
        finished
    };
    settle_finished(&app, finished);
    after_command(&app)
}

#[tauri::command]
pub fn timer_resume(state: State<'_, TimerState>, app: AppHandle) -> Snapshot {
    // pause와 동일하게 만료를 먼저 정산한다 (코어는 Finished에서 resume을 무시한다).
    let now = now_ms();
    let finished = {
        let mut pomodoro = state.0.lock().unwrap();
        let finished = pomodoro.poll(now);
        if finished.is_none() {
            pomodoro.resume(now);
        }
        finished
    };
    settle_finished(&app, finished);
    after_command(&app)
}

#[tauri::command]
pub fn timer_reset(state: State<'_, TimerState>, app: AppHandle) -> Snapshot {
    state.0.lock().unwrap().reset();
    after_command(&app)
}

#[tauri::command]
pub fn timer_get_state(state: State<'_, TimerState>) -> Snapshot {
    let snapshot = state.0.lock().unwrap().snapshot(now_ms());
    snapshot
}

#[tauri::command]
pub fn timer_get_config(state: State<'_, TimerState>) -> Config {
    let config = state.0.lock().unwrap().config();
    config
}

/// 다음 start부터 적용된다 — 진행 중 세션은 바꾸지 않는다 (코어 계약).
#[tauri::command]
pub fn timer_set_config(
    focus_minutes: u32,
    break_minutes: u32,
    state: State<'_, TimerState>,
) -> Result<Config, String> {
    let config = Config::new(focus_minutes, break_minutes)?;
    state.0.lock().unwrap().set_config(config);
    Ok(config)
}

/// 1초마다: 종료 감지(poll) → finished 이벤트, tick 이벤트, 트레이 타이틀 갱신.
pub fn spawn_tick_thread(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        tick(&app);
    });
}

fn tick(app: &AppHandle) {
    let now = now_ms();
    let finished = {
        let state = app.state::<TimerState>();
        let mut pomodoro = state.0.lock().unwrap();
        pomodoro.poll(now)
    };
    settle_finished(app, finished);
    let _ = app.emit(EVENT_TICK, current_snapshot(app));
    refresh_tray(app);
}

/// 만료 정산 공통 경로 — poll이 방금 Finished로 전이시켰다면 finished 이벤트 + 알림.
/// 틱 스레드와 pause/resume 커맨드가 공유한다.
fn settle_finished(app: &AppHandle, finished: Option<Phase>) {
    if let Some(phase) = finished {
        let _ = app.emit(EVENT_FINISHED, phase);
        notify_finished(app, phase);
    }
}

/// 세션 종료 알림 — 웹뷰가 숨겨져 있어도 발송되도록 Rust에서 보낸다 (R8).
/// dev 모드에서는 번들 ID 문제로 도착하지 않는 것이 정상이다 (KTD6).
fn notify_finished(app: &AppHandle, phase: Phase) {
    use tauri_plugin_notification::NotificationExt;
    let (title, body) = match phase {
        Phase::Focus => ("집중 세션 종료", "수고했어요! 휴식할 시간이에요 🐧"),
        Phase::Break => ("휴식 세션 종료", "다시 집중해 볼까요? 🐧"),
    };
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 남은_ms를_mmss로_포맷한다() {
        assert_eq!(format_mmss(25 * 60_000), "25:00");
        assert_eq!(format_mmss(61_000), "01:01");
        assert_eq!(format_mmss(9_000), "00:09");
        assert_eq!(format_mmss(0), "00:00");
    }

    #[test]
    fn 초는_올림_처리되어_시작_직후에도_설정_분이_보인다() {
        assert_eq!(format_mmss(1_499_999), "25:00");
        assert_eq!(format_mmss(59_999), "01:00");
    }

    #[test]
    fn 육십분_이상은_분_단위를_그대로_표시한다() {
        assert_eq!(format_mmss(90 * 60_000), "90:00");
    }

    #[test]
    fn 트레이_타이틀은_running과_paused에서만_표시된다() {
        assert_eq!(
            tray_title(&Snapshot::Running {
                phase: Phase::Focus,
                remaining_ms: 25 * 60_000
            }),
            Some("25:00".to_string())
        );
        // Paused는 일시정지 시점의 남은 시간을 고정 표시한다 (R7)
        assert_eq!(
            tray_title(&Snapshot::Paused {
                phase: Phase::Break,
                remaining_ms: 90_000
            }),
            Some("01:30".to_string())
        );
        assert_eq!(tray_title(&Snapshot::Idle), None);
        assert_eq!(tray_title(&Snapshot::Finished { phase: Phase::Focus }), None);
    }
}
