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

use std::collections::{BTreeMap, BTreeSet};

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
    /// 참여 마리 → 핀 자리(펭귄 `x`). **id 오름차순으로 오른쪽부터** 배정하므로
    /// 같은 마릿수는 항상 같은 배치를 낳는다 (R12).
    pins: BTreeMap<PetId, f64>,
    /// 이미 맞은 마리. 같은 펭귄을 두 번 맞히지 않는다.
    struck: BTreeSet<PetId>,
    /// 판이 열릴 때 잰 레인. 판이 도는 몇 초 동안은 이게 세계다 — 도중에 경계가
    /// 바뀌어도 핀과 공이 서로 다른 좌표계를 보지 않게 한다.
    lane: Bounds,
    ball: Option<Ball>,
    /// `Settling`의 뜸이 끝나는 시각.
    until_ms: u64,
    /// 판이 마지막으로 진행된 시각. 공은 마리와 따로 도므로 자기 시계를 갖는다.
    last_step_ms: u64,
}

impl Bowling {
    pub(super) fn new(pins: BTreeMap<PetId, f64>, lane: Bounds, now_ms: u64) -> Self {
        Bowling {
            phase: BoardPhase::Gathering,
            pins,
            struck: BTreeSet::new(),
            lane,
            ball: None,
            until_ms: 0,
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
    pub fn pin_of(&self, id: PetId) -> Option<f64> {
        self.pins.get(&id).copied()
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
        self.struck.remove(&id);
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
        }
    }

    /// 공을 놓는다. **가로 속도만 쓴다** (R6) — 세로는 버린다. 문턱보다 살살
    /// 놓으면 굴러가지 않고 그 자리에 남아, 사용자가 다시 집을 수 있다.
    pub(super) fn release(&mut self, now_ms: u64, vx: f64) {
        let vx = clamp_roll(vx, self.lane_width());
        let Some(ball) = self.ball.as_mut() else {
            return;
        };
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

    /// 이 마리가 지금 공에 맞았는가. 맞았으면 표시하고 공의 속도를 깎는다.
    /// **같은 마리를 두 번 맞히지 않고, 맞아도 공은 멈추지 않는다** (A2).
    pub(super) fn hit(&mut self, id: PetId, pet_center_x: f64) -> bool {
        if self.struck.contains(&id) {
            return false;
        }
        let Some(ball) = self.ball.as_mut() else {
            return false;
        };
        if (ball.x - pet_center_x).abs() > BOWLING_HIT_RADIUS {
            return false;
        }
        ball.vx *= 1.0 - BOWLING_SPEED_LOSS_PER_PIN;
        self.struck.insert(id);
        true
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

    /// 이 판의 세계 폭. 속도 상한과 감속이 여기에 비례한다 — 판이 도는 동안은
    /// 레인이 세계이므로 바깥 `World::width()`를 다시 묻지 않는다.
    fn lane_width(&self) -> f64 {
        let w = self.lane.right - self.lane.left;
        if w > 0.0 {
            w
        } else {
            FALLBACK_WORLD_WIDTH
        }
    }
}

/// 굴리기 속도를 세계 폭이 정한 상한으로 자른다 — 던지기(`clamp_throw`)와
/// 같은 관례다. 화면이 넓어지면 같은 손짓이 더 멀리 간다 (R5).
pub(super) fn clamp_roll(vx: f64, world_width: f64) -> f64 {
    let width = if world_width > 0.0 {
        world_width
    } else {
        FALLBACK_WORLD_WIDTH
    };
    let max = (width * BOWLING_MAX_WORLDS_PER_SEC).max(THROW_MIN_SPEED);
    vx.clamp(-max, max)
}

/// 핀 자리들 — 오른쪽 끝에서 왼쪽으로 `count`개. 반환 순서가 곧 id 오름차순의
/// 배정 순서라, 같은 마릿수면 항상 같은 배치가 나온다 (R12).
///
/// **좁은 화면 방어**: 계산한 가장 왼쪽 핀이 공이 굴러올 길([`BOWLING_LANE_MIN`])을
/// 침범하면 간격을 줄여 다시 배분한다 (A5). 레인이 아예 없는 경우(폭 0)에도
/// 패닉하지 않고 전부 같은 자리에 겹쳐 선다.
pub(super) fn pin_positions(count: usize, lane: Bounds) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    let right = lane.right.max(lane.left);
    let first = (right - BOWLING_PIN_MARGIN).max(lane.left);
    let lane_left = (lane.left + BOWLING_LANE_MIN).min(first);
    let gap = if count > 1 {
        BOWLING_PIN_GAP.min((first - lane_left).max(0.0) / (count - 1) as f64)
    } else {
        BOWLING_PIN_GAP
    };
    (0..count)
        .map(|i| (first - gap * i as f64).clamp(lane.left, right))
        .collect()
}

/// 공이 처음 놓이는 자리 — 레인 **왼쪽 바닥**. 반환은 공 **중심**이다.
///
/// 세로는 펭귄 발밑과 같은 선에 맞춘다. 펭귄은 `floor_y`에 **왼쪽 위 모서리**가
/// 놓이므로 발밑은 `floor_y + PET_SIZE`이고, 공은 그 선에 밑면이 닿아야 한다.
pub(super) fn ball_home(lane: Bounds) -> (f64, f64) {
    (
        lane.left + BOWLING_BALL_SIZE / 2.0,
        lane.floor_y + PET_SIZE - BOWLING_BALL_SIZE / 2.0,
    )
}

#[cfg(test)]
#[path = "bowling_tests.rs"]
mod tests;
