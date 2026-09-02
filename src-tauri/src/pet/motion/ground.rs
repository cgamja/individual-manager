//! 지상 이동 — 걷기 · 방향 전환 · 슬라이딩 · 굴러떨어지기.
//!
//! 넷이 한 파일인 이유는 **벽에 닿았을 때를 공유하기 때문**이다. 걷다가도
//! 미끄러지다가도 벽에 닿고, 그 판정이 두 벌이 되면 한쪽만 고쳐지고 조용히
//! 갈라진다. `hit_wall` 한 곳이 그것을 막는다.
//!
//! # 굴러떨어지기
//!
//! 벽에 닿으면 늘 얌전히 돌아서던 것을 확률로 갈랐다 — 30%는 그대로 박고 뒤로
//! 나자빠진 뒤 일어난다. 새 동작을 하나 더 만드는 것보다 **이미 매일 일어나는
//! 사건**에 갈래를 주는 게 체감 밀도를 더 올린다.
//!
//! # 감속은 남은 시간 비율로 한다
//!
//! 슬라이딩과 굴러떨어지기가 같은 방식을 쓴다. 마찰 상수를 두면 정지 판정이
//! 따로 필요해지고 그게 틀리면 **영원히 미끄러지는 상태**가 생기는데, 이 방식은
//! 끝나는 순간 속도가 정확히 0이라 그 상태를 표현할 수 없다.
//!
//! 슬라이딩은 **길이를 고정하고 출발 속도를 뽑는다.** 길이를 뽑으면 CSS 길이를
//! 코어 상수와 맞출 수 없다. 속도 하한은 취향이 아니라 계산이다 — 가장 느린
//! 슬라이딩(264px)이 가장 긴 걷기(252px)보다 멀어야 한다.
//!
//! 유휴 뒤에는 슬라이딩이 안 나온다. 서 있다가 갑자기 배를 깔면 준비 동작이 없다.

use super::super::*;

impl Pet {
    pub(in crate::pet) fn tick_walk(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
            self.x += self.facing.sign() * WALK_SPEED * dt;
            self.after_ground_move(now_ms, bounds);
    }

    pub(in crate::pet) fn tick_turn(&mut self, now_ms: u64) {
            if now_ms >= self.behavior_until_ms {
                self.facing = self.facing.flipped();
                let until = now_ms + self.range(WALK_MS);
                self.enter(Behavior::Walk, until);
            }
    }

    pub(in crate::pet) fn tick_slide(&mut self, now_ms: u64, bounds: Bounds, dt: f64) {
            // 감속은 남은 시간 비율로 한다. 마찰 상수를 두면 정지 판정이 따로
            // 필요해지고 그게 틀리면 영원히 미끄러지는데, 이 방식은 끝나는
            // 순간 속도가 정확히 0이라 그 상태를 표현할 수 없다 (굴러떨어지기와 같다).
            let remaining =
                self.behavior_until_ms.saturating_sub(now_ms) as f64 / SLIDE_MS as f64;
            self.x += self.facing.sign() * self.slide_speed * remaining * dt;
            self.after_ground_move(now_ms, bounds);
    }

    pub(in crate::pet) fn tick_tumble(&mut self, now_ms: u64, dt: f64) {
            // 남은 시간에 비례해 감속한다. 마찰 상수를 두면 정지 판정이 따로
            // 필요해지고 그게 틀리면 영원히 미끄러지는데, 이 방식은 동작이
            // 끝나는 순간 속도가 정확히 0이라 그 상태를 표현할 수 없다.
            let remaining =
                self.behavior_until_ms.saturating_sub(now_ms) as f64 / TUMBLE_MS as f64;
            self.x += self.facing.sign() * TUMBLE_SPEED * remaining * dt;
            if now_ms >= self.behavior_until_ms {
                self.get_up(now_ms);
            }
    }

    /// 지상 이동 한 틱을 마무리한다 — 경계에 닿았으면 벽 반응, 시간이 다 됐으면 다음 동작.
    ///
    /// **걷기와 슬라이딩이 이 함수를 공유한다.** `hit_wall`만 나눠 쓰고 그 앞의
    /// 분기 사슬을 복사해 두면, 경계 처리를 고칠 때(F2의 화면 넘기가 그렇다)
    /// 한쪽만 고쳐지고 조용히 갈라진다. 실제로 복사해 뒀다가 리뷰에서 잡혔다.
    fn after_ground_move(&mut self, now_ms: u64, bounds: Bounds) {
        if bounds.right <= bounds.left {
            // 걸어다닐 폭이 없는 화면(펭귄보다 좁은 작업 영역)에서는 양쪽 경계가
            // 겹쳐 매 step마다 Turn으로 들어가 영원히 제자리에서 돈다.
            // 그럴 때는 회전을 건너뛰고 평소처럼 유휴로 넘어가게 둔다.
            self.x = bounds.left;
            if now_ms >= self.behavior_until_ms {
                self.pick_next(now_ms, bounds);
            }
        } else if self.x <= bounds.left {
            self.x = bounds.left;
            self.hit_wall(now_ms);
        } else if self.x >= bounds.right {
            self.x = bounds.right;
            self.hit_wall(now_ms);
        } else if now_ms >= self.behavior_until_ms {
            self.pick_next(now_ms, bounds);
        }
    }

    /// 벽에 닿았을 때 — 얌전히 돌아서거나, 그대로 박고 굴러 넘어진다.
    ///
    /// 좌우 경계가 이 함수를 **공유한다.** "벽에 닿았다"를 판정하는 코드가 두 곳이
    /// 되면 한쪽만 고쳐지고 조용히 갈라진다.
    pub(in crate::pet) fn hit_wall(&mut self, now_ms: u64) {
        if self.range((0, 99)) < TUMBLE_AT_WALL_PERCENT {
            // 반동으로 벽 반대쪽으로 굴러간다. 방향을 **여기서** 뒤집는 이유는
            // 진행 방향과 `facing`이 어긋나면 웹뷰가 회전을 반대로 그리기
            // 때문이다. `Turn`은 끝날 때 뒤집지만 최종 결과는 같다.
            self.facing = self.facing.flipped();
            self.enter(Behavior::Tumble, now_ms + TUMBLE_MS);
        } else {
            self.enter(Behavior::Turn, now_ms + TURN_MS);
        }
    }

    /// 미끄러지기 시작한다. **출발 속도를 여기서 한 번 뽑는다** — 길이는 고정이고
    /// 이 값이 거리를 정하므로, 매 틱 뽑으면 감속이 들쭉날쭉해진다.
    pub(in crate::pet) fn enter_slide(&mut self, now_ms: u64) {
        let (lo, hi) = SLIDE_SPEED;
        self.slide_speed = lo + self.fraction() * (hi - lo);
        self.enter(Behavior::Slide, now_ms + SLIDE_MS);
    }

    /// 사용자가 시켜서 미끄러진다 (설정 창의 "슬라이딩").
    ///
    /// **낚시와 달리 바닥에서만 먹는다.** 낚시는 허공에 앉는 게 더 웃겼지만
    /// 미끄러지는 것은 **바닥과 닿아야** 성립한다 — 공중에서 배를 깔면 그냥
    /// 헤엄이다. 들려 있을 때도 거절한다.
    ///
    /// 걸을 폭이 없는 화면에서는 미끄러질 자리도 없다 — 그 판정은 세계를 아는
    /// 쪽(`step`)이 하므로 여기서는 보지 않고, 진입해도 첫 틱에 정리된다.
    pub fn start_slide(&mut self, now_ms: u64) -> bool {
        // **이미 미끄러지는 중이면 거절한다.** 여기서 다시 진입하면 코어는 길이를
        // 늘리는데 웹뷰는 클래스가 그대로라 애니메이션을 되감지 않는다 — 누운
        // 그림이 끝나고도 펭귄이 선 채로 최대 2.4초를 더 미끄러진다.
        // `shouldRestart`가 "같은 한 번짜리 클래스가 연달아 오지 않는다"에 기대고
        // 있으므로, 그 전제를 깨지 않는 쪽을 고른다.
        if self.air || matches!(self.behavior, Behavior::Dragged | Behavior::Slide) {
            return false;
        }
        self.last_stimulus_ms = now_ms;
        self.enter_slide(now_ms);
        true
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
