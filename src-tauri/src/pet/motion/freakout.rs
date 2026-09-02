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
                    // 안전판 — 목적지에 영영 못 닿아도 발작에 갇히지 않는다.
                    // **여기서 곧장 `Pant`로 가면 안 된다**: 공중이면 지상
                    // 동작이 되어 같은 step의 clamp가 y를 바닥으로 순간이동
                    // 시킨다. 상한을 미뤄 다음 틱부터 평소 경로로 내려온다.
                    // (`dt`는 `MAX_STEP_MS`로 잘리지만 이 비교는 벽시계라,
                    // 틱 스레드가 밀리면 거의 안 움직인 채 상한을 넘길 수 있다.)
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
                        self.facing = if dx > 0.0 { Facing::Right } else { Facing::Left };
                    }
                }
            }
            FreakoutPhase::Pant => {
                if now_ms >= self.behavior_until_ms {
                    // **`get_up`을 쓰지 않는다** — 70% 약올리기는 "아무 일
                    // 없었다는 듯"과 정반대다 (얼음낚시와 같은 판단이다).
                    self.enter_idle(now_ms);
                }
            }
        }
    }

    /// 발작 한 판을 시작한다.
    ///
    /// **첫 목적지를 여기서 뽑지 않는다.** 제자리를 목적지로 두면 첫 틱이 곧바로
    /// "도착"으로 판정해 목적지를 뽑는데, 그러면 이 함수가 `bounds`를 받을 필요가
    /// 없어져 `start_fishing`·`start_slide`와 시그니처가 같아진다.
    pub(in crate::pet) fn enter_freakout(&mut self, now_ms: u64) {
        self.freakout_until_ms = now_ms + self.range(FREAKOUT_MS);
        self.target = (self.x, self.y);
        // 이 시각은 **안전판이다.** 정상 경로는 `freakout_until_ms`로 끝나고,
        // 이 값은 어떤 이유로든 목적지에 영영 못 닿았을 때만 쓰인다
        // (헤엄이 상한을 두는 것과 같은 이유).
        let until = self.freakout_until_ms + FREAKOUT_MS.1;
        self.enter(Behavior::Freakout { freakout: FreakoutPhase::Dash }, until);
    }

    /// 다음으로 튈 곳. 방향은 균등, 거리는 [`FREAKOUT_HOP`]에서 뽑고 **경계 안으로
    /// 자른다** — 세계는 화면 하나이고 경계는 벽이다 (PRINCIPLE 2).
    fn next_freakout_target(&mut self, bounds: Bounds) -> (f64, f64) {
        let angle = self.fraction() * std::f64::consts::TAU;
        let (lo, hi) = FREAKOUT_HOP;
        let hop = lo + self.fraction() * (hi - lo);
        let tx = (self.x + angle.cos() * hop).clamp(bounds.left, bounds.right.max(bounds.left));
        let ty = (self.y + angle.sin() * hop)
            .clamp(bounds.top.min(bounds.floor_y), bounds.floor_y);
        (tx, ty)
    }

    /// 판을 접는다 — 바닥이면 숨을 고르고, 공중이면 바닥을 목적지로 돌린다.
    ///
    /// **바닥 복귀를 국면으로 만들지 않는다.** 공중에서 곧장 `Pant`로 가면
    /// `enter()`가 지상 동작으로 바꾸고 같은 step의 clamp가 y를 바닥으로
    /// **순간이동**시킨다. 목적지만 바닥으로 돌리면 같은 돌진으로 내려온다.
    ///
    /// 예산 만료와 안전판이 **이 한 곳을 공유한다** — 두 벌이 되면 한쪽만
    /// 고쳐지고 조용히 갈라진다.
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
            Behavior::Freakout { freakout: FreakoutPhase::Pant },
            now_ms + FREAKOUT_PANT_MS,
        );
    }

    /// 사용자가 시켜서 발작한다 (설정 창의 "발작").
    ///
    /// 저절로 나오는 발작은 바닥 전용이지만(`pick_next`가 지상에서만 부른다)
    /// 시켜서 하는 것은 공중도 허용한다 — 돌진이 어차피 공중 동작이다.
    /// 거절은 들려 있을 때와 **이미 발작 중일 때**뿐이다 (`start_squawk`와 같은
    /// 이유 — 재진입하면 웹뷰가 애니메이션을 되감지 못한다).
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
