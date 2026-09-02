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
    /// 지금 국면이 끝나는 시각. `Settling`의 뜸과 판 전체의 안전 상한에 쓴다.
    until_ms: u64,
}

impl Bowling {
    pub(super) fn new(pins: BTreeMap<PetId, f64>, lane: Bounds, now_ms: u64) -> Self {
        Bowling {
            phase: BoardPhase::Gathering,
            pins,
            struck: BTreeSet::new(),
            lane,
            ball: None,
            until_ms: now_ms + BOWLING_MAX_MS,
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
