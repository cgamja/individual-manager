//! 핀볼 모드 — 켜면 착지 등급 판정을 건너뛰고 벽·천장·바닥이 전부 반사면이 된다.
//!
//! 왼쪽 클릭은 빠따가 아니라 채가 되어, 맞은 지점 반대 방향으로 날아간다.
//! `landing_of`를 지우지 않고 앞에서 가로채므로 끄면 그대로 돌아온다.

use super::super::*;
use super::air::{landing_of, Landing};

impl Pet {
    /// 핀볼 모드를 켜고 끈다. 앱 전역 설정이라 브릿지가 전 마리에 건다.
    pub fn set_pinball(&mut self, on: bool) {
        self.pinball = on;
    }

    pub fn pinball(&self) -> bool {
        self.pinball
    }

    /// 바닥에 닿는 순간의 착지를 고른다.
    pub(in crate::pet) fn landing(&self, impact_vy: f64) -> Landing {
        if !self.pinball {
            return landing_of(impact_vy);
        }
        if impact_vy >= BOUNCE_MIN_SPEED {
            Landing::Bounce(-impact_vy * PINBALL_DAMPING)
        } else {
            Landing::Settle(Behavior::Land, LAND_MS)
        }
    }

    /// 벽·천장에서 튈 때 남는 속도 비율. 핀볼에서 벽은 범퍼다.
    pub(in crate::pet) fn wall_damping(&self) -> f64 {
        if self.pinball {
            PINBALL_DAMPING
        } else {
            BOUNCE_DAMPING
        }
    }

    /// 바닥에서 통통 튈 때 **가로** 속도에 남는 비율. 세로는 `landing`이 정한다.
    pub(in crate::pet) fn floor_vx_damping(&self) -> f64 {
        if self.pinball {
            PINBALL_DAMPING
        } else {
            FLOOR_BOUNCE_DAMPING
        }
    }

    /// 채로 후려친다 (핀볼 모드).
    pub(in crate::pet) fn flip(&mut self, now_ms: u64, world: &World, nx: f64, ny: f64) {
        let len = (nx * nx + ny * ny).sqrt();
        let (dx, dy) = if len > f64::EPSILON {
            (-nx / len, -ny / len)
        } else {
            (0.0, -1.0)
        };
        let speed = (world.width() * PINBALL_HIT_WORLDS_PER_SEC).max(THROW_MIN_SPEED);
        self.vx = dx * speed;
        self.vy = dy * speed;
        if self.vx.abs() > 1.0 {
            self.facing = if self.vx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        self.enter(Behavior::Thrown, now_ms);
    }
}

#[cfg(test)]
#[path = "pinball_tests.rs"]
mod tests;
