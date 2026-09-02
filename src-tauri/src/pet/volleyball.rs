//! 비치발리볼 판 — 코트 기하, 팀, 공, 랠리 계획.
//!
//! **볼링과 정반대의 물건이다.** 볼링은 사용자가 공을 굴려서 재미의 책임을 물리가
//! 지지만, 여기서는 사용자가 버튼을 누른 뒤 **아무것도 안 한다.** 그래서 이 모듈의
//! 실패 모드는 "물리가 틀렸다"가 아니라 **"20초 동안 보고 있기 지루하다"**이고,
//! [`Volleyball::plan_hit`]이 그 답을 통째로 지고 있다.
//!
//! **판은 `Pets`가 소유한다. `Pet`이 아니다** — 볼링과 같은 근거다: 판이 도는 중에
//! 펭귄을 지웠을 때 그 자리를 비우는 일을 원자적으로 할 자리가 거기밖에 없다.
//!
//! **난수는 판이 갖는다.** 볼링은 난수를 하나도 안 썼지만(핀 자리는 id 순, 공은
//! 완전 결정적) 여기는 랠리가 난수로 만들어진다. `Pet::rng`를 태우면 **판에 참여한
//! 마리만 이후 동작 시퀀스가 밀리므로** 판이 자기 xorshift를 갖고, 시드는 밖에서
//! 받는다 — 그래야 같은 시드가 같은 랠리를 낳는다 (PRINCIPLE 3).

use std::collections::BTreeMap;

use serde::Serialize;

use super::*;

/// 네트의 어느 쪽인가.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub(super) fn other(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// 판 전체가 거쳐 가는 국면.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CourtPhase {
    /// 펭귄들이 화면 가운데의 자기 자리로 날아가는 중. **공은 아직 없다.**
    Gathering,
    /// 랠리 중. 서브도 여기 안에 있다 (서브는 국면이 아니라 왕복 0번이다).
    Rally,
    /// 공이 모래에 닿았다. 이긴 쪽은 좋아하고 진 쪽은 약 오른다.
    Point,
}

/// 코트 — 판이 열릴 때 경계에서 **한 번** 재고, 판이 도는 20초 동안은 이게 세계다.
/// 도중에 경계가 바뀌어도 펭귄과 공이 서로 다른 좌표계를 보지 않게 한다 (볼링의
/// `lane`과 같은 이유).
///
/// 좌표 관례가 둘 섞여 있어 이름으로 구분한다: `*_cx`는 **몸통 가운데 x**(공과
/// 거리를 잴 때 쓴다), [`spot_of`](Self::spot_of)가 돌려주는 것은 `Pet::x`와 같은
/// **좌상단**이다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Court {
    /// 네트의 세계 x (몸통 가운데 기준).
    net_cx: f64,
    /// 네트에서 코트 끝까지.
    half: f64,
    /// 네트에서 가장 가까운 자리까지.
    gap: f64,
    /// 펭귄이 서는 y (좌상단). 모래는 이보다 `PET_SIZE` 아래다.
    floor_y: f64,
    /// 올라갈 수 있는 최고점 — 공의 정점을 여기서 자른다.
    top: f64,
}

impl Court {
    /// 경계에서 코트를 낸다. **너무 좁으면 `None`** — 판을 아예 안 연다.
    /// 억지로 우겨넣으면 네 마리가 한 점에 겹쳐 서고 그건 판이 아니다.
    pub(super) fn new(bounds: Bounds) -> Option<Court> {
        let width = bounds.right - bounds.left;
        if width < VOLLEY_MIN_WORLD_WIDTH {
            return None;
        }
        // 코트가 화면보다 넓으면 화면에 맞춘다. 그때도 한쪽 폭이 남도록
        // 네트 여백을 함께 줄인다 — `gap`을 고정으로 두면 좁은 화면에서
        // `half <= gap`이 되어 설 자리가 사라진다.
        let half = VOLLEY_COURT_HALF.min(width / 2.0);
        let gap = VOLLEY_NET_GAP.min(half / 2.0);
        Some(Court {
            net_cx: (bounds.left + bounds.right) / 2.0 + PET_SIZE / 2.0,
            half,
            gap,
            floor_y: bounds.floor_y,
            top: bounds.top.min(bounds.floor_y),
        })
    }

    pub(super) fn net_cx(&self) -> f64 {
        self.net_cx
    }

    /// 모래 표면 — 펭귄 발밑이다 (`Pet::y`는 좌상단이므로 `+ PET_SIZE`).
    pub(super) fn sand_y(&self) -> f64 {
        self.floor_y + PET_SIZE
    }

    /// 네트 꼭대기.
    pub(super) fn net_top_y(&self) -> f64 {
        self.sand_y() - VOLLEY_NET_HEIGHT
    }

    /// 펭귄이 공을 치는 높이 (공 **중심**의 y). **네트 꼭대기보다 위다** —
    /// 그래서 타점에서 타점으로 가는 포물선은 네트에 걸릴 수가 없다 (KTD6).
    pub(super) fn contact_y(&self) -> f64 {
        self.floor_y - VOLLEY_REACH
    }

    /// 공의 정점이 넘으면 안 되는 선. 넘으면 화면 위로 사라진다.
    pub(super) fn ceiling_y(&self) -> f64 {
        self.top
    }

    /// 이 팀이 설 수 있는 **몸통 가운데 x** 범위.
    pub(super) fn span_of(&self, side: Side) -> (f64, f64) {
        match side {
            Side::Left => (self.net_cx - self.half, self.net_cx - self.gap),
            Side::Right => (self.net_cx + self.gap, self.net_cx + self.half),
        }
    }

    /// `n`마리 중 `k`번째가 설 자리 — 돌려주는 것은 **`Pet::x`/`Pet::y`**(좌상단)다.
    /// 한 마리면 자기 코트 가운데, 여럿이면 폭에 고르게 퍼진다.
    pub(super) fn spot_of(&self, side: Side, k: usize, n: usize) -> (f64, f64) {
        let (lo, hi) = self.span_of(side);
        let cx = if n <= 1 {
            (lo + hi) / 2.0
        } else {
            lo + (hi - lo) * (k.min(n - 1) as f64) / ((n - 1) as f64)
        };
        (cx - PET_SIZE / 2.0, self.floor_y)
    }

    /// 이 x가 네트의 어느 쪽인가. 정확히 네트 위면 왼쪽으로 본다 — 공이 네트
    /// 꼭대기에 정확히 서는 일은 물리적으로 없고, 갈래를 하나로 정해 둬야
    /// 같은 시드가 같은 결과를 낸다.
    pub(super) fn side_of_cx(&self, cx: f64) -> Side {
        if cx <= self.net_cx {
            Side::Left
        } else {
            Side::Right
        }
    }

    /// 코트 창이 덮을 사각형 — `(x, y, 폭, 높이)` 논리 좌표.
    /// 네트 꼭대기부터 모래 아래까지다.
    pub fn rect(&self) -> (f64, f64, f64, f64) {
        let 반폭 = self.half + PET_SIZE / 2.0;
        (
            self.net_cx - 반폭,
            self.net_top_y(),
            반폭 * 2.0,
            VOLLEY_NET_HEIGHT + VOLLEY_SAND_DEPTH,
        )
    }
}

/// 마릿수를 팀으로 나눈다 — **id 오름차순으로 번갈아.** 홀수면 왼쪽이 하나 많다.
///
/// 난수를 쓰지 않는다: 같은 마릿수가 항상 같은 배치를 낳아야 "펭귄을 하나 지웠더니
/// 남은 애들이 통째로 자리를 바꿨다"가 안 생긴다.
pub(super) fn assign_sides(count: usize) -> Vec<Side> {
    (0..count)
        .map(|i| if i % 2 == 0 { Side::Left } else { Side::Right })
        .collect()
}

/// 브릿지와 웹뷰가 보는 공.
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct VolleyBallSnapshot {
    /// 공 **중심**의 세계 좌표.
    pub x: f64,
    pub y: f64,
    /// 날아가는 중인가 — 웹뷰가 도는 그림을 그리는 데 쓴다.
    pub flying: bool,
}

/// 브릿지가 한 번에 받아 가는 판의 겉모습. 락을 한 번만 잡으려고 묶었다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VolleySnapshot {
    /// 코트 창이 덮을 사각형.
    pub court: (f64, f64, f64, f64),
    /// 지금 화면에 있어야 하는 공. 모이는 중에는 없다.
    pub ball: Option<VolleyBallSnapshot>,
}

#[cfg(test)]
#[path = "volleyball_tests.rs"]
mod tests;
