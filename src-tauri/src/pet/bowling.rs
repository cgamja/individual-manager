//! 볼링 판 — 판 전체의 국면, 핀 자리, 그리고 공.
//!
//! **판은 `Pets`가 소유한다. `Pet`이 아니다** (KTD2). 공과 핀 자리 배정표를 `Pet`
//! 안에 넣으면 "어느 마리가 공을 소유하나"라는 답 없는 질문이 생기고, 무엇보다
//! 판이 도는 중에 펭귄을 지웠을 때 그 자리를 비우는 일을 원자적으로 할 자리가
//! 사라진다 (R11/AE4).
//!
//! **국면이 두 층이다.** 여기 [`BoardPhase`]는 판 전체(모으는 중 → 공 대기 →
//! 굴러가는 중 → 정리)이고, 마리별 국면은 `motion/bowling.rs`의
//! [`BowlingPhase`](super::BowlingPhase)다. 나눈 이유는 "전부 섰는가"를 물어볼
//! 자리가 필요해서다 — **판이 마리를 몰지 그 반대가 아니다** (KTD8).

use std::collections::BTreeMap;

use serde::Serialize;

use super::*;

/// 볼링 판 전체가 거쳐 가는 국면.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoardPhase {
    /// 펭귄들이 자기 자리로 걸어가는 중. **공은 아직 없다** (R4).
    Gathering,
    /// 전부 섰다. 공이 놓이고 사용자를 기다린다.
    Ready,
    /// 공이 굴러가는 중.
    Rolling,
    /// 공이 멎었다. 뜸을 들이고 흩어진다.
    Settling,
}

/// 공. **수평으로만 움직인다** (R6) — 조준 각도가 없다.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Ball {
    /// 공 **중심**의 세계 좌표. 펭귄의 `x`가 왼쪽 위 모서리인 것과 다르다 —
    /// 히트 판정이 중심끼리의 거리라 중심으로 들고 있는 편이 헷갈리지 않는다.
    x: f64,
    /// 직전 틱의 x. **히트 판정은 점이 아니라 이번 틱에 지나온 구간으로 한다** —
    /// 틱이 밀리면(`MAX_STEP_MS` 250ms) 한 틱에 260px을 지나므로, 지금 위치만
    /// 보면 히트 반경(52px)보다 좁은 핀을 통째로 뛰어넘는다.
    prev_x: f64,
    y: f64,
    vx: f64,
    /// 사용자가 집고 있는가. 집고 있는 동안에는 물리가 돌지 않는다.
    held: bool,
}

/// 브릿지와 웹뷰가 보는 공. 창을 어디에 놓을지와 무엇을 그릴지를 정한다.
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct BallSnapshot {
    /// 공 중심의 세계 좌표.
    pub x: f64,
    pub y: f64,
    /// 굴러가는 중인가 — 웹뷰가 구르는 그림을 그리는 데 쓴다.
    pub rolling: bool,
    /// 사용자가 집고 있는가.
    pub held: bool,
}

/// 한 판.
pub struct Bowling {
    phase: BoardPhase,
    /// 아직 서 있는 마리 → 핀 자리(펭귄 `x`, `y`). **id 오름차순으로 꼭짓점부터**
    /// 배정하므로 같은 마릿수는 항상 같은 배치를 낳는다 (R12).
    pins: BTreeMap<PetId, (f64, f64)>,
    /// 튕겨 나간 마리 → **직전 틱의 몸통 가운데.** 연쇄가 여기서 산다 —
    /// 날아가는 동안 아직 선 핀을 친다. 착지하면 빠진다.
    ///
    /// 위치를 들고 있는 이유는 판정을 **지나온 구간**으로 하기 위해서다.
    /// 튕기는 속도가 세계 폭에 비례하므로 넓은 화면에서는 한 틱에 140px 넘게
    /// 날아, 지금 위치만 보면 이웃을 통째로 뛰어넘는다 (공 판정과 같은 함정).
    knocked: BTreeMap<PetId, (f64, f64)>,
    /// 판이 열릴 때 잰 레인. 판이 도는 몇 초 동안은 이게 세계다 — 도중에 경계가
    /// 바뀌어도 핀과 공이 서로 다른 좌표계를 보지 않게 한다.
    lane: Bounds,
    ball: Option<Ball>,
    /// `Settling`의 뜸이 끝나는 시각.
    until_ms: u64,
    /// **판 자체의 마감 시각.** 마리마다 안전 상한이 있어도 그것만으로는
    /// 부족하다 — 마리들이 서로 다른 시각에 만료되면 마지막 한 마리가 빠질
    /// 때까지 판이(그리고 공 창이) 남는다. 판을 통째로 끊는 시계가 따로 있어야
    /// 방치된 한 판이 화면에 2분씩 놓여 있지 않는다.
    deadline_ms: u64,
    /// 판이 마지막으로 진행된 시각. 공은 마리와 따로 도므로 자기 시계를 갖는다.
    last_step_ms: u64,
}

impl Bowling {
    pub(super) fn new(pins: BTreeMap<PetId, (f64, f64)>, lane: Bounds, now_ms: u64) -> Self {
        Bowling {
            phase: BoardPhase::Gathering,
            pins,
            knocked: BTreeMap::new(),
            lane,
            ball: None,
            until_ms: 0,
            deadline_ms: now_ms + BOWLING_MAX_MS,
            last_step_ms: now_ms,
        }
    }

    pub fn phase(&self) -> BoardPhase {
        self.phase
    }

    /// 지금 판에 서 있는 마리들. id 오름차순이다.
    pub fn participants(&self) -> Vec<PetId> {
        self.pins.keys().copied().collect()
    }

    /// 이 마리의 핀 자리. 참여하지 않으면 `None`.
    pub fn pin_of(&self, id: PetId) -> Option<(f64, f64)> {
        self.pins.get(&id).copied()
    }

    /// 지금 날아가는 중인 마리들과 **직전 틱의 위치** — 이들이 아직 선 핀을 친다.
    pub(super) fn knocked(&self) -> Vec<(PetId, (f64, f64))> {
        self.knocked.iter().map(|(id, at)| (*id, *at)).collect()
    }

    /// 한 마리가 맞아 판에서 튕겨 나간다. `at`은 지금 몸통 가운데다.
    pub(super) fn knock(&mut self, id: PetId, at: (f64, f64)) {
        self.pins.remove(&id);
        self.knocked.insert(id, at);
    }

    /// 날아가는 마리의 위치를 이번 틱 값으로 갱신한다.
    pub(super) fn track_knocked(&mut self, id: PetId, at: (f64, f64)) {
        if let Some(slot) = self.knocked.get_mut(&id) {
            *slot = at;
        }
    }

    /// 착지해서 더는 아무것도 못 치는 마리를 연쇄 목록에서 뺀다.
    pub(super) fn retain_knocked(&mut self, mut flying: impl FnMut(PetId) -> bool) {
        self.knocked.retain(|id, _| flying(*id));
    }

    /// 지금 화면에 있어야 하는 공. 모으는 중에는 없다 (R4).
    pub fn ball(&self) -> Option<BallSnapshot> {
        self.ball.map(|b| BallSnapshot {
            x: b.x,
            y: b.y,
            rolling: self.phase == BoardPhase::Rolling,
            held: b.held,
        })
    }

    /// 참여 마리가 하나도 안 남았는가. 그러면 판을 접는다 (R11).
    pub(super) fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }

    /// 한 마리가 판에서 빠진다 — 지워졌거나(AE4), 드래그로 끌려 나갔거나(A4),
    /// 맞아서 다른 동작으로 넘어갔거나.
    pub(super) fn leave(&mut self, id: PetId) {
        self.pins.remove(&id);
        self.knocked.remove(&id);
    }

    /// 이번 틱의 경과 시간(초). 틱이 밀려도 공이 순간이동하지 않게
    /// `Pet::step`과 같은 상한을 쓴다.
    pub(super) fn tick(&mut self, now_ms: u64) -> f64 {
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        self.last_step_ms = now_ms;
        elapsed as f64 / 1000.0
    }

    /// 전부 섰다 — 왼쪽 바닥에 공을 놓고 사용자를 기다린다 (R4).
    pub(super) fn open_ball(&mut self) {
        let (x, y) = ball_home(self.lane);
        self.ball = Some(Ball {
            x,
            prev_x: x,
            y,
            vx: 0.0,
            held: false,
        });
        self.phase = BoardPhase::Ready;
    }

    /// 공을 집는다. 굴러가는 중이면 손이 안 닿는다 — 한 판에 한 번 굴린다.
    pub(super) fn grab(&mut self) -> bool {
        if self.phase != BoardPhase::Ready {
            return false;
        }
        let Some(ball) = self.ball.as_mut() else {
            return false;
        };
        ball.held = true;
        ball.vx = 0.0;
        true
    }

    /// 집은 공을 옮긴다. **레인 안에 붙들어 둔다** — 화면 밖으로 끌고 나가면
    /// 다시 집을 방법이 없다.
    pub(super) fn drag(&mut self, dx: f64) {
        let lo = self.lane.left + BOWLING_BALL_SIZE / 2.0;
        let hi = (self.lane.right + PET_SIZE - BOWLING_BALL_SIZE / 2.0).max(lo);
        let Some(ball) = self.ball.as_mut() else {
            return;
        };
        if ball.held {
            ball.x = (ball.x + dx).clamp(lo, hi);
            // 손으로 옮기는 것은 굴러간 게 아니다 — 스윕 구간을 남기면 끌고
            // 지나간 핀이 놓는 순간 한꺼번에 맞는다.
            ball.prev_x = ball.x;
        }
    }

    /// 공을 놓는다. **가로 속도만 쓴다** (R6) — 세로는 버린다. 문턱보다 살살
    /// 놓으면 굴러가지 않고 그 자리에 남아, 사용자가 다시 집을 수 있다.
    pub(super) fn release(&mut self, now_ms: u64, vx: f64) {
        // **집고 있던 공만 놓을 수 있다.** 웹뷰가 보내는 것을 그대로 믿으면
        // 안 된다: 빠르게 튕기면 `pointerup`이 `ball_drag_start`의 왕복보다
        // 먼저 도착해, 집기가 거절됐는데도 놓기가 온다. 그때 그대로 반영하면
        // **굴러가던 공의 속도를 도중에 덮어써** 판이 그 자리에서 끝난다.
        if self.phase != BoardPhase::Ready {
            return;
        }
        let vx = clamp_roll(vx, self.lane_width());
        let Some(ball) = self.ball.as_mut() else {
            return;
        };
        if !ball.held {
            return;
        }
        ball.held = false;
        if vx.abs() < BOWLING_MIN_ROLL_SPEED {
            ball.vx = 0.0;
            return;
        }
        ball.vx = vx;
        self.phase = BoardPhase::Rolling;
        self.last_step_ms = now_ms;
    }

    /// 공을 한 틱 굴린다. **감속도로 줄인다** — 매 틱 비율로 줄이면 속도가 0에
    /// 닿지 않아 판이 영영 안 끝난다.
    pub(super) fn roll(&mut self, dt: f64) {
        let 감속 = self.lane_width() * BOWLING_DECEL_WORLDS_PER_SEC2 * dt;
        let Some(ball) = self.ball.as_mut() else {
            return;
        };
        ball.prev_x = ball.x;
        ball.x += ball.vx * dt;
        ball.vx = if ball.vx.abs() <= 감속 {
            0.0
        } else {
            ball.vx - ball.vx.signum() * 감속
        };
        if ball.vx.abs() < BOWLING_STOP_SPEED {
            ball.vx = 0.0;
        }
    }

    /// 이 마리가 지금 공에 맞았는가. 맞았으면 공의 속도를 깎고 **튕겨 나갈
    /// 방향**을 준다. 맞아도 공은 멈추지 않는다 (A2).
    ///
    /// 판이 화면 중앙의 평면에 서므로 판정도 2차원이다 — 공이 지나는 줄에서
    /// 멀리 떨어진 핀은 공에 직접 맞지 않고, **연쇄로만** 쓰러진다.
    pub(super) fn ball_hit(
        &mut self,
        pet_center_x: f64,
        pet_center_y: f64,
    ) -> Option<(f64, f64)> {
        let ball = self.ball.as_mut()?;
        // 세로는 점으로, 가로는 **이번 틱에 지나온 구간**으로 잰다. 지금 위치만
        // 보면 틱이 밀렸을 때 핀을 통째로 뛰어넘는다.
        if (ball.y - pet_center_y).abs() > BOWLING_HIT_RADIUS {
            return None;
        }
        let (lo, hi) = (ball.prev_x.min(ball.x), ball.prev_x.max(ball.x));
        if pet_center_x < lo - BOWLING_HIT_RADIUS || pet_center_x > hi + BOWLING_HIT_RADIUS {
            return None;
        }
        ball.vx *= 1.0 - BOWLING_SPEED_LOSS_PER_PIN;
        Some((pet_center_x - ball.x, pet_center_y - ball.y))
    }

    /// 공이 멎었거나 레인을 벗어났는가 — 판이 끝나는 조건이다. 시간 상한을
    /// 따로 두지 않는 이유는 공이 감속하므로 **반드시** 멎기 때문이다 (A1).
    pub(super) fn ball_done(&self) -> bool {
        let Some(ball) = self.ball else {
            return true;
        };
        ball.vx == 0.0
            || ball.x + BOWLING_BALL_SIZE / 2.0 < self.lane.left
            || ball.x - BOWLING_BALL_SIZE / 2.0 > self.lane.right + PET_SIZE
    }

    /// 공이 멎었다 — 뜸을 들이고 흩어진다.
    pub(super) fn settle(&mut self, now_ms: u64) {
        self.phase = BoardPhase::Settling;
        self.until_ms = now_ms + BOWLING_SETTLE_MS;
        if let Some(ball) = self.ball.as_mut() {
            ball.vx = 0.0;
        }
    }

    /// 뜸이 다 됐는가.
    pub(super) fn settled(&self, now_ms: u64) -> bool {
        now_ms >= self.until_ms
    }

    /// 판이 통째로 시간을 다 썼는가. 아무도 공을 굴리지 않고 자리를 뜬 판이
    /// 화면에 영원히 놓여 있지 않게 하는 마지막 장치다 (R11).
    pub(super) fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    /// 이 판의 세계 폭. 속도 상한과 감속과 튕겨 나가는 세기가 여기에 비례한다 —
    /// 판이 도는 동안은 레인이 세계이므로 바깥 `World::width()`를 다시 묻지 않는다.
    pub(super) fn lane_width(&self) -> f64 {
        let w = self.lane.right - self.lane.left;
        if w > 0.0 {
            w
        } else {
            FALLBACK_WORLD_WIDTH
        }
    }
}

/// 삼각 대형의 핀 자리들 — **꼭짓점이 왼쪽**(공이 오는 쪽)을 향한다.
///
/// 줄 `r`에는 `r+1`자리가 있고(1, 2, 3, …), 앞줄부터 채우다 남은 마지막 줄은
/// 가운데로 모은다. 반환 순서가 곧 id 오름차순의 배정 순서라, 같은 마릿수면
/// 항상 같은 배치가 나온다 (R12).
///
/// **바닥이 아니라 화면 세로 중앙에 선다** — 2차원 바닥은 선이라 삼각형을
/// 만들 수 없었다(그래서 처음에는 한 줄이었다). 판을 공중의 평면으로 옮기면
/// 그 제약이 사라진다 (2026-09-02 사용자 지시).
///
/// **좁은 화면 방어**: 대형이 공이 굴러올 길([`BOWLING_LANE_MIN`])을 침범하거나
/// 위아래로 넘치면 간격을 줄여 다시 배분한다. 영역이 아예 없어도 패닉하지 않는다.
pub(super) fn pin_positions(count: usize, lane: Bounds) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    let rows = triangle_rows(count);
    let widest = rows.iter().copied().max().unwrap_or(1);

    let right = lane.right.max(lane.left);
    let back = (right - BOWLING_PIN_MARGIN).max(lane.left);
    let lane_left = (lane.left + BOWLING_LANE_MIN).min(back);
    let row_gap = if rows.len() > 1 {
        BOWLING_ROW_GAP.min((back - lane_left).max(0.0) / (rows.len() - 1) as f64)
    } else {
        BOWLING_ROW_GAP
    };

    let center_y = lane_center_y(lane);
    let half_height = ((lane.floor_y - lane.top) / 2.0).max(0.0);
    let col_gap = if widest > 1 {
        BOWLING_COL_GAP.min(2.0 * half_height / (widest - 1) as f64)
    } else {
        BOWLING_COL_GAP
    };

    let mut out = Vec::with_capacity(count);
    for (r, &in_row) in rows.iter().enumerate() {
        // 뒷줄일수록 오른쪽이다. 꼭짓점(r=0)이 가장 왼쪽.
        let x = back - row_gap * (rows.len() - 1 - r) as f64;
        for k in 0..in_row {
            let y = center_y + (k as f64 - (in_row - 1) as f64 / 2.0) * col_gap;
            out.push((
                x.clamp(lane.left, right),
                y.clamp(lane.top.min(lane.floor_y), lane.floor_y),
            ));
        }
    }
    out
}

/// 점 `p`에서 선분 `a`–`b`까지의 **거리 제곱과 가장 가까운 점.**
///
/// 연쇄 판정이 점이 아니라 "이번 틱에 지나온 구간"이라 필요하다. 제곱으로
/// 돌려주는 이유는 비교만 하면 되기 때문이다 — `sqrt`를 매 쌍마다 부르지 않는다.
pub(super) fn dist2_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, (f64, f64)) {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    // 제자리에 있었으면 선분이 아니라 점이다.
    let t = if len2 <= f64::EPSILON {
        0.0
    } else {
        (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0)
    };
    let near = (a.0 + abx * t, a.1 + aby * t);
    let (dx, dy) = (p.0 - near.0, p.1 - near.1);
    (dx * dx + dy * dy, near)
}

/// `count`마리를 삼각형으로 세울 때 각 줄에 몇 마리가 서는가.
/// 앞줄(꼭짓점)부터 1, 2, 3, …으로 채우고 마지막 줄만 모자랄 수 있다.
fn triangle_rows(count: usize) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut left = count;
    let mut width = 1;
    while left > 0 {
        let take = left.min(width);
        rows.push(take);
        left -= take;
        width += 1;
    }
    rows
}

/// 판이 서는 높이 — 펭귄 `y`(왼쪽 위 모서리) 기준의 화면 세로 중앙.
pub(super) fn lane_center_y(lane: Bounds) -> f64 {
    (lane.top.min(lane.floor_y) + lane.floor_y) / 2.0
}

/// 공이 처음 놓이는 자리 — 레인 **왼쪽 끝, 판과 같은 높이**. 반환은 공 **중심**이다.
///
/// 세로는 핀의 몸통 가운데에 맞춘다. 펭귄은 `y`에 **왼쪽 위 모서리**가 놓이므로
/// 몸통 가운데는 `y + PET_SIZE / 2`이고, 공 중심이 그 선에 있어야 가운데 줄을
/// 정면으로 맞힌다.
pub(super) fn ball_home(lane: Bounds) -> (f64, f64) {
    (
        lane.left + BOWLING_BALL_SIZE / 2.0,
        lane_center_y(lane) + PET_SIZE / 2.0,
    )
}

/// 굴리기 속도를 세계 폭이 정한 상한으로 자른다 — 던지기(`clamp_throw`)와
/// 같은 관례다. 화면이 넓어지면 같은 손짓이 더 멀리 간다 (R5).
pub(super) fn clamp_roll(vx: f64, world_width: f64) -> f64 {
    let width = if world_width > 0.0 {
        world_width
    } else {
        FALLBACK_WORLD_WIDTH
    };
    let max = (width * BOWLING_MAX_WORLDS_PER_SEC).max(BOWLING_MIN_MAX_SPEED);
    vx.clamp(-max, max)
}

#[cfg(test)]
#[path = "bowling_tests.rs"]
mod tests;
