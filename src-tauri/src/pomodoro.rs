//! 뽀모도로 타이머 코어 — Tauri 무의존 순수 상태머신.
//! 진실 원천은 타임스탬프(end_time): 남은 시간은 항상 `end_time - now`로 계산한다 (KTD2).
//! `now`는 epoch ms로 주입받아 테스트에서 시간을 제어한다.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Config {
    pub focus_minutes: u32,
    pub break_minutes: u32,
}

impl Config {
    /// 0분 설정은 거부한다 (최소 1분).
    pub fn new(focus_minutes: u32, break_minutes: u32) -> Result<Self, String> {
        if focus_minutes == 0 || break_minutes == 0 {
            return Err("집중/휴식 시간은 최소 1분이어야 합니다".to_string());
        }
        Ok(Config {
            focus_minutes,
            break_minutes,
        })
    }

    pub fn minutes_for(&self, phase: Phase) -> u32 {
        match phase {
            Phase::Focus => self.focus_minutes,
            Phase::Break => self.break_minutes,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            focus_minutes: 25,
            break_minutes: 5,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Idle,
    Running { phase: Phase, end_time_ms: u64 },
    Paused { phase: Phase, remaining_ms: u64 },
    Finished { phase: Phase },
}

/// 스냅샷 — 프론트 렌더링/트레이 표시용 직렬화 뷰.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Snapshot {
    Idle,
    Running { phase: Phase, remaining_ms: u64 },
    Paused { phase: Phase, remaining_ms: u64 },
    Finished { phase: Phase },
}

pub struct Pomodoro {
    config: Config,
    state: State,
}

impl Pomodoro {
    pub fn new(config: Config) -> Self {
        Pomodoro {
            config,
            state: State::Idle,
        }
    }

    /// Idle/Finished에서만 새 세션을 시작한다. Running/Paused에서는 무시된다.
    pub fn start(&mut self, phase: Phase, now_ms: u64) {
        match self.state {
            State::Idle | State::Finished { .. } => {
                let duration_ms = u64::from(self.config.minutes_for(phase)) * 60_000;
                self.state = State::Running {
                    phase,
                    end_time_ms: now_ms + duration_ms,
                };
            }
            State::Running { .. } | State::Paused { .. } => {}
        }
    }

    /// Running → Paused. 남은 시간을 보존한다. 그 외 상태에서는 무시된다.
    pub fn pause(&mut self, now_ms: u64) {
        if let State::Running { phase, end_time_ms } = self.state {
            self.state = State::Paused {
                phase,
                remaining_ms: end_time_ms.saturating_sub(now_ms),
            };
        }
    }

    /// Paused → Running. end_time을 재계산한다. 그 외 상태에서는 무시된다.
    pub fn resume(&mut self, now_ms: u64) {
        if let State::Paused {
            phase,
            remaining_ms,
        } = self.state
        {
            self.state = State::Running {
                phase,
                end_time_ms: now_ms + remaining_ms,
            };
        }
    }

    /// 어느 상태에서든 Idle로 되돌린다.
    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    /// Running이고 end_time이 지났으면 Finished로 전이하고 종료된 단계를 돌려준다.
    /// 그 외에는 None.
    pub fn poll(&mut self, now_ms: u64) -> Option<Phase> {
        if let State::Running { phase, end_time_ms } = self.state {
            if now_ms >= end_time_ms {
                self.state = State::Finished { phase };
                return Some(phase);
            }
        }
        None
    }

    /// 남은 시간(ms). 0 아래로 내려가지 않는다. Idle/Finished는 0.
    pub fn remaining_ms(&self, now_ms: u64) -> u64 {
        match self.state {
            State::Running { end_time_ms, .. } => end_time_ms.saturating_sub(now_ms),
            State::Paused { remaining_ms, .. } => remaining_ms,
            State::Idle | State::Finished { .. } => 0,
        }
    }

    /// 설정 교체. 진행 중(Running/Paused) 세션에는 영향을 주지 않고 다음 start부터 적용된다.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn config(&self) -> Config {
        self.config
    }

    pub fn snapshot(&self, now_ms: u64) -> Snapshot {
        match self.state {
            State::Idle => Snapshot::Idle,
            State::Running { phase, .. } => Snapshot::Running {
                phase,
                remaining_ms: self.remaining_ms(now_ms),
            },
            State::Paused {
                phase,
                remaining_ms,
            } => Snapshot::Paused {
                phase,
                remaining_ms,
            },
            State::Finished { phase } => Snapshot::Finished { phase },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;
    const T0: u64 = 1_700_000_000_000;

    fn 기본_타이머() -> Pomodoro {
        Pomodoro::new(Config::default())
    }

    #[test]
    fn 시작하면_남은_시간이_설정한_분과_같다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        assert_eq!(p.remaining_ms(T0), 25 * MIN);

        let mut p = Pomodoro::new(Config::new(50, 10).unwrap());
        p.start(Phase::Focus, T0);
        assert_eq!(p.remaining_ms(T0), 50 * MIN);
        p.reset();
        p.start(Phase::Break, T0);
        assert_eq!(p.remaining_ms(T0), 10 * MIN);
    }

    #[test]
    fn 일시정지하면_시간이_흘러도_남은_시간이_줄지_않는다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.pause(T0 + 5 * MIN);
        assert_eq!(p.remaining_ms(T0 + 5 * MIN), 20 * MIN);
        assert_eq!(p.remaining_ms(T0 + 60 * MIN), 20 * MIN);
    }

    #[test]
    fn 일시정지_후_재개하면_남은_시간이_보존된다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.pause(T0 + 5 * MIN);
        p.resume(T0 + 30 * MIN);
        assert_eq!(p.remaining_ms(T0 + 30 * MIN), 20 * MIN);
        assert_eq!(p.remaining_ms(T0 + 31 * MIN), 19 * MIN);
    }

    #[test]
    fn 종료_시각이_지나면_finished가_되고_남은_시간은_0_아래로_내려가지_않는다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        assert_eq!(p.poll(T0 + 24 * MIN), None);
        assert_eq!(p.poll(T0 + 25 * MIN), Some(Phase::Focus));
        assert_eq!(p.remaining_ms(T0 + 30 * MIN), 0);
        assert!(matches!(
            p.snapshot(T0 + 30 * MIN),
            Snapshot::Finished { phase: Phase::Focus }
        ));
    }

    #[test]
    fn 리셋하면_idle로_돌아간다() {
        // Running에서
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.reset();
        assert!(matches!(p.snapshot(T0), Snapshot::Idle));

        // Paused에서
        p.start(Phase::Focus, T0);
        p.pause(T0 + MIN);
        p.reset();
        assert!(matches!(p.snapshot(T0), Snapshot::Idle));

        // Finished에서
        p.start(Phase::Focus, T0);
        p.poll(T0 + 25 * MIN);
        p.reset();
        assert!(matches!(p.snapshot(T0), Snapshot::Idle));
    }

    #[test]
    fn finished에서_start를_호출하면_새_세션이_running으로_시작된다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.poll(T0 + 25 * MIN);
        p.start(Phase::Break, T0 + 26 * MIN);
        assert!(matches!(
            p.snapshot(T0 + 26 * MIN),
            Snapshot::Running { phase: Phase::Break, .. }
        ));
        assert_eq!(p.remaining_ms(T0 + 26 * MIN), 5 * MIN);
    }

    #[test]
    fn running_중_start는_무시된다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.start(Phase::Break, T0 + MIN);
        assert!(matches!(
            p.snapshot(T0 + MIN),
            Snapshot::Running { phase: Phase::Focus, .. }
        ));
        assert_eq!(p.remaining_ms(T0 + MIN), 24 * MIN);
    }

    #[test]
    fn 오래_방치해도_벽시계_기준으로_정확하다() {
        // AE2: 25분 시작 후 10분 방치 → 남은 시간 15분
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        assert_eq!(p.remaining_ms(T0 + 10 * MIN), 15 * MIN);
    }

    #[test]
    fn 영분_설정은_거부된다() {
        assert!(Config::new(0, 5).is_err());
        assert!(Config::new(25, 0).is_err());
        assert!(Config::new(1, 1).is_ok());
    }

    #[test]
    fn finished_상태에서_pause와_resume은_무시된다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.poll(T0 + 25 * MIN);
        p.pause(T0 + 26 * MIN);
        assert!(matches!(
            p.snapshot(T0 + 26 * MIN),
            Snapshot::Finished { phase: Phase::Focus }
        ));
        p.resume(T0 + 26 * MIN);
        assert!(matches!(
            p.snapshot(T0 + 26 * MIN),
            Snapshot::Finished { phase: Phase::Focus }
        ));
    }

    #[test]
    fn running_중_설정_변경은_현재_세션의_end_time을_바꾸지_않는다() {
        let mut p = 기본_타이머();
        p.start(Phase::Focus, T0);
        p.set_config(Config::new(50, 10).unwrap());
        assert_eq!(p.remaining_ms(T0 + MIN), 24 * MIN);
        // 다음 start부터 새 설정이 적용된다
        p.reset();
        p.start(Phase::Focus, T0);
        assert_eq!(p.remaining_ms(T0), 50 * MIN);
    }
}
