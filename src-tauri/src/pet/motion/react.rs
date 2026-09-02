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

    /// 옆에서 휘두른 방망이에 맞아 날아간다 — **`Pets`가 부른다.** `forward`는
    /// 때린 마리가 보는 방향(양수면 오른쪽)이고, 여기서 앞으로 비스듬히 위로
    /// 뜨는 속도가 된다.
    ///
    /// **`whack_seq`를 올리지 않는다.** 그 값은 웹뷰에서 "방망이를 한 번
    /// 휘두른다"는 신호인데 이 마리는 맞는 쪽이다 — 올리면 방망이가 두 개 보인다.
    /// 대신 "퍽" 소리도 안 난다(핀볼 채 타격과 같은 취급이다).
    ///
    /// **착지 등급을 그대로 탄다.** 사용자가 방금 클릭해서 만든 결과라 철푸덕이
    /// 나오는 편이 "내가 날렸다"는 인과가 선명하다 — 저절로 나는 철푸덕과 성격이
    /// 다르다.
    pub(in crate::pet) fn swing_knocked(&mut self, now_ms: u64, forward: f64, world_width: f64) {
        self.last_stimulus_ms = now_ms;
        let speed = (world_width * SWING_KNOCK_WORLDS_PER_SEC).max(THROW_MIN_SPEED);
        // (앞 1, 위 LIFT)를 정규화한다 — 각도를 바꿔도 세기가 안 따라 변하게.
        let len = (1.0 + SWING_KNOCK_LIFT * SWING_KNOCK_LIFT).sqrt();
        self.vx = if forward < 0.0 { -speed } else { speed } / len;
        self.vy = -SWING_KNOCK_LIFT * speed / len;
        self.facing = if self.vx > 0.0 {
            Facing::Right
        } else {
            Facing::Left
        };
        self.enter(Behavior::Thrown, now_ms);
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
