//! 볼링 — 핀이 되어 **화면 세로 중앙**에 삼각형으로 뜨고, 공에 맞으면
//! 튕겨 나간다.
//!
//! **맞은 뒤의 상태는 볼링이 아니다.** 맞은 핀은 `Thrown`이 되어 평소의 던지기
//! 물리를 그대로 탄다 — 포물선, 벽 튕김, 착지 네 갈래. 그래서 국면이 셋뿐이고
//! (`Gather`/`Ready`/`Scatter`) "맞은 상태"라는 국면이 따로 없다. 맞은 마리는
//! `Behavior::Bowling`이 아니게 되므로 판이 다음 틱에 알아서 참여 목록에서
//! 뺀다 (2026-09-02 사용자 지시).
//!
//! **스스로 끝나는 것은 `Gather`와 `Scatter`뿐이다.** `Ready`는
//! 판(`Pets::bowling`)이 몰아 준다 — 여러 마리가 하나의 사건을 공유하므로
//! "언제 흩어지나"를 마리 혼자서는 답할 수 없다 (KTD8). 대신 **판이 사라져도
//! 영원히 떠 있지 않도록** 안전 상한을 하나 들고 있는다 (R11).
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
            BowlingPhase::Ready => {
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

    /// 자기 핀 자리로 **날아간다.** 목적지가 공중의 한 점이라 헤엄(`tick_swim`)과
    /// 같은 꼴로 `target`을 향해 곧장 간다 — 순간이동하지 않는다 (R2).
    fn tick_bowling_gather(&mut self, now_ms: u64, _bounds: Bounds, dt: f64) {
        let (tx, ty) = self.target;
        let (dx, dy) = (tx - self.x, ty - self.y);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
            self.x = tx;
            self.y = ty;
            self.enter_bowling(BowlingPhase::Ready, now_ms);
            return;
        }
        let step = (BOWLING_GATHER_SPEED * dt).min(dist);
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

    /// 판이 열려 이 마리가 핀이 된다. 들려 있거나 이미 볼링 중이면 거절한다 —
    /// `start_fishing`·`start_slide`와 같은 꼴이다.
    pub fn start_bowling(&mut self, now_ms: u64, pin_x: f64, pin_y: f64) -> bool {
        // **흩어지는 중(`Scatter`)은 거절하지 않는다.** 판이 끝나면 곧바로
        // 버튼이 살아나는데, 그 0.6초 동안 새 판을 거절하면 눌리는데 아무 일도
        // 안 일어나는 구간이 생긴다.
        if matches!(
            self.behavior,
            Behavior::Dragged
                | Behavior::Bowling {
                    bowling: BowlingPhase::Gather | BowlingPhase::Ready
                }
        ) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.vx = 0.0;
        self.vy = 0.0;
        self.target = (pin_x, pin_y);
        self.enter_bowling(BowlingPhase::Gather, now_ms);
        true
    }

    /// 맞아서 튕겨 나간다 — 판이 부른다. `(dx, dy)`는 때린 것에서 이 마리로
    /// 향하는 방향이다.
    ///
    /// **볼링 전용 자세를 만들지 않는다.** 평소 던져졌을 때와 똑같이 `Thrown`이
    /// 되어 포물선을 그리고 벽에 튕기고 착지 네 갈래를 탄다 — 맞은 핀이 제자리에서
    /// 도는 것보다 이쪽이 훨씬 볼링답고, 물리를 두 벌 유지하지 않아도 된다
    /// (2026-09-02 사용자 지시). 이 순간부터 이 마리는 판의 참여자가 아니다.
    pub(in crate::pet) fn bowling_knocked(&mut self, now_ms: u64, dx: f64, dy: f64, world: f64) {
        let len = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = if len > f64::EPSILON {
            (dx / len, dy / len)
        } else {
            // 정확히 겹쳤다 — 공이 가던 쪽으로 민다.
            (1.0, 0.0)
        };
        let speed = (world * BOWLING_KNOCK_WORLDS_PER_SEC).max(BOWLING_MIN_MAX_SPEED);
        self.vx = ux * speed;
        self.vy = uy * speed;
        if self.vx.abs() > 1.0 {
            self.facing = if self.vx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        self.enter(Behavior::Thrown, now_ms);
    }

    /// 판이 끝났다 — 흩어진다. **모든 국면이 여기로 끝난다**: 곧장 유휴로 가면
    /// `.pg-all`에 걸린 변형이 한 프레임에 사라져 펭귄이 튄다
    /// (얼음낚시의 `Pack`, 발작의 `Pant`와 같은 자리다).
    pub(in crate::pet) fn bowling_scatter(&mut self, now_ms: u64) {
        if matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Gather | BowlingPhase::Ready
            }
        ) {
            self.enter_bowling(BowlingPhase::Scatter, now_ms);
        }
    }

    /// 아직 판에 서 있는가. 판이 매 틱 참여 목록을 추리는 데 쓴다 —
    /// 드래그·빠따로 다른 동작에 넘어간 마리는 여기서 걸러진다 (A4).
    pub(in crate::pet) fn is_bowling(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Gather | BowlingPhase::Ready
            }
        )
    }

    /// 자기 자리에 다 떴는가. 판이 "전부 섰는가"를 묻는 자리다.
    pub(in crate::pet) fn bowling_stood(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Bowling {
                bowling: BowlingPhase::Ready
            }
        )
    }

    /// 아직 날아가는 중인가. 판이 연쇄 목록을 추리는 데 쓴다.
    pub(in crate::pet) fn is_flying(&self) -> bool {
        matches!(self.behavior, Behavior::Thrown)
    }

    fn enter_bowling(&mut self, phase: BowlingPhase, now_ms: u64) {
        if phase == BowlingPhase::Ready {
            // 공이 굴러오는 쪽을 본다 — 등을 보이고 떠 있으면 맞는 게 안 보인다.
            self.facing = Facing::Left;
            self.vx = 0.0;
            self.vy = 0.0;
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
