//! 얼음낚시 — 구멍 뚫기·드리우기·입질·잡음·꽝·정리 여섯 국면을 돈다.

use super::super::*;

impl Pet {
    /// 얼음낚시 국면 진행 — 매 틱. 이 부른다.
    ///
    /// **위치를 건드리지 않는다** — 앉은 자리에서 한다.
    pub(in crate::pet) fn tick_fishing(&mut self, now_ms: u64, fishing: FishingPhase) {
        // 위치를 건드리지 않는다 — 앉은 자리에서 한다 (R5).
        // **국면 도중에 자르지 않는다**: 예산 확인은 드리우기로
        // 들어가는 순간에만 한다. 중간에 끊으면 낚싯대를 든 채
        // 사라지거나 채는 동작이 반쯤에서 잘린다.
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
                // **잡아도 판이 끝나지 않는다.** 잡을 때마다 끝내면 판
                // 길이가 40% 확률에 좌우돼 중앙값이 20초 아래로 내려간다
                // — 졸기(12~25초)보다 짧아져서 "가장 긴 동작"이라는
                // 존재 이유가 사라진다. 예산 하나만 판 길이를 정한다.
                FishingPhase::Miss | FishingPhase::Catch => {
                    self.enter_fishing_wait(now_ms)
                }
                FishingPhase::Pack => {
                    if self.air {
                        // 허공에서 낚시했으면 이제 마저 떨어진다
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
        }
    }

    /// 사용자가 시켜서 낚시를 시작한다 (설정 창의 "얼음낚시").
    ///
    /// **고도를 그대로 물려받는다.** 헤엄치는 중에 시키면 그 높이에 그대로 앉아
    /// 허공에서 낚시한다 — 얼음도 물도 없는 데서 낚싯대를 드리우는 게 이 앱의
    /// 결에 맞는다 (PRINCIPLE 1). 바닥으로 끌어내리면 헤엄치다 순간이동한다.
    ///
    /// **들려 있을 때만 거절한다.** 손에 쥔 채로 낚시를 시작하면 놓는 순간
    /// 낙하와 낚시가 겹친다. 시작했으면 참을 돌려주므로, 부르는 쪽이 "왜 아무
    /// 일도 없나"를 설명할 수 있다.
    ///
    /// 자극 시각을 갱신한다 — 시켜 놓고 5분 뒤에 조는 건 이상하다.
    pub fn start_fishing(&mut self, now_ms: u64) -> bool {
        // 이미 낚시 중이면 거절한다 — 이유는 `start_slide`와 같다
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
    ///
    /// 예산을 **여기서 한 번만** 뽑아 절대 시각으로 들고 있는다 — 국면이
    /// 몇 바퀴를 돌든 한 판의 길이는 이 값 하나가 정한다.
    pub(in crate::pet) fn enter_ice_fishing(&mut self, now_ms: u64) {
        self.fishing_until_ms = now_ms + self.range(FISHING_SESSION_MS);
        self.enter_fishing(FishingPhase::Dig, now_ms + FISHING_DIG_MS);
    }

    /// 드리우기로 들어가거나, 예산이 다 됐으면 일어난다.
    ///
    /// **구멍을 다 뚫었을 때·잡았을 때·꽝을 봤을 때가 이 함수를 공유한다.** 예산을
    /// 보는 코드가 두 벌이 되면 한쪽만 고쳐지고 조용히 갈라진다 (`hit_wall`과 같은 이유).
    /// 판이 끝나는 길도 여기 하나뿐이므로 **모든 판은 `Pack`을 거친다.**
    ///
    /// 나가는 길이 `get_up`이 아닌 것은 의도다 — 넘어졌다 일어난 뒤와 달리,
    /// 30초 얌전히 앉아 있다가 갑자기 약을 올리는 건 결이 다르다.
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
