//! 얼음낚시 — 구멍 뚫기·드리우기·입질·잡음·꽝·정리 여섯 국면을 돈다.

use super::super::*;

impl Pet {
    /// 얼음낚시 국면 진행 — 매 틱. 이 부른다.
    pub(in crate::pet) fn tick_fishing(&mut self, now_ms: u64, fishing: FishingPhase) {
        if now_ms >= self.behavior_until_ms {
        match fishing {
            FishingPhase::Dig => self.enter_fishing_wait(now_ms),
            FishingPhase::Wait => self.enter_fishing(
                FishingPhase::Bite,
                now_ms + FISHING_BITE_MS,
            ),
            FishingPhase::Bite => {
                let (phase, hold) =
                    if self.range((0, 99)) < FISHING_CATCH_PERCENT {
                        (FishingPhase::Catch, FISHING_CATCH_MS)
                    } else {
                        (FishingPhase::Miss, FISHING_MISS_MS)
                    };
                self.enter_fishing(phase, now_ms + hold);
            }
            FishingPhase::Miss | FishingPhase::Catch => {
                self.enter_fishing_wait(now_ms)
            }
            FishingPhase::Pack => {
                if self.air {
                    self.enter(Behavior::Falling, now_ms);
                } else {
                    self.enter_idle(now_ms);
                }
            }
        }
        }
    }

    /// 사용자가 시켜서 낚시를 시작한다 (설정 창의 "얼음낚시").
    pub fn start_fishing(&mut self, now_ms: u64) -> bool {
        if matches!(
            self.behavior,
            Behavior::Dragged | Behavior::IceFishing { .. }
        ) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_ice_fishing(now_ms);
        true
    }

    /// 얼음낚시 한 판을 시작한다. 구멍 뚫기부터다.
    pub(in crate::pet) fn enter_ice_fishing(&mut self, now_ms: u64) {
        self.fishing_until_ms = now_ms + self.range(FISHING_SESSION_MS);
        self.enter_fishing(FishingPhase::Dig, now_ms + FISHING_DIG_MS);
    }

    /// 드리우기로 들어가거나, 예산이 다 됐으면 일어난다.
    fn enter_fishing_wait(&mut self, now_ms: u64) {
        if now_ms >= self.fishing_until_ms {
            self.enter_fishing(FishingPhase::Pack, now_ms + FISHING_PACK_MS);
            return;
        }
        let until = now_ms + self.range(FISHING_WAIT_MS);
        self.enter_fishing(FishingPhase::Wait, until);
    }

    fn enter_fishing(&mut self, fishing: FishingPhase, until_ms: u64) {
        self.enter(Behavior::IceFishing { fishing }, until_ms);
    }
}

#[cfg(test)]
#[path = "fishing_tests.rs"]
mod tests;
