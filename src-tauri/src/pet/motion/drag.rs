//! 드래그 — 집어 들고, 옮기고, 놓거나 던진다. 들고 있는 동안 자율 이동이 멈춘다.

use super::super::*;
use super::air::clamp_throw;

impl Pet {
    /// 드래그 시작 — 자율 이동을 멈춘다 (R6).
    pub fn drag_start(&mut self, now_ms: u64) {
        self.last_stimulus_ms = now_ms;
        self.vy = 0.0;
        self.enter(Behavior::Dragged, now_ms);
    }

    /// 드래그 이동량 반영. 들고 있는 동안에는 영역 밖으로도 따라간다 —
    /// 경계 정산은 놓는 시점(step의 clamp)에서 한다.
    pub fn drag_by(&mut self, dx: f64, dy: f64) {
        if self.behavior == Behavior::Dragged {
            self.x += dx;
            self.y += dy;
        }
    }

    /// 드래그 놓기 (R6, R12). 놓는 순간의 속도(논리 px/초)를 받아, 세게 던졌으면
    /// 그 속도로 포물선을 그리고 살짝 놓았으면 제자리에서 떨어진다.
    pub fn drag_end(&mut self, now_ms: u64, vx: f64, vy: f64, world: &World) {
        self.last_stimulus_ms = now_ms;
        let (vx, vy) = clamp_throw(vx, vy, world.width());
        if (vx * vx + vy * vy).sqrt() >= THROW_MIN_SPEED {
            self.vx = vx;
            self.vy = vy;
            self.enter(Behavior::Thrown, now_ms);
        } else {
            self.vx = 0.0;
            self.vy = 0.0;
            self.enter(Behavior::Falling, now_ms);
        }
    }
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
