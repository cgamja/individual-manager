//! 볼링 — 핀이 되어 한 줄로 서고, 공이 지나가면 빙글빙글 돈다.
//!
//! **국면 넷 중 스스로 끝나는 것은 `Gather`와 `Scatter`뿐이다.** `Ready`와
//! `Struck`은 판(`Pets::bowling`)이 몰아 준다 — 여러 마리가 하나의 사건을
//! 공유하므로 "언제 흩어지나"를 마리 혼자서는 답할 수 없다 (KTD8). 대신
//! **판이 사라져도 영원히 서 있지 않도록** 안전 상한을 하나 들고 있는다 (R11).
//!
//! 진입은 사용자가 누르는 버튼뿐이다 — `pick_next`를 건드리지 않는다 (KTD1).
//! 확률 사다리에 분기를 하나 끼우면 뒤의 모든 확률이 밀려 튜닝된 빈도표가
//! 통째로 흐트러지고 골든 수열을 재기준화해야 한다.

use super::super::*;

impl Pet {
    /// 볼링 국면 진행 — 매 틱 `step`이 부른다.
    pub(in crate::pet) fn tick_bowling(
        &mut self,
        now_ms: u64,
        bowling: BowlingPhase,
        bounds: Bounds,
        dt: f64,
    ) {
        match bowling {
            BowlingPhase::Gather => self.tick_bowling_gather(now_ms, bounds, dt),
            BowlingPhase::Ready | BowlingPhase::Struck => {
                if now_ms >= self.behavior_until_ms {
                    self.enter_bowling(BowlingPhase::Scatter, now_ms);
                }
            }
            BowlingPhase::Scatter => {
                if now_ms >= self.behavior_until_ms {
                    if self.air {
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
        }
    }

    /// 자기 핀 자리로 걸어간다. 목적지를 갖는 지상 이동은 이게 처음이라
    /// `tick_walk`(방향과 시간만 안다)를 못 쓰고 `target`을 재사용한다 (KTD7).
    ///
    /// 공중에 있었으면 **내려오면서** 간다. 바닥으로 순간이동시키면
    /// "걸어서 간다"(R2)를 그 자리에서 어긴다.
    fn tick_bowling_gather(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        let tx = self.target.0;
        let dx = tx - self.x;
        if dx.abs() > 1.0 {
            self.facing = if dx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        self.x += dx.signum() * (BOWLING_GATHER_SPEED * dt).min(dx.abs());

        if self.air {
            let 남은 = (bounds.floor_y - self.y).max(0.0);
            self.y += (BOWLING_DESCENT_SPEED * dt).min(남은);
            if bounds.floor_y - self.y <= ARRIVE_EPSILON {
                self.y = bounds.floor_y;
                self.air = false;
            }
        }

        let 도착 = (tx - self.x).abs() <= ARRIVE_EPSILON && !self.air;
        if 도착 || now_ms >= self.behavior_until_ms {
            self.x = tx;
            self.air = false;
            self.enter_bowling(BowlingPhase::Ready, now_ms);
        }
    }

    /// 판이 열려 이 마리가 핀이 된다. 들려 있거나 이미 볼링 중이면 거절한다 —
    /// `start_fishing`·`start_slide`와 같은 꼴이다.
    pub fn start_bowling(&mut self, now_ms: u64, pin_x: f64, floor_y: f64) -> bool {
        if matches!(self.behavior, Behavior::Dragged | Behavior::Bowling { .. }) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.vx = 0.0;
        self.vy = 0.0;
        self.target = (pin_x, floor_y);
        self.enter_bowling(BowlingPhase::Gather, now_ms);
        true
    }

    /// 공이 지나갔다 — 판이 부른다. **이미 맞았거나 흩어지는 중이면 무시한다**:
    /// 되감으면 한 번 넘어진 펭귄이 다시 일어나 또 넘어진다.
    pub(in crate::pet) fn bowling_struck(&mut self, now_ms: u64) {
        if matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Gather | BowlingPhase::Ready
            }
        ) {
            self.enter_bowling(BowlingPhase::Struck, now_ms);
        }
    }

    /// 판이 끝났다 — 흩어진다. **모든 국면이 여기로 끝난다**: 곧장 유휴로 가면
    /// `.pg-all`에 걸린 변형이 한 프레임에 사라져 펭귄이 튄다
    /// (얼음낚시의 `Pack`, 발작의 `Pant`와 같은 자리다).
    pub(in crate::pet) fn bowling_scatter(&mut self, now_ms: u64) {
        if matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Gather | BowlingPhase::Ready | BowlingPhase::Struck
            }
        ) {
            self.enter_bowling(BowlingPhase::Scatter, now_ms);
        }
    }

    /// 아직 판에 서 있는가. 판이 매 틱 참여 목록을 추리는 데 쓴다 —
    /// 드래그·빠따로 다른 동작에 넘어간 마리는 여기서 걸러진다 (A4).
    pub(in crate::pet) fn is_bowling(&self) -> bool {
        matches!(self.behavior, Behavior::Bowling { .. })
    }

    /// 자기 자리에 다 섰는가. 판이 "전부 섰는가"를 묻는 자리다.
    pub(in crate::pet) fn bowling_stood(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Ready | BowlingPhase::Struck
            }
        )
    }

    fn enter_bowling(&mut self, phase: BowlingPhase, now_ms: u64) {
        if phase == BowlingPhase::Ready {
            // 공이 굴러오는 쪽을 본다 — 등을 보이고 서 있으면 맞는 게 안 보인다.
            self.facing = Facing::Left;
        }
        let hold = match phase {
            BowlingPhase::Scatter => BOWLING_SCATTER_MS,
            // 판이 몰아 주는 국면들. 이 값은 국면 길이가 아니라 **안전 상한**이다.
            _ => BOWLING_MAX_MS,
        };
        self.enter(Behavior::Bowling { bowling: phase }, now_ms + hold);
    }
}

#[cfg(test)]
#[path = "bowling_tests.rs"]
mod tests;
