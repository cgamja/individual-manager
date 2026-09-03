//! 안물 — 묻지 않았다며 조잘거리며 춤춘다.
//!
//! 버튼으로만 시작한다. `pick_next`에 없어서 저절로는 안 나온다.

use super::super::*;

impl Pet {
    /// 설정 창의 "안물". 이미 하는 중이거나 들고 있으면 거부한다.
    pub fn start_dont_ask(&mut self, now_ms: u64) -> bool {
        if matches!(self.behavior, Behavior::Dragged | Behavior::DontAsk) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter(Behavior::DontAsk, now_ms + DONT_ASK_MS);
        true
    }

    pub(in crate::pet) fn tick_dont_ask(&mut self, now_ms: u64) {
        if now_ms >= self.behavior_until_ms {
            if self.air {
                self.enter(Behavior::Falling, now_ms);
            } else {
                self.enter_idle(now_ms);
            }
        }
    }
}

#[cfg(test)]
#[path = "dont_ask_tests.rs"]
mod tests;
