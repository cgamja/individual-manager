//! 공중 — 헤엄·낙하·던져짐과 바닥에 닿았을 때의 착지 네 갈래.

use super::super::*;

/// 바닥에 닿았을 때 무엇을 하는가.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::pet) enum Landing {
    /// 통통 — 반동 속도를 갖고 다시 떠오른다.
    Bounce(f64),
    /// 멈춰 선다 — 동작과 그 동작을 유지할 시간.
    Settle(Behavior, u64),
}

/// 바닥에 닿는 순간의 낙하 속도로 착지를 고른다.
pub(in crate::pet) fn landing_of(impact_vy: f64) -> Landing {
    if impact_vy >= SPRAWL_MIN_IMPACT {
        Landing::Settle(Behavior::Sprawl, SPRAWL_MS)
    } else if impact_vy >= SPLAT_MIN_IMPACT {
        Landing::Settle(Behavior::Splat, SPLAT_MS)
    } else if impact_vy >= BOUNCE_MIN_SPEED {
        Landing::Bounce(-impact_vy * FLOOR_BOUNCE_DAMPING)
    } else {
        Landing::Settle(Behavior::Land, LAND_MS)
    }
}

/// 이 세계에서 허용하는 던지기 최고 속도 (논리 px/초).
pub(in crate::pet) fn throw_max_speed(world_width: f64) -> f64 {
    let width = if world_width > 0.0 {
        world_width
    } else {
        FALLBACK_WORLD_WIDTH
    };
    (width * THROW_MAX_WORLDS_PER_SEC).max(THROW_MIN_SPEED)
}

/// 던지기 속도를 방향은 유지한 채 세계 폭이 정한 상한으로 자른다.
pub(in crate::pet) fn clamp_throw(vx: f64, vy: f64, world_width: f64) -> (f64, f64) {
    let max = throw_max_speed(world_width);
    let speed = (vx * vx + vy * vy).sqrt();
    if speed <= max || speed == 0.0 {
        return (vx, vy);
    }
    let k = max / speed;
    (vx * k, vy * k)
}

impl Pet {
    pub(in crate::pet) fn tick_falling(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        self.vy += GRAVITY * dt;
        self.y += self.vy * dt;
        if self.y >= bounds.floor_y {
            self.y = bounds.floor_y;
            match self.landing(self.vy) {
                Landing::Bounce(vy) => self.vy = vy,
                Landing::Settle(behavior, hold) => {
                    self.vy = 0.0;
                    self.enter(behavior, now_ms + hold);
                }
            }
        }
    }

    pub(in crate::pet) fn tick_swim(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        let (tx, ty) = self.target;
        let (dx, dy) = (tx - self.x, ty - self.y);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
            if self.y >= bounds.floor_y - ARRIVE_EPSILON {
                self.vy = 0.0;
                self.enter(Behavior::Land, now_ms + LAND_MS);
            } else if !self.swim_descending && self.range((0, 99)) < SWIM_FREEFALL_PERCENT {
                self.vy = 0.0;
                self.enter(Behavior::Falling, now_ms);
            } else {
                self.target = (self.x, bounds.floor_y);
                self.swim_descending = true;
                let 남은 = (bounds.floor_y - self.y).max(0.0);
                self.behavior_until_ms =
                    now_ms + ((남은 / SWIM_DESCENT_SPEED) * 2_000.0) as u64 + 1_000;
            }
        } else {
            let speed = if self.swim_descending {
                SWIM_DESCENT_SPEED
            } else {
                SWIM_SPEED
            };
            let step = (speed * dt).min(dist);
            self.x += dx / dist * step;
            self.y += dy / dist * step;
            if dx.abs() > 1.0 {
                self.facing = if dx > 0.0 {
                    Facing::Right
                } else {
                    Facing::Left
                };
            }
        }
    }

    pub(in crate::pet) fn tick_thrown(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        self.vy += GRAVITY * dt;
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        if (self.x <= bounds.left && self.vx < 0.0) || (self.x >= bounds.right && self.vx > 0.0) {
            self.vx = -self.vx * self.wall_damping();
        }
        if self.y <= bounds.top && self.vy < 0.0 {
            self.vy = -self.vy * self.wall_damping();
        }
        if self.vx.abs() > 1.0 {
            self.facing = if self.vx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        if self.y >= bounds.floor_y && self.vy >= 0.0 {
            self.y = bounds.floor_y;
            match self.landing(self.vy) {
                Landing::Bounce(vy) => {
                    self.vy = vy;
                    self.vx *= self.floor_vx_damping();
                }
                Landing::Settle(behavior, hold) => {
                    self.vx = 0.0;
                    self.vy = 0.0;
                    self.enter(behavior, now_ms + hold);
                }
            }
        }
    }

    pub(in crate::pet) fn tick_landed(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            self.get_up(now_ms);
        }
    }

    /// 헤엄 목적지를 영역 안에서 무작위로 고른다 (R11).
    pub(in crate::pet) fn enter_swim(&mut self, now_ms: u64, bounds: Bounds) {
        self.swim_descending = false;
        let width = (bounds.right - bounds.left).max(0.0);
        let height = (bounds.floor_y - bounds.top).max(0.0);
        let tx = bounds.left + self.fraction() * width;
        let ty = bounds.top + self.fraction().powf(1.4) * height;
        self.target = (tx, ty);
        let dist = ((tx - self.x).powi(2) + (ty - self.y).powi(2)).sqrt();
        let budget_ms = ((dist / SWIM_SPEED) * 2_000.0) as u64 + 1_000;
        self.enter(Behavior::Swim, now_ms + budget_ms);
    }
}

#[cfg(test)]
#[path = "air_tests.rs"]
mod tests;
