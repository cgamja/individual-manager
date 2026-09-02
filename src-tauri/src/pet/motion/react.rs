//! 클릭에 대한 반응 — 방망이 휘두르기, 약 올리기, 빽빽거리기.
//!
//! 짧은 간격으로 스무 번 연달아 맞으면 스윙을 건너뛰고 곧바로 화낸다.

use super::super::*;

impl Pet {
    pub(in crate::pet) fn tick_swing(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            if self.air {
                self.enter(Behavior::Falling, now_ms);
            } else {
                self.enter_sassy(now_ms);
            }
        }
    }

    pub(in crate::pet) fn tick_sassy(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            if self.air {
                self.enter(Behavior::Falling, now_ms);
            } else {
                self.enter_idle(now_ms);
            }
        }
    }

    pub(in crate::pet) fn tick_squawk(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            if self.air {
                self.enter(Behavior::Falling, now_ms);
            } else {
                self.enter_idle(now_ms);
            }
        }
    }

    /// 클릭 — 졸고 있어도 깨워서 놀라게 한다 (R5).
    pub fn whack(&mut self, now_ms: u64, world: &World, nx: f64, ny: f64) {
        self.last_stimulus_ms = now_ms;
        if self.pinball {
            self.flip(now_ms, world, nx, ny);
            return;
        }
        self.whack_seq += 1;
        self.vx = 0.0;
        self.vy = 0.0;

        if now_ms < self.squawk_until_ms {
            self.last_whack_ms = Some(now_ms);
            self.enter_squawk(now_ms);
            return;
        }

        self.whack_run = match self.last_whack_ms {
            Some(last) if now_ms.saturating_sub(last) <= SQUAWK_GAP_MS => self.whack_run + 1,
            _ => 1,
        };
        self.last_whack_ms = Some(now_ms);
        if self.whack_run >= SQUAWK_WHACK_COUNT {
            self.enter_squawk(now_ms);
            return;
        }
        self.enter(Behavior::Swing, now_ms + SWING_MS);
    }

    /// 사용자가 시켜서 빽빽거린다 (설정 창의 "빽빽거리기").
    pub fn start_squawk(&mut self, now_ms: u64) -> bool {
        if matches!(self.behavior, Behavior::Dragged | Behavior::Squawk) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_squawk(now_ms);
        true
    }

    /// 빽빽거리기 한 판을 시작한다. 연타와 "시켜보기"가 이 한 곳을 공유한다 —
    /// 예산과 카운터 초기화를 두 벌로 만들면 한쪽만 고쳐지고 조용히 갈라진다.
    pub(in crate::pet) fn enter_squawk(&mut self, now_ms: u64) {
        self.whack_run = 0;
        self.squawk_until_ms = now_ms + SQUAWK_MS;
        self.enter(Behavior::Squawk, self.squawk_until_ms);
    }

    /// 넘어졌다 일어난 뒤 — 대체로 약을 올리고, 아니면 그냥 유휴로 간다.
    pub(in crate::pet) fn get_up(&mut self, now_ms: u64) {
        if self.range((0, 99)) < SASSY_AFTER_LAND_PERCENT {
            self.enter_sassy(now_ms);
        } else {
            self.enter_idle(now_ms);
        }
    }

    /// 싸가지 반응 하나를 고른다 — 직전과 같은 종류는 피한다.
    pub(in crate::pet) fn enter_sassy(&mut self, now_ms: u64) {
        let mut sassy = SASSY_KINDS[self.range((0, 4)) as usize];
        if Some(sassy) == self.last_sassy {
            let next =
                (SASSY_KINDS.iter().position(|k| *k == sassy).unwrap() + 1) % SASSY_KINDS.len();
            sassy = SASSY_KINDS[next];
        }
        self.last_sassy = Some(sassy);
        self.enter(Behavior::Sassy { sassy }, now_ms + SASSY_MS);
    }
}

#[cfg(test)]
#[path = "react_tests.rs"]
mod tests;
