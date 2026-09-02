//! 발작 — 아주 낮은 확률로 사방을 튀어 다니다 숨을 고른다.

use super::super::*;

impl Pet {
    /// 발작 국면 진행 — 매 틱. `step`이 부른다.
    pub(in crate::pet) fn tick_freakout(
        &mut self,
        now_ms: u64,
        freakout: FreakoutPhase,
        bounds: Bounds,
        dt: f64,
    ) {
        match freakout {
            FreakoutPhase::Dash => {
                let (tx, ty) = self.target;
                let (dx, dy) = (tx - self.x, ty - self.y);
                let dist = (dx * dx + dy * dy).sqrt();
                if now_ms >= self.behavior_until_ms {
                    self.behavior_until_ms = now_ms + FREAKOUT_MS.1;
                    self.freakout_go_home(now_ms, bounds);
                } else if dist <= ARRIVE_EPSILON {
                    if now_ms < self.freakout_until_ms {
                        self.target = self.next_freakout_target(bounds);
                    } else {
                        self.freakout_go_home(now_ms, bounds);
                    }
                } else {
                    let step = (FREAKOUT_SPEED * dt).min(dist);
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
            FreakoutPhase::Pant => {
                if now_ms >= self.behavior_until_ms {
                    self.enter_idle(now_ms);
                }
            }
        }
    }

    /// 발작 한 판을 시작한다.
    pub(in crate::pet) fn enter_freakout(&mut self, now_ms: u64) {
        self.freakout_until_ms = now_ms + self.range(FREAKOUT_MS);
        self.target = (self.x, self.y);
        let until = self.freakout_until_ms + FREAKOUT_MS.1;
        self.enter(
            Behavior::Freakout {
                freakout: FreakoutPhase::Dash,
            },
            until,
        );
    }

    /// 다음으로 튈 곳. 방향은 균등, 거리는 [`FREAKOUT_HOP`]에서 뽑고 **경계 안으로
    /// 자른다** — 세계는 화면 하나이고 경계는 벽이다 (PRINCIPLE 2).
    fn next_freakout_target(&mut self, bounds: Bounds) -> (f64, f64) {
        let angle = self.fraction() * std::f64::consts::TAU;
        let (lo, hi) = FREAKOUT_HOP;
        let hop = lo + self.fraction() * (hi - lo);
        let tx = (self.x + angle.cos() * hop).clamp(bounds.left, bounds.right.max(bounds.left));
        let ty = (self.y + angle.sin() * hop).clamp(bounds.top.min(bounds.floor_y), bounds.floor_y);
        (tx, ty)
    }

    /// 판을 접는다 — 바닥이면 숨을 고르고, 공중이면 바닥을 목적지로 돌린다.
    fn freakout_go_home(&mut self, now_ms: u64, bounds: Bounds) {
        if self.y >= bounds.floor_y - ARRIVE_EPSILON {
            self.enter_freakout_pant(now_ms);
        } else {
            self.target = (self.x, bounds.floor_y);
        }
    }

    /// 바닥에서 숨을 고른다. **모든 판이 이 국면으로 끝난다** — 곧장 유휴로 가면
    /// `.pg-all`에 걸린 변형이 한 프레임에 사라져 펭귄이 튄다 (얼음낚시의 `Pack`).
    fn enter_freakout_pant(&mut self, now_ms: u64) {
        self.enter(
            Behavior::Freakout {
                freakout: FreakoutPhase::Pant,
            },
            now_ms + FREAKOUT_PANT_MS,
        );
    }

    /// 사용자가 시켜서 발작한다 (설정 창의 "발작").
    pub fn start_freakout(&mut self, now_ms: u64) -> bool {
        if matches!(self.behavior, Behavior::Dragged | Behavior::Freakout { .. }) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_freakout(now_ms);
        true
    }
}

#[cfg(test)]
#[path = "freakout_tests.rs"]
mod tests;
