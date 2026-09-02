//! 비치발리볼 판 — 코트 기하, 팀, 공, 랠리 계획.
//!
//! **볼링과 정반대의 물건이다.** 볼링은 사용자가 공을 굴려서 재미의 책임을 물리가
//! 지지만, 여기서는 사용자가 버튼을 누른 뒤 **아무것도 안 한다.** 그래서 이 모듈의
//! 실패 모드는 "물리가 틀렸다"가 아니라 **"20초 동안 보고 있기 지루하다"**이고,
//! [`Volleyball::hit`]이 그 답을 통째로 지고 있다.
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
        let height = bounds.floor_y - bounds.top;
        // **세로도 본다.** 공을 띄울 높이가 없으면 체공이 0에 눌려 공이
        // 순간이동하고(`flight_ms_for`의 천장 자르기), 그건 판이 아니다.
        if width < VOLLEY_MIN_WORLD_WIDTH || height < VOLLEY_MIN_WORLD_HEIGHT {
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

    #[cfg(test)]
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

    /// 코트 밖으로 나갔는가 — 목적지가 늘 코트 안이라 일어나지 않지만,
    /// 일어나면 공이 영영 안 떨어지므로 방어로 둔다.
    pub(super) fn out_of(&self, cx: f64) -> bool {
        let 반폭 = self.half + PET_SIZE;
        cx < self.net_cx - 반폭 || cx > self.net_cx + 반폭
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

/// 공. **완전히 탄도다** — 중력 하나로 날고, 속도는 칠 때만 바뀐다.
#[derive(Clone, Copy, PartialEq, Debug)]
struct VolleyBall {
    /// 공 **중심**의 세계 좌표.
    x: f64,
    y: f64,
    /// 직전 틱의 y. **타점 판정은 점이 아니라 이번 틱에 지나온 구간으로 한다** —
    /// 틱이 밀리면(`MAX_STEP_MS` 250ms) 한 틱에 300px을 내려와, 지금 위치만 보면
    /// 타점을 통째로 뛰어넘는다 (볼링 공 판정과 똑같은 함정이다).
    prev_y: f64,
    vx: f64,
    vy: f64,
}

/// 한 판.
pub struct Volleyball {
    phase: CourtPhase,
    court: Court,
    /// 참여 마리 → (팀, 자기 자리). id 오름차순이다.
    players: BTreeMap<PetId, (Side, (f64, f64))>,
    ball: Option<VolleyBall>,
    /// 지금 공이 향하는 팀. `receiver`가 비어도(그 팀이 통째로 빠져도) 남는다.
    to_side: Side,
    /// 이번 왕복에서 받을 마리와 목적지(몸통 가운데 x). 없으면 아무도 안 뛴다.
    receiver: Option<(PetId, f64)>,
    /// 방금 계획한 왕복의 체공 (ms). 테스트가 리듬을 확인하는 데 쓴다.
    last_flight_ms: u64,
    /// **예산이 다 됐다 — 이후 접촉 판정을 통째로 건너뛴다.**
    ///
    /// 이게 종료의 증명이다. 여덟 마리가 코트를 빽빽하게 덮고 있어도 공은
    /// 반드시 모래에 닿는다. 받으러 뛰는 것은 막지 않으므로 화면에는
    /// "못 미치는 리시브"가 그대로 보인다.
    rally_over: bool,
    /// 랠리 예산이 끝나는 시각. 이 뒤의 첫 왕복이 킬샷이다.
    rally_until_ms: u64,
    /// `Point`의 뜸이 끝나는 시각.
    until_ms: u64,
    /// **판 자체의 마감 시각.** 마리마다 안전 상한이 있어도 서로 다른 시각에
    /// 만료되면 마지막 한 마리가 빠질 때까지 코트가 남는다 (볼링과 같은 근거).
    deadline_ms: u64,
    last_step_ms: u64,
    /// **판이 갖는 난수.** `Pet::rng`를 태우면 판에 참여한 마리만 이후 동작
    /// 시퀀스가 밀린다 — 같은 시드가 같은 랠리를 낳게 하려면 여기 있어야 한다.
    rng: u64,
}

impl Volleyball {
    pub(super) fn new(
        players: BTreeMap<PetId, (Side, (f64, f64))>,
        court: Court,
        now_ms: u64,
        seed: u64,
    ) -> Self {
        let mut board = Volleyball {
            phase: CourtPhase::Gathering,
            court,
            players,
            ball: None,
            to_side: Side::Left,
            receiver: None,
            last_flight_ms: 0,
            rally_over: false,
            rally_until_ms: 0,
            until_ms: 0,
            deadline_ms: now_ms + VOLLEY_MAX_MS,
            last_step_ms: now_ms,
            // 시드가 0이면 xorshift가 0에 갇힌다 (`Pet::new_at`과 같은 방어).
            rng: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        };
        // **예산은 판이 열릴 때 정한다.** 모이는 시간도 20초에 포함된다 —
        // "한 판이 얼마나 걸리나"는 사용자가 버튼을 누른 순간부터 세는 것이다.
        board.rally_until_ms = now_ms + board.range(VOLLEY_SESSION_MS);
        board
    }

    pub fn phase(&self) -> CourtPhase {
        self.phase
    }

    pub fn court(&self) -> Court {
        self.court
    }

    /// 지금 판에 있는 마리들. id 오름차순이다.
    pub fn participants(&self) -> Vec<PetId> {
        self.players.keys().copied().collect()
    }

    pub(super) fn player_count(&self) -> usize {
        self.players.len()
    }

    /// 이 마리의 팀. 참여하지 않으면 `None`.
    pub(super) fn side_of(&self, id: PetId) -> Option<Side> {
        self.players.get(&id).map(|(side, _)| *side)
    }

    /// 한 팀의 마리들. id 오름차순이다.
    pub(super) fn ids_on(&self, side: Side) -> Vec<PetId> {
        self.players
            .iter()
            .filter(|(_, (s, _))| *s == side)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 다음 왕복에서 공을 받을 팀 — 부르는 쪽이 그 팀의 위치를 모아 넘기는 데 쓴다.
    pub(super) fn next_side(&self) -> Side {
        self.to_side.other()
    }

    /// 이번 왕복에서 받을 마리와 목적지.
    pub(super) fn receiver(&self) -> Option<(PetId, f64)> {
        self.receiver
    }

    /// 방금 계획한 왕복의 체공. 리듬이 실제로 갈리는지 테스트가 확인한다.
    #[cfg(test)]
    pub(super) fn last_flight_ms(&self) -> u64 {
        self.last_flight_ms
    }

    /// 한 마리가 판에서 빠진다 — 지워졌거나, 드래그로 끌려 나갔거나,
    /// 빠따에 맞아 다른 동작으로 넘어갔거나.
    pub(super) fn leave(&mut self, id: PetId) {
        self.players.remove(&id);
        if self.receiver.is_some_and(|(rid, _)| rid == id) {
            // **받을 마리가 사라지면 아무도 안 받는다** — 공은 그대로 모래로
            // 떨어지고 판이 끝난다. 다른 마리에게 넘기지 않는 이유: 사용자가
            // 방금 집어 올린 그 마리가 받을 차례였다는 사실이 화면에 보인다.
            self.receiver = None;
        }
    }

    /// 지금 화면에 있어야 하는 공. 모이는 중에는 없다.
    pub fn ball(&self) -> Option<VolleyBallSnapshot> {
        self.ball.map(|b| VolleyBallSnapshot {
            x: b.x,
            y: b.y,
            flying: self.phase == CourtPhase::Rally,
        })
    }

    /// 브릿지가 한 번에 받아 가는 겉모습.
    pub fn snapshot(&self) -> VolleySnapshot {
        VolleySnapshot {
            court: self.court.rect(),
            ball: self.ball(),
        }
    }

    /// 이번 틱의 경과 시간(초). 틱이 밀려도 공이 순간이동하지 않게
    /// `Pet::step`과 같은 상한을 쓴다.
    pub(super) fn tick(&mut self, now_ms: u64) -> f64 {
        let elapsed = now_ms.saturating_sub(self.last_step_ms).min(MAX_STEP_MS);
        self.last_step_ms = now_ms;
        elapsed as f64 / 1000.0
    }

    /// 판이 통째로 시간을 다 썼는가.
    pub(super) fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    /// 서브 — **국면이 아니라 "자기에게 보내는 왕복 0번"이다** (KTD4).
    ///
    /// 서버 위로 공을 똑바로 띄우고 받을 마리를 **자기 자신**으로 잡아 두면,
    /// 공이 내려올 때 평소의 접촉 판정이 그대로 발동해 서버가 때려 넘긴다.
    /// 물리도 코드도 한 벌이고 화면에는 진짜 토스가 보인다.
    pub(super) fn serve(&mut self, now_ms: u64, positions: &[(PetId, Side, f64)]) {
        let 먼저 = if self.next_u64() % 2 == 0 {
            Side::Left
        } else {
            Side::Right
        };
        let server = positions
            .iter()
            .find(|(_, s, _)| *s == 먼저)
            .or_else(|| positions.first());
        let Some(&(id, side, cx)) = server else {
            return;
        };
        let t = VOLLEY_SERVE_MS as f64 / 1000.0;
        let c = self.court.contact_y();
        self.ball = Some(VolleyBall {
            x: cx,
            y: c,
            prev_y: c,
            vx: 0.0,
            vy: -VOLLEY_GRAVITY * t / 2.0,
        });
        self.to_side = side;
        self.receiver = Some((id, cx));
        self.last_flight_ms = VOLLEY_SERVE_MS;
        self.phase = CourtPhase::Rally;
        self.last_step_ms = now_ms;
    }

    /// 공을 한 틱 날린다.
    pub(super) fn step_ball(&mut self, dt: f64) {
        let Some(ball) = self.ball.as_mut() else {
            return;
        };
        ball.prev_y = ball.y;
        ball.vy += VOLLEY_GRAVITY * dt;
        ball.x += ball.vx * dt;
        ball.y += ball.vy * dt;
    }

    /// 몸통 가운데가 `cx`인 마리가 지금 공을 치는가.
    ///
    /// **예산이 다 됐으면 무조건 아니다** — 그게 판이 반드시 끝나는 이유다.
    pub(super) fn contact_at(&self, cx: f64) -> bool {
        if self.rally_over {
            return false;
        }
        let Some(ball) = self.ball else {
            return false;
        };
        // 올라가는 공은 안 친다 — 올라갈 때도 치면 서브가 자기 손을 두 번 떠난다.
        if ball.vy <= 0.0 {
            return false;
        }
        let c = self.court.contact_y();
        // **점이 아니라 이번 틱에 지나온 구간으로 본다** (`prev_y` 참고).
        if !(ball.prev_y <= c && ball.y >= c) {
            return false;
        }
        (ball.x - cx).abs() <= VOLLEY_REACH_X
    }

    /// 맞았다 — **다음 왕복을 계획한다.** 20초를 지루하지 않게 채우는 일이
    /// 전부 이 함수 안에 있다.
    ///
    /// `on_side`는 [`next_side`](Self::next_side) 팀의 (id, 몸통 가운데 x)들이다.
    ///
    /// 갈래 넷: **어디로**(목적지를 뽑는다) → **누가**(목적지에서 가장 가까운
    /// 마리, 난수가 아니라 거리다) → **어떻게**(체공 세 등급) → **얼마나
    /// 멀리**(체공을 도착 가능한 최소값으로 눌러 잡으므로 먼 공은 저절로
    /// 토스가 된다). 마지막 왕복만 예외로 도착 보장을 끄고 빈 곳에 꽂는다.
    pub(super) fn hit(&mut self, now_ms: u64, on_side: &[(PetId, f64)]) {
        let Some(mut ball) = self.ball else {
            return;
        };
        // **타점에 딱 붙여 놓는다.** 다음 포물선이 정확히 타점에서 출발해야
        // "타점에서 타점으로"라는 네트 보장(KTD6)이 성립한다.
        ball.y = self.court.contact_y();
        ball.prev_y = ball.y;

        let to = self.to_side.other();
        let (lo, hi) = self.court.span_of(to);
        let 마지막 = now_ms >= self.rally_until_ms;

        let target_cx = if 마지막 {
            // **아무 마리에게서도 가장 먼 자리에 꽂는다.** 받는 마리는 뛰지만
            // 못 미치고, 공이 모래에 박힌다.
            farthest_from(on_side, lo, hi)
        } else {
            lo + self.fraction() * (hi - lo)
        };
        let receiver = nearest_to(on_side, target_cx);
        let 뛸_거리 = receiver
            .and_then(|id| on_side.iter().find(|(i, _)| *i == id))
            .map_or(0.0, |(_, cx)| (target_cx - cx).abs());

        let grade = if 마지막 {
            VOLLEY_FLIGHT_MS[0]
        } else {
            VOLLEY_FLIGHT_MS[(self.next_u64() % VOLLEY_FLIGHT_MS.len() as u64) as usize]
        };
        // 마지막 왕복만 **도착 보장을 끈다** — 그래서 못 받는다.
        let ms = flight_ms_for(&self.court, if 마지막 { 0.0 } else { 뛸_거리 }, grade);
        let t = ms as f64 / 1000.0;
        ball.vx = (target_cx - ball.x) / t;
        // 타점에서 출발해 `t` 뒤 타점으로 돌아온다.
        ball.vy = -VOLLEY_GRAVITY * t / 2.0;

        self.ball = Some(ball);
        self.to_side = to;
        self.receiver = receiver.map(|id| (id, target_cx));
        self.last_flight_ms = ms;
        if 마지막 {
            self.rally_over = true;
        }
    }

    /// 공이 모래에 닿았거나 코트를 벗어났는가 — 랠리가 끝나는 조건이다.
    pub(super) fn landed(&self) -> bool {
        let Some(ball) = self.ball else {
            return false;
        };
        ball.y + VOLLEY_BALL_SIZE / 2.0 >= self.court.sand_y() || self.court.out_of(ball.x)
    }

    /// 공이 떨어진 쪽 = **진 팀.** 기록이 아니라 반응의 대상이다 (R10/R11).
    pub(super) fn loser(&self) -> Side {
        self.ball
            .map_or(self.to_side, |b| self.court.side_of_cx(b.x))
    }

    /// 득점 — 공을 모래에 눕히고 뜸을 들인다.
    pub(super) fn settle(&mut self, now_ms: u64) {
        self.phase = CourtPhase::Point;
        self.until_ms = now_ms + VOLLEY_POINT_MS;
        self.receiver = None;
        if let Some(ball) = self.ball.as_mut() {
            ball.vx = 0.0;
            ball.vy = 0.0;
            ball.y = self.court.sand_y() - VOLLEY_BALL_SIZE / 2.0;
            ball.prev_y = ball.y;
        }
    }

    pub(super) fn settled(&self, now_ms: u64) -> bool {
        now_ms >= self.until_ms
    }

    /// 예산이 다 된 것으로 친다 — 테스트가 킬샷 뒤의 상태를 직접 만드는 데 쓴다.
    #[cfg(test)]
    fn force_rally_over(&mut self) {
        self.rally_over = true;
    }

    /// xorshift64 — 판이 자기 난수를 소유한다 (`Pet::next_u64`와 같은 것을
    /// 쓰지만 **상태가 다르다**: 마리의 수열을 태우지 않는 것이 핵심이다).
    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn fraction(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, (lo, hi): (u64, u64)) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }
}

/// 목적지에서 **가장 가까운** 마리. 난수가 아니라 거리로 정하는 것이 핵심이다 —
/// 난수로 뽑으면 "왜 쟤가?"가 되고 아무도 안 뛰는 것처럼 보인다. 거리로 정하면
/// *공이 저쪽으로 갔으니 쟤가 뛰겠구나*가 읽힌다.
///
/// 같은 거리면 id가 작은 쪽 — 안 정해 두면 같은 시드가 다른 랠리를 낳는다.
pub(super) fn nearest_to(on_side: &[(PetId, f64)], target_cx: f64) -> Option<PetId> {
    on_side
        .iter()
        .min_by(|a, b| {
            (a.1 - target_cx)
                .abs()
                .total_cmp(&(b.1 - target_cx).abs())
                .then(a.0.cmp(&b.0))
        })
        .map(|(id, _)| *id)
}

/// `[lo, hi]`에서 **어느 마리에게서도 가장 먼** 지점. 킬샷의 목적지다.
///
/// 후보는 양 끝과 이웃한 두 마리의 중간점뿐이다 — 최소 거리 함수가 구간마다
/// 꺾인 직선이라 최댓값이 반드시 그중 하나에서 나온다.
fn farthest_from(on_side: &[(PetId, f64)], lo: f64, hi: f64) -> f64 {
    if on_side.is_empty() {
        return (lo + hi) / 2.0;
    }
    let mut xs: Vec<f64> = on_side.iter().map(|(_, cx)| *cx).collect();
    xs.sort_by(f64::total_cmp);
    let 최소거리 = |c: f64| {
        xs.iter()
            .map(|x| (c - x).abs())
            .fold(f64::INFINITY, f64::min)
    };
    let mut best = lo;
    let mut best_d = 최소거리(lo);
    let mut 후보 = vec![hi];
    for w in xs.windows(2) {
        후보.push((w[0] + w[1]) / 2.0);
    }
    for c in 후보 {
        let c = c.clamp(lo, hi);
        let d = 최소거리(c);
        if d > best_d {
            best_d = d;
            best = c;
        }
    }
    best
}

/// 이 왕복의 체공 (ms).
///
/// **아래를 눌러 잡는다**: 등급이 뭐든 받을 마리가 도착할 시간은 준다. 그래서
/// 먼 곳을 노린 공은 저절로 토스가 되고 펭귄이 코트를 가로질러 길게 뛴다 —
/// 목적지 주사위 하나가 "누가 뛰나"와 "어떻게 나나"를 함께 굴린다.
///
/// **위는 둘로 자른다**: 가장 긴 등급을 넘지 않고, 정점이 천장을 넘지 않는다.
/// 정점은 `g·T²/8`이므로 천장까지의 높이 `h`가 `T ≤ sqrt(8h/g)`를 정한다.
fn flight_ms_for(court: &Court, 뛸_거리: f64, grade_ms: u64) -> u64 {
    let 필요 = (뛸_거리 / VOLLEY_CHASE_SPEED * 1000.0).ceil() as u64 + VOLLEY_ARRIVE_MARGIN_MS;
    let ms = grade_ms.max(필요).min(VOLLEY_FLIGHT_MS[2]);
    let 높이 = (court.contact_y() - court.ceiling_y()).max(0.0);
    let 천장 = (8.0 * 높이 / VOLLEY_GRAVITY).sqrt() * 1000.0;
    (ms as f64).min(천장).max(VOLLEY_FLIGHT_MS[0] as f64 / 2.0) as u64
}

#[cfg(test)]
#[path = "volleyball_tests.rs"]
mod tests;
