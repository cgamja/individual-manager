//! 단체 야차 — 마리 하나가 링에서 거쳐 가는 국면.
//!
//! **다른 모션과 역할이 반대다.** 보통은 마리가 제 물리를 굴리지만, 야차의 난투는
//! "누구를 노리나"라는 마리 하나의 `step`으로 답할 수 없는 질문이라 **판이 좌표를
//! 갖고 마리는 받아 적는다** (`pet/yacha.rs`). 이 파일이 스스로 움직이는 것은
//! 모이기(`Gather`)뿐이다.
//!
//! **넉백은 여기에도 없다.** `yacha_hurt`가 좌표를 한 줄도 안 건드리는 것이
//! "서로 튕겨나가지 않는다"(R7)의 자리다 — 판이 주는 좌표는 **제 발로 간 결과**라
//! 다른 이야기다 (플랜의 "움직임의 두 종류" 절).

use super::super::*;

impl Pet {
    pub(in crate::pet) fn tick_yacha(
        &mut self,
        now_ms: u64,
        yacha: YachaPhase,
        bounds: Bounds,
        dt: f64,
    ) {
        match yacha {
            // 유일하게 스스로 움직이는 국면.
            YachaPhase::Gather => self.tick_yacha_gather(now_ms, bounds, dt),
            // 나머지는 전부 판이 몬다 — 여기서는 아무것도 안 한다.
            _ => {}
        }
    }

    /// 한가운데의 자기 자리로 **날아간다.** 볼링·발리볼의 모이기와 같은 꼴이다.
    fn tick_yacha_gather(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
        // 목적지는 매 틱 지금 경계로 다시 잡는다 — 판이 도는 중에 해상도가
        // 바뀌어도 못 닿는 자리에 갇히지 않는다 (발리볼과 같은 방어).
        let tx = self.target.0.clamp(bounds.left, bounds.right - PET_SIZE);
        let ty = self.target.1.clamp(bounds.top, bounds.floor_y);
        let (dx, dy) = (tx - self.x, ty - self.y);
        let dist = dx.hypot(dy);
        if dist <= ARRIVE_EPSILON || now_ms >= self.behavior_until_ms {
            self.x = tx;
            self.y = ty;
            self.enter_yacha(YachaPhase::Guard, now_ms);
            return;
        }
        let step = (YACHA_GATHER_SPEED * dt).min(dist);
        self.x += dx / dist * step;
        self.y += dy / dist * step;
    }

    /// 판이 열려 이 마리가 링으로 간다. 들려 있거나 이미 판에 있으면 거절한다.
    pub fn start_yacha(&mut self, now_ms: u64, spot: (f64, f64), face: Facing) -> bool {
        if matches!(
            self.behavior,
            Behavior::Dragged
                | Behavior::Bowling { .. }
                | Behavior::Volleyball { .. }
                | Behavior::Yacha { .. }
        ) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.vx = 0.0;
        self.vy = 0.0;
        self.target = spot;
        self.facing = face;
        self.enter_yacha(YachaPhase::Gather, now_ms);
        true
    }

    /// 판이 정한 자리와 자세를 받아 적는다. **난투 중 좌표의 유일한 출처다.**
    ///
    /// 국면이 안 바뀌었으면 `enter`를 다시 밟지 않는다 — 밟으면 CSS 애니메이션이
    /// 매 틱 되감겨 주먹이 영영 안 뻗는다.
    pub(in crate::pet) fn yacha_apply(
        &mut self,
        now_ms: u64,
        at: (f64, f64),
        phase: YachaPhase,
        face: Facing,
    ) {
        self.x = at.0;
        self.y = at.1;
        self.facing = face;
        if self.behavior != (Behavior::Yacha { yacha: phase }) {
            self.enter_yacha(phase, now_ms);
        }
    }

    /// 이번 라운드의 대표 타격으로 뽑혔다 — 웹뷰가 "퍽"을 한 발 낸다.
    pub(in crate::pet) fn yacha_thud(&mut self, down: bool) {
        self.punch_seq += 1;
        self.punch_down = down;
    }

    /// 최후의 1인 — 양 날개를 든다.
    pub(in crate::pet) fn yacha_win(&mut self, now_ms: u64) {
        self.enter_yacha(YachaPhase::Win, now_ms);
    }

    /// 벨트를 차고 세레모니.
    pub(in crate::pet) fn yacha_champ(&mut self, now_ms: u64) {
        self.enter_yacha(YachaPhase::Champ, now_ms);
    }

    /// 야차 국면에 있기는 한가 — **`Champ`까지 포함한다.** 판이 "이 마리가 아직
    /// 내 것인가"를 물을 때 쓴다.
    pub(in crate::pet) fn in_yacha(&self) -> bool {
        matches!(self.behavior, Behavior::Yacha { .. })
    }

    /// 판에 참여 중인가. **`Champ`은 뺀다** — 세레모니 중에 판이 "참여자가
    /// 없다"고 접으면 벨트 수여가 나오기 전에 미녀가 사라진다.
    pub(in crate::pet) fn is_yachaing(&self) -> bool {
        self.in_yacha()
            && !matches!(
                self.behavior,
                Behavior::Yacha {
                    yacha: YachaPhase::Champ
                }
            )
    }

    /// 한가운데에 도착해 섰는가. 전부 서야 난투가 시작된다.
    pub(in crate::pet) fn yacha_stood(&self) -> bool {
        self.in_yacha()
            && !matches!(
                self.behavior,
                Behavior::Yacha {
                    yacha: YachaPhase::Gather
                }
            )
    }

    /// 판이 끝났거나 이 마리가 빠진다.
    ///
    /// **자유낙하로 두면 안 된다.** 판이 화면 세로 중앙이라 떨어지는 높이가
    /// 세계의 절반이고, 그러면 착지 속도가 `SPLAT_MIN_IMPACT`(700)를 넘어
    /// **매 판마다 전원이 동시에 철푸덕한다.** 비치발리볼의 `leave_court`가
    /// 이미 밟은 함정이라 그 답을 그대로 쓴다 — 헤엄의 내려앉기를 재사용해
    /// 날개를 저어 내려온다.
    pub(in crate::pet) fn leave_ring(&mut self, now_ms: u64, bounds: Bounds) {
        if !self.air {
            self.enter_idle(now_ms);
            return;
        }
        self.vx = 0.0;
        self.vy = 0.0;
        self.target = (self.x, bounds.floor_y);
        self.swim_descending = true;
        let 남은 = (bounds.floor_y - self.y).max(0.0);
        let 예산 = ((남은 / SWIM_DESCENT_SPEED) * 2_000.0) as u64 + 1_000;
        self.enter(Behavior::Swim, now_ms + 예산);
    }

    fn enter_yacha(&mut self, phase: YachaPhase, now_ms: u64) {
        self.vx = 0.0;
        self.vy = 0.0;
        let hold = match phase {
            YachaPhase::Punch => YACHA_SWING_MS,
            YachaPhase::Hurt => YACHA_HURT_MS,
            YachaPhase::Win => YACHA_WIN_MS,
            // 판이 몰아 주는 국면들. 이 값은 국면 길이가 아니라 **안전 상한**이다.
            _ => YACHA_MAX_MS,
        };
        self.enter(Behavior::Yacha { yacha: phase }, now_ms + hold);
    }
}

#[cfg(test)]
#[path = "yacha_tests.rs"]
mod tests;
