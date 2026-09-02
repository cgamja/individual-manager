//! 핀볼 모드 — 켜면 착지 등급 판정을 건너뛰고 벽·천장·바닥이 전부 반사면이 된다.
//!
//! 왼쪽 클릭은 빠따가 아니라 채가 되어, 맞은 지점 반대 방향으로 날아간다.
//! `landing_of`를 지우지 않고 앞에서 가로채므로 끄면 그대로 돌아온다.
//!
//! **다른 펭귄도 반사면이다.** 쌍 판정([`Pet::bump_of`])과 그 결과를 한 마리에 먹이는
//! [`Pet::bumped`]가 여기 있고, 전 마리를 훑는 루프는 `Pets::collide_pinball`
//! (`pet/mod.rs`)이다 — 마리 하나의 `step`으로는 "옆 마리와 부딪혔는가"를 답할 수 없다.

use super::super::*;
use super::air::{clamp_throw, landing_of, Landing};

impl Pet {
    /// 이 마리와 `other`가 **이번 틱에** 부딪혔는가. 부딪혔으면 `(법선, 임펄스 크기)`를
    /// 준다 — 법선은 `other`에게서 **이 마리로** 향하는 단위벡터이고, 양쪽은 법선만
    /// 뒤집어 같은 [`Pet::bumped`]를 쓴다.
    ///
    /// **지금 위치가 아니라 지나온 경로로 본다.** 핀볼에서는 한 틱(최대 250ms)에 수백
    /// px을 날아가므로 지금 위치만 보면 서로를 스쳐 지나가면서도 어느 틱에도 반경 안에
    /// 안 잡힌다 (볼링 공·연쇄 판정과 똑같은 함정이다).
    ///
    /// 다만 볼링은 *움직이는 하나 → 서 있는 여럿*이라 점과 선분이었는데 여기서는 **둘 다
    /// 움직인다.** 두 선분의 최소 거리를 새로 짜는 대신 **상대 좌표로 접는다**: 상대
    /// 위치 `R(t) = A(t) - B(t)`는 t에 대해 직선이므로, 원점과 선분 `R0 → R1`의 거리가
    /// 곧 두 마리의 최근접 거리다. 그래서 `dist2_to_segment`를 그대로 재사용하고, 그
    /// 함수가 주는 최근접점이 그대로 충돌 법선이 된다.
    ///
    /// **난수를 쓰지 않는다** — 골든 수열(동작 시퀀스)이 그대로여야 한다.
    pub(in crate::pet) fn bump_of(
        &self,
        my_prev: (f64, f64),
        other: &Pet,
        other_prev: (f64, f64),
    ) -> Option<((f64, f64), f64)> {
        // 앱 전역 설정이라 실제로는 둘의 값이 같지만, 필드는 마리마다 있으므로 코어가
        // 스스로 방어한다.
        if !self.pinball || !other.pinball {
            return None;
        }
        // 들려 있는 마리는 사용자의 손 안에 있다. 손을 밀어낼 수는 없다.
        if self.behavior == Behavior::Dragged || other.behavior == Behavior::Dragged {
            return None;
        }

        let r0 = (my_prev.0 - other_prev.0, my_prev.1 - other_prev.1);
        let r1 = (
            self.center_x() - other.center_x(),
            self.center_y() - other.center_y(),
        );
        let (d2, near) = dist2_to_segment((0.0, 0.0), r0, r1);
        if d2 > PINBALL_COLLIDE_RADIUS * PINBALL_COLLIDE_RADIUS {
            return None;
        }

        let (vx, vy) = (self.vx - other.vx, self.vy - other.vy);
        let len = (near.0 * near.0 + near.1 * near.1).sqrt();
        let (nx, ny) = if len > f64::EPSILON {
            (near.0 / len, near.1 / len)
        } else {
            // 정확히 겹쳤다 — 다가온 방향의 반대로 민다. 그것도 0이면 둘 다 제자리에
            // 겹쳐 선 것이라 밀 이유가 없다 (겹쳐 서는 것은 평소에도 일어난다).
            let speed = (vx * vx + vy * vy).sqrt();
            if speed <= f64::EPSILON {
                return None;
            }
            (-vx / speed, -vy / speed)
        };

        // **다가가는 중일 때만 튕긴다.** 이 한 줄이 이미 겹친 쌍의 이중 타격과 잔진동을
        // 통째로 막으므로 위치를 억지로 떼어놓는 처리가 필요 없다.
        let approach = vx * nx + vy * ny;
        if approach >= 0.0 {
            return None;
        }
        // 질량이 같은 1차원 탄성 충돌. 접선 성분은 건드리지 않아 스치면 스친 만큼만 꺾인다.
        Some(((nx, ny), -(1.0 + PINBALL_BUMP_DAMPING) * approach / 2.0))
    }

    /// 다른 마리와 부딪혀 튕겨 나간다. `(nx, ny)`는 **상대에게서 이 마리로** 향하는
    /// 단위벡터, `j`는 [`bump_of`]가 준 임펄스 크기다.
    ///
    /// **부딪힘 전용 자세를 만들지 않는다** — 던져졌을 때와 똑같이 `Thrown`이 되어
    /// 포물선을 그리고 벽에 튕긴다. 물리를 두 벌 유지하지 않는 것이 볼링의
    /// `bowling_knocked`와 같은 선택이다.
    pub(in crate::pet) fn bumped(
        &mut self,
        now_ms: u64,
        nx: f64,
        ny: f64,
        j: f64,
        world_width: f64,
    ) {
        self.vx += j * nx;
        self.vy += j * ny;
        // 바닥에 선 마리는 위로 조금 띄운다. 수평으로만 밀리면 다음 틱에 곧바로 다시
        // 착지해 **한 틱 미끄러지고 끝난다** — 맞은 것이 안 보인다. `enter`가 `air`를
        // 켜기 전에 봐야 한다.
        if !self.air {
            let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
            if speed > f64::EPSILON {
                self.vy = -speed * PINBALL_BUMP_LIFT;
            }
        }
        // 연쇄가 쌓여도 던지기 상한을 넘지 않는다. 사람 손보다 빠른 펭귄은 눈이 못 쫓는다.
        let (vx, vy) = clamp_throw(self.vx, self.vy, world_width);
        self.vx = vx;
        self.vy = vy;
        if self.vx.abs() > 1.0 {
            self.facing = if self.vx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        self.enter(Behavior::Thrown, now_ms);
    }

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
