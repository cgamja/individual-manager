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

/// 길이가 있으면 단위벡터로. 0이면 방향이라는 것이 없다.
fn 단위((x, y): (f64, f64)) -> Option<(f64, f64)> {
    let len = (x * x + y * y).sqrt();
    (len > f64::EPSILON).then_some((x / len, y / len))
}

impl Pet {
    /// 이 마리와 `other`가 **이번 틱에** 부딪혔는가. 부딪혔으면 `(법선, 임펄스 크기)`를
    /// 준다 — 법선은 `other`에게서 **이 마리로** 향하는 단위벡터이고, 양쪽은 법선만
    /// 뒤집어 같은 [`Pet::bumped`]를 쓴다.
    ///
    /// **지금 위치가 아니라 지나온 경로로 본다.** 핀볼에서는 한 틱(최대 250ms)에 수백
    /// px을 날아가므로 지금 위치만 보면 서로를 스쳐 지나가면서도 어느 틱에도 반경 안에
    /// 안 잡힌다 (볼링 공·연쇄 판정과 똑같은 함정이다). 다만 볼링은 *움직이는 하나 →
    /// 서 있는 여럿*이라 점과 선분이었는데 여기서는 **둘 다 움직인다.** 두 선분의 최소
    /// 거리를 새로 짜는 대신 **상대 좌표로 접는다**: 상대 위치 `R(t) = A(t) - B(t)`가
    /// t에 대해 직선이라 두 마리의 최근접 거리는 원점과 선분 `R0 → R1`의 거리와 같고,
    /// 그게 볼링에서 온 `dist2_to_segment`다. 여기까지가 넓은 그물이다.
    ///
    /// **법선은 최근접점이 아니라 닿는 순간의 상대 위치다.** 최근접점을 그대로 쓰면
    /// 판정이 조용히 죽는다: 최근접이 구간 **내부**라는 것은 이번 틱에 서로를 지나쳤다는
    /// 뜻이고, 그때 최근접 방향은 **정의상 상대 변위와 직교한다** — 상대속도의 법선
    /// 성분이 0이라 아래 접근 판정이 스침을 **전부** 기각한다. 이 함수가 존재하는 이유인
    /// 바로 그 경우가 사라지는데, 증상은 "부딪혔는데 아무 일도 안 일어난다"뿐이라 눈에
    /// 안 띈다. 그래서 그물에 걸리면 `|R0 + t·D| = 반경`의 첫 근을 따로 푼다.
    ///
    /// **상대속도도 `vx`/`vy`가 아니라 자취로 잰다** ([`Sweep`]). 그 둘은 던져졌을 때만
    /// 0이 아니라서, 한 틱 안에서 날아와 착지까지 끝낸 마리는 틱 끝에 속도가 0이고
    /// 미끄러지거나 헤엄치는 마리는 처음부터 0이다 — 화면을 가로질러 놓고 판정에는 서
    /// 있는 것으로 보인다. 대신 평소 동작까지 전부 보이게 되므로
    /// `PINBALL_BUMP_MIN_SPEED`가 걷기를 걸러낸다.
    ///
    /// **난수를 쓰지 않는다** — 골든 수열(동작 시퀀스)이 그대로여야 한다.
    pub(in crate::pet) fn bump_of(
        &self,
        mine: Sweep,
        other: &Pet,
        theirs: Sweep,
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

        let 내_중심 = (self.center_x(), self.center_y());
        let 상대_중심 = (other.center_x(), other.center_y());
        let r0 = (mine.from.0 - theirs.from.0, mine.from.1 - theirs.from.1);
        let r1 = (내_중심.0 - 상대_중심.0, 내_중심.1 - 상대_중심.1);
        let 반경2 = PINBALL_COLLIDE_RADIUS * PINBALL_COLLIDE_RADIUS;
        // 넓은 그물 — 이번 틱에 한 번도 반경 안에 안 들어왔으면 볼 것이 없다.
        if dist2_to_segment((0.0, 0.0), r0, r1).0 > 반경2 {
            return None;
        }

        let 시작2 = r0.0 * r0.0 + r0.1 * r0.1;
        let 닿는_자리 = if 시작2 <= 반경2 {
            // 틱 시작에 이미 겹쳐 있었다. 아래 접근 판정이 이중 타격을 막는다.
            r0
        } else {
            // `|R0 + t·D|² = 반경²`의 **첫** 근이 닿는 순간이다.
            let d = (r1.0 - r0.0, r1.1 - r0.1);
            let a = d.0 * d.0 + d.1 * d.1;
            if a <= f64::EPSILON {
                return None;
            }
            let b = 2.0 * (r0.0 * d.0 + r0.1 * d.1);
            let 판별식 = b * b - 4.0 * a * (시작2 - 반경2);
            if 판별식 < 0.0 {
                return None;
            }
            let t = (-b - 판별식.sqrt()) / (2.0 * a);
            if !(0.0..=1.0).contains(&t) {
                return None;
            }
            (r0.0 + d.0 * t, r0.1 + d.1 * t)
        };

        let 내_속도 = mine.velocity(내_중심);
        let 상대_속도 = theirs.velocity(상대_중심);
        let (vx, vy) = (내_속도.0 - 상대_속도.0, 내_속도.1 - 상대_속도.1);
        // 스치듯 지나가는 평소 동작까지 튕기면 걷기가 통째로 망가진다.
        if vx * vx + vy * vy < PINBALL_BUMP_MIN_SPEED * PINBALL_BUMP_MIN_SPEED {
            return None;
        }
        // 틱 시작에 좌표가 정확히 같았으면 방향이 없다 — 이번 틱에 벌어진 쪽을 본다.
        // 둘 다 0이면 상대 변위도 0이라 위 문턱에서 이미 걸렸다.
        let (nx, ny) = 단위(닿는_자리).or_else(|| 단위(r1))?;

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
    /// 단위벡터, `j`는 [`Pet::bump_of`]가 준 임펄스 크기다.
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
        // 바닥에 선 마리는 위로 조금 튼다. 수평으로만 밀리면 다음 틱에 곧바로 다시
        // 착지해 **한 틱 미끄러지고 끝난다** — 맞은 것이 안 보인다. `enter`가 `air`를
        // 켜기 전에 봐야 한다.
        //
        // **속력을 더하지 않고 방향만 튼다.** 세로를 그냥 얹으면 속력이 √(1+비율²)배로
        // 늘어 바닥 높이 충돌의 실효 반발 계수가 1에 가까워진다 — "부딪힘으로 속도가
        // 늘지 않는다"가 조용히 깨지고, 뒤엉킨 여덟 마리가 잘 안 멎는다.
        if !self.air {
            let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
            if speed > f64::EPSILON {
                if self.vx.abs() > f64::EPSILON {
                    self.vy = -speed * PINBALL_BUMP_LIFT;
                    let 가로 = (speed * speed - self.vy * self.vy).max(0.0).sqrt();
                    self.vx = self.vx.signum() * 가로;
                } else {
                    // 세로로만 밀렸다 — 틀 가로 성분이 없으니 통째로 위로 돌린다.
                    self.vy = -speed;
                }
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
        // 얻어맞는 것도 자극이다. 안 찍으면 랠리 한복판에서 졸기 문턱에 걸린다.
        self.last_stimulus_ms = now_ms;
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
