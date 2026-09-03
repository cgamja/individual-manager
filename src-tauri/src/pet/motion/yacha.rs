//! 단체 야차 — 마리 하나가 링에서 거쳐 가는 국면.
//!
//! **이 파일에는 `self.x`를 피격으로 바꾸는 줄이 하나도 없다.** 서로 튕겨나가지
//! 않는 것이 이 동작의 정의이기 때문이다 (R7) — 휘청이는 몸짓은 CSS가 그리고
//! 창은 제자리에 있는다. 자리를 옮기는 것은 링으로 날아갈 때(`Gather`)와 이웃이
//! 쓰러져 자리를 다시 잡을 때(`Guard`)뿐이다.

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
            YachaPhase::Gather => self.tick_yacha_gather(now_ms, bounds, dt),
            // 이웃이 쓰러지면 판이 목표를 새로 준다 — 그쪽으로 미끄러져 붙는다.
            YachaPhase::Guard => self.tick_yacha_guard(dt),
            YachaPhase::Punch | YachaPhase::Hurt => {
                if now_ms >= self.behavior_until_ms {
                    self.enter_yacha(YachaPhase::Guard, now_ms);
                }
            }
            // 판이 몰아 주는 국면들. 스스로 나가지 않는다.
            YachaPhase::Down | YachaPhase::Win | YachaPhase::Champ => {}
        }
    }

    /// 링의 자기 자리로 **날아간다.** 볼링·발리볼의 모이기와 같은 꼴이다.
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

    /// 가드 자세. **피격으로는 안 움직인다** — 여기서 x가 변하는 유일한 이유는
    /// 이웃이 쓰러져 판이 자리를 다시 잡아 준 것이다.
    fn tick_yacha_guard(&mut self, dt: f64) {
        let dx = self.target.0 - self.x;
        if dx.abs() <= ARRIVE_EPSILON {
            self.x = self.target.0;
            return;
        }
        let step = (YACHA_CLOSE_SPEED * dt).min(dx.abs());
        self.x += dx.signum() * step;
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
        self.yacha_hits = 0;
        self.enter_yacha(YachaPhase::Gather, now_ms);
        true
    }

    /// 판이 자리를 다시 잡아 준다 (이웃이 쓰러졌을 때). 국면은 안 건드린다 —
    /// 새로 밟으면 CSS 애니메이션이 매번 되감긴다.
    pub(in crate::pet) fn yacha_restance(&mut self, spot: (f64, f64)) {
        self.target = spot;
        self.y = spot.1;
    }

    /// 펀치를 뻗는다. `thud`면 이번 라운드의 대표 타격이라 소리가 난다.
    pub(in crate::pet) fn yacha_punch(&mut self, now_ms: u64, toward: Facing) {
        self.facing = toward;
        self.enter_yacha(YachaPhase::Punch, now_ms);
    }

    /// 맞는다. **좌표를 안 건드린다** — 이 한 줄의 부재가 R7이다.
    pub(in crate::pet) fn yacha_hurt(&mut self, now_ms: u64) {
        self.yacha_hits += 1;
        self.enter_yacha(YachaPhase::Hurt, now_ms);
    }

    /// 이번 라운드의 대표 타격으로 뽑혔다 — 웹뷰가 "퍽"을 한 발 낸다.
    pub(in crate::pet) fn yacha_thud(&mut self, down: bool) {
        self.punch_seq += 1;
        self.punch_down = down;
    }

    /// 쓰러진다. 눈이 X자가 되고 판이 끝날 때까지 안 일어난다.
    pub(in crate::pet) fn yacha_down(&mut self, now_ms: u64) {
        self.enter_yacha(YachaPhase::Down, now_ms);
    }

    /// 최후의 1인 — 양 날개를 든다.
    pub(in crate::pet) fn yacha_win(&mut self, now_ms: u64) {
        self.enter_yacha(YachaPhase::Win, now_ms);
    }

    /// 벨트를 차고 세레모니.
    pub(in crate::pet) fn yacha_champ(&mut self, now_ms: u64) {
        self.enter_yacha(YachaPhase::Champ, now_ms);
    }

    /// 지금까지 맞은 횟수. 판이 "누가 쓰러지나"를 이걸로 정한다.
    pub(in crate::pet) fn yacha_hits(&self) -> u32 {
        self.yacha_hits
    }

    /// 야차 국면에 있기는 한가 — **`Champ`까지 포함한다.** 판이 "이 마리가
    /// 아직 내 것인가"를 물을 때 쓴다.
    pub(in crate::pet) fn in_yacha(&self) -> bool {
        matches!(self.behavior, Behavior::Yacha { .. })
    }

    /// 판에 참여 중인가. **`Champ`은 뺀다** — 세레모니 중에 판이 "참여자가
    /// 없다"고 접으면 축하 그림이 나오기 전에 링이 사라진다 (발리볼이
    /// `Cheer`/`Sulk`를 뺀 것과 같은 자리다).
    pub(in crate::pet) fn is_yachaing(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Yacha {
                yacha: YachaPhase::Gather
                    | YachaPhase::Guard
                    | YachaPhase::Punch
                    | YachaPhase::Hurt
                    | YachaPhase::Down
                    | YachaPhase::Win
            }
        )
    }

    /// 링에 도착해 섰는가. 전부 서야 난투가 시작된다.
    pub(in crate::pet) fn yacha_stood(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Yacha {
                yacha: YachaPhase::Guard | YachaPhase::Punch | YachaPhase::Hurt
            }
        )
    }

    /// 쓰러져 있는가.
    #[cfg(test)]
    pub(in crate::pet) fn yacha_is_down(&self) -> bool {
        matches!(
            self.behavior,
            Behavior::Yacha {
                yacha: YachaPhase::Down
            }
        )
    }

    /// 판이 끝났거나 이 마리가 빠진다.
    ///
    /// **자유낙하로 두면 안 된다.** 링이 화면 세로 중앙이라 떨어지는 높이가
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
        if matches!(phase, YachaPhase::Guard) {
            self.vx = 0.0;
            self.vy = 0.0;
        }
        let hold = match phase {
            YachaPhase::Punch => YACHA_PUNCH_MS,
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
