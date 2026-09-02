//! 지상 이동 — 걷기·방향 전환·슬라이딩·굴러떨어지기와 벽 판정.

use super::super::*;

impl Pet {
    pub(in crate::pet) fn tick_walk(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        self.x += self.facing.sign() * WALK_SPEED * dt;
        self.after_ground_move(now_ms, bounds);
    }

    pub(in crate::pet) fn tick_turn(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            self.facing = self.facing.flipped();
            let until = now_ms + self.range(WALK_MS);
            self.enter(Behavior::Walk, until);
        }
    }

    pub(in crate::pet) fn tick_slide(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        let remaining =
            self.behavior_until_ms.saturating_sub(now_ms) as f64 / SLIDE_MS as f64;
        self.x += self.facing.sign() * self.slide_speed * remaining * dt;
        self.after_ground_move(now_ms, bounds);
    }

    pub(in crate::pet) fn tick_tumble(&mut self, now_ms: u64, dt: f64) {
        let remaining =
            self.behavior_until_ms.saturating_sub(now_ms) as f64 / TUMBLE_MS as f64;
        self.x += self.facing.sign() * TUMBLE_SPEED * remaining * dt;
        if now_ms >= self.behavior_until_ms {
            self.get_up(now_ms);
        }
    }

    /// 지상 이동 한 틱을 마무리한다 — 경계에 닿았으면 벽 반응, 시간이 다 됐으면 다음 동작.
    fn after_ground_move(&mut self, now_ms: u64, bounds: Bounds) {
        if bounds.right <= bounds.left {
            self.x = bounds.left;
            if now_ms >= self.behavior_until_ms {
                self.pick_next(now_ms, bounds);
            }
        } else if self.x <= bounds.left {
            self.x = bounds.left;
            self.hit_wall(now_ms);
        } else if self.x >= bounds.right {
            self.x = bounds.right;
            self.hit_wall(now_ms);
        } else if now_ms >= self.behavior_until_ms {
            self.pick_next(now_ms, bounds);
        }
    }

    /// 벽에 닿았을 때 — 얌전히 돌아서거나, 그대로 박고 굴러 넘어진다.
    pub(in crate::pet) fn hit_wall(&mut self, now_ms: u64) {
        if self.range((0, 99)) < TUMBLE_AT_WALL_PERCENT {
            self.facing = self.facing.flipped();
            self.enter(Behavior::Tumble, now_ms + TUMBLE_MS);
        } else {
            self.enter(Behavior::Turn, now_ms + TURN_MS);
        }
    }

    /// 미끄러지기 시작한다. **출발 속도를 여기서 한 번 뽑는다** — 길이는 고정이고
    /// 이 값이 거리를 정하므로, 매 틱 뽑으면 감속이 들쭉날쭉해진다.
    pub(in crate::pet) fn enter_slide(&mut self, now_ms: u64) {
        let (lo, hi) = SLIDE_SPEED;
        self.slide_speed = lo + self.fraction() * (hi - lo);
        self.enter(Behavior::Slide, now_ms + SLIDE_MS);
    }

    /// 사용자가 시켜서 미끄러진다 (설정 창의 "슬라이딩").
    pub fn start_slide(&mut self, now_ms: u64) -> bool {
        if self.air || matches!(self.behavior, Behavior::Dragged | Behavior::Slide) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_slide(now_ms);
        true
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
