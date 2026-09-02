//! 비치발리볼 — 코트로 날아가 서고, 받으러 뛰고, 때리고, 좋아하거나 약 오른다.
//!
//! **판이 마리를 몬다.** `Ready`와 `Chase`의 목적지를 정하는 것은 판
//! (`Pets::volleyball`)이고, 여기 있는 것은 "정해진 자리로 어떻게 가는가"뿐이다 —
//! "다음 공이 어디로 오나"는 마리 하나의 `step`으로 답할 수 없다 (볼링과 같은 규칙).
//!
//! **나가는 문은 `Cheer`/`Sulk`다.** 곧장 유휴로 가면 `.pg-all`에 걸린 변형이 한
//! 프레임에 사라져 펭귄이 튄다 (얼음낚시의 `Pack`, 발작의 `Pant`, 볼링의 `Scatter`와
//! 같은 자리). 그림은 **싸가지 반응의 keyframe을 CSS에서 그대로 참조**하지만
//! 국면은 `Volleyball` 안에 남는다 — `Behavior::Sassy`로 넘겨 버리면 축하하는 동안
//! **옷이 사라진다.**
//!
//! 진입은 사용자가 누르는 버튼뿐이다 — `pick_next`를 건드리지 않으므로 골든 수열이
//! 재기준화되지 않는다.

use super::super::*;

impl Pet {
    /// 비치발리볼 국면 진행 — 매 틱 `step`이 부른다.
    pub(in crate::pet) fn tick_volley(&mut self, now_ms: u64, volley: VolleyPhase, dt: f64) {
        match volley {
            VolleyPhase::Gather => self.tick_volley_gather(now_ms, dt),
            VolleyPhase::Chase => self.tick_volley_chase(now_ms, dt),
            // 판이 몰아 주는 국면. 여기 시각은 국면 길이가 아니라 **안전 상한**이라,
            // 다다랐다는 것은 판이 사라졌다는 뜻이다.
            VolleyPhase::Ready => {
                if now_ms >= self.behavior_until_ms {
                    self.leave_court(now_ms);
                }
            }
            VolleyPhase::Bump => {
                if now_ms >= self.behavior_until_ms {
                    self.enter_volley(VolleyPhase::Ready, now_ms);
                }
            }
            VolleyPhase::Cheer | VolleyPhase::Sulk => {
                if now_ms >= self.behavior_until_ms {
                    // **선 자리에서 그대로 떨어진다.** 판이 화면 세로 중앙이라
                    // 축하가 끝나면 공중에 떠 있는 상태다 — 볼링의 `Scatter`가
                    // 같은 자리에서 하는 것과 같다. 가로 자리는 안 건드리므로
                    // "선 그 자리에서 평소로"(R12)는 그대로다.
                    if self.air {
                        self.enter(Behavior::Falling, now_ms);
                    } else {
                        self.enter_idle(now_ms);
                    }
                }
            }
        }
    }

    /// 자기 자리로 **날아간다.** 목적지가 화면 세로 중앙의 한 점이라 헤엄·볼링
    /// 모으기와 같은 꼴로 `target`을 향해 곧장 간다 — 순간이동하지 않는다.
    fn tick_volley_gather(&mut self, now_ms: u64, dt: f64) {
        let (tx, ty) = self.target;
        let (dx, dy) = (tx - self.x, ty - self.y);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
            self.x = tx;
            self.y = ty;
            self.enter_volley(VolleyPhase::Ready, now_ms);
            return;
        }
        let step = (VOLLEY_GATHER_SPEED * dt).min(dist);
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

    /// 공이 떨어질 자리로 **뛴다.** 가로만 움직인다 — 판의 높이를 유지한 채
    /// 옆으로 달린다. 도착하면 서서 기다린다.
    ///
    /// **자기 팀 범위 밖으로는 안 나간다.** 판이 목적지를 자기 코트 안에만 주지만,
    /// 그 보장이 여기 있어야 판이 실수해도 네트를 넘어가는 그림이 안 나온다.
    fn tick_volley_chase(&mut self, now_ms: u64, dt: f64) {
        if now_ms >= self.behavior_until_ms {
            self.leave_court(now_ms);
            return;
        }
        let (tx, hi) = (self.target.0, self.volley_span.1);
        let tx = tx.clamp(self.volley_span.0, hi);
        let dx = tx - self.x;
        if dx.abs() <= ARRIVE_EPSILON {
            self.x = tx;
            self.enter_volley(VolleyPhase::Ready, now_ms);
            return;
        }
        let step = (VOLLEY_CHASE_SPEED * dt).min(dx.abs());
        self.x += dx.signum() * step;
    }

    /// 판이 열려 이 마리가 코트로 간다. 들려 있거나 이미 판(볼링·비치발리볼)에
    /// 있으면 거절한다 — `start_fishing`·`start_bowling`과 같은 꼴이다.
    ///
    /// `spot`은 자기 자리(`Pet::x`/`y`), `span`은 자기 팀이 뛸 수 있는 x 범위(좌상단
    /// 기준), `face`는 네트를 보는 방향이다.
    pub fn start_volley(
        &mut self,
        now_ms: u64,
        spot: (f64, f64),
        span: (f64, f64),
        face: Facing,
    ) -> bool {
        if matches!(
            self.behavior,
            Behavior::Dragged | Behavior::Bowling { .. } | Behavior::Volleyball { .. }
        ) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.vx = 0.0;
        self.vy = 0.0;
        self.target = spot;
        self.volley_span = span;
        self.volley_face = face;
        self.enter_volley(VolleyPhase::Gather, now_ms);
        true
    }

    /// 판이 "이 공은 네가 받아라"라고 시킨다. 이미 서 있든 뛰는 중이든 목적지만
    /// 갈아 끼운다 — 국면을 새로 밟으면 CSS 애니메이션이 매번 되감긴다.
    pub(in crate::pet) fn volley_chase(&mut self, now_ms: u64, target_x: f64) {
        self.target = (target_x, self.target.1);
        if !matches!(
            self.behavior,
            Behavior::Volleyball {
                volley: VolleyPhase::Chase
            }
        ) {
            self.enter_volley(VolleyPhase::Chase, now_ms);
        }
    }

    /// 공을 때린다. **좌표는 안 바뀐다** — 뛰어오르는 그림은 웹뷰가 그린다
    /// (PRINCIPLE 4).
    pub(in crate::pet) fn volley_bump(&mut self, now_ms: u64) {
        self.enter_volley(VolleyPhase::Bump, now_ms);
    }

    /// 판이 끝났다 — 이겼으면 좋아하고 졌으면 약 오른다 (R11).
    ///
    /// **`enter_sassy`를 부르지 않는다.** 그쪽은 `range()`로 난수를 태우고 "직전과
    /// 같은 반응은 피한다"를 굴리는데, 여기서는 어느 반응인지가 이미 정해져 있고
    /// 무엇보다 **판이 마리의 난수를 태우면 안 된다.**
    pub(in crate::pet) fn volley_finish(&mut self, now_ms: u64, won: bool) {
        let phase = if won {
            VolleyPhase::Cheer
        } else {
            VolleyPhase::Sulk
        };
        // 웹뷰가 CSS에서 싸가지 반응의 keyframe을 재사용하므로, "직전에 무슨
        // 반응을 했나"의 기억도 함께 맞춰 둔다 — 판이 끝나자마자 클릭했을 때
        // 같은 그림이 두 번 나오지 않는다.
        self.last_sassy = Some(if won {
            SassyKind::ButtWiggle
        } else {
            SassyKind::TurnAway
        });
        self.enter(
            Behavior::Volleyball { volley: phase },
            now_ms + VOLLEY_CHEER_MS,
        );
    }

    /// 아직 판에 있는가. 판이 매 틱 참여 목록을 추리는 데 쓴다 — 드래그·빠따로
    /// 다른 동작에 넘어간 마리는 여기서 걸러진다.
    ///
    /// **`Cheer`/`Sulk`는 아니다.** 그 둘은 판이 이미 끝난 뒤의 여운이라, 판이
    /// 붙들고 있으면 참여자가 안 빠져 코트가 제때 걷히지 않는다.
    pub(in crate::pet) fn is_volleying(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Volleyball {
                volley: VolleyPhase::Gather
                    | VolleyPhase::Ready
                    | VolleyPhase::Chase
                    | VolleyPhase::Bump
            }
        )
    }

    /// 자기 자리에 다 섰는가. 판이 "전부 섰는가"를 묻는 자리다.
    pub(in crate::pet) fn volley_stood(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Volleyball {
                volley: VolleyPhase::Ready | VolleyPhase::Chase | VolleyPhase::Bump
            }
        )
    }

    /// 판에서 빠져나가는 한 길 — **공중이면 떨어지고 아니면 유휴로.** 판이
    /// 화면 세로 중앙이라 나가는 자리는 거의 항상 공중이다.
    fn leave_court(&mut self, now_ms: u64) {
        if self.air {
            self.enter(Behavior::Falling, now_ms);
        } else {
            self.enter_idle(now_ms);
        }
    }

    fn enter_volley(&mut self, phase: VolleyPhase, now_ms: u64) {
        match phase {
            VolleyPhase::Ready | VolleyPhase::Chase => {
                // **네트를 본다.** 등을 지고 서면 공이 오는 게 안 보인다.
                // 뛰는 동안에도 네트를 보는 것이 배구 자세다 — 진행 방향으로
                // 돌면 옆걸음이 아니라 도망가는 그림이 된다.
                self.facing = self.volley_face;
                self.vx = 0.0;
                self.vy = 0.0;
            }
            _ => {}
        }
        let hold = match phase {
            VolleyPhase::Bump => VOLLEY_BUMP_MS,
            VolleyPhase::Cheer | VolleyPhase::Sulk => VOLLEY_CHEER_MS,
            // 판이 몰아 주는 국면들. 이 값은 국면 길이가 아니라 **안전 상한**이다.
            _ => VOLLEY_MAX_MS,
        };
        self.enter(Behavior::Volleyball { volley: phase }, now_ms + hold);
    }
}

#[cfg(test)]
#[path = "volleyball_tests.rs"]
mod tests;
