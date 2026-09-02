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
    /// **판**의 y (펭귄 좌상단 기준) — 화면 세로 중앙이다. 볼링이 핀을 공중에
    /// 세운 것과 같은 자리이고, **모래는 여기가 아니라 저 아래 화면 바닥에 있다.**
    play_y: f64,
    /// 모래 표면 (해변의 윗선). 화면 바닥이다.
    sand_y: f64,
    /// 올라갈 수 있는 최고점 — 공의 정점을 여기서 자른다.
    top: f64,
    /// 세계의 좌우 끝 (펭귄 좌상단 기준). 모래사장이 여기서 더 뻗는다.
    left: f64,
    right: f64,
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
        let top = bounds.top.min(bounds.floor_y);
        Some(Court {
            net_cx: (bounds.left + bounds.right) / 2.0 + PET_SIZE / 2.0,
            half,
            gap,
            // **판은 화면 세로 중앙이다** — 볼링의 `lane_center_y`와 같다.
            play_y: (top + bounds.floor_y) / 2.0,
            // 모래는 판이 아니라 **화면 바닥**에 있다. 발밑 선
            // (`floor_y + PET_SIZE`)은 작업 영역 바닥과 같아서 거기 그리면
            // 화면 밖이므로, 표면을 그보다 `VOLLEY_SAND_RISE`만큼 올린다.
            sand_y: bounds.floor_y + PET_SIZE - VOLLEY_SAND_RISE,
            top,
            left: bounds.left,
            right: bounds.right,
        })
    }

    pub(super) fn net_cx(&self) -> f64 {
        self.net_cx
    }

    /// 모래 표면 — **화면 바닥의 해변**이다. 공이 여기 닿으면 랠리가 끝난다.
    /// 펭귄 발밑 선보다 `VOLLEY_SAND_RISE`만큼 **위**다 (아래는 화면 밖이다).
    pub(super) fn sand_y(&self) -> f64 {
        self.sand_y
    }

    /// 코트 창의 아래 끝 — 발밑 선보다 더 아래로, 화면 밖까지 내려간다.
    fn window_bottom(&self) -> f64 {
        self.sand_y + VOLLEY_SAND_RISE + VOLLEY_SAND_DEPTH
    }

    /// 네트 꼭대기. **판에 매달려 있다** — 펭귄 머리 바로 밑이다.
    pub(super) fn net_top_y(&self) -> f64 {
        self.play_y + VOLLEY_NET_DROP
    }

    /// 펭귄이 공을 치는 높이 (공 **중심**의 y). **네트 꼭대기보다 위다** —
    /// 그래서 타점에서 타점으로 가는 포물선은 네트에 걸릴 수가 없다 (KTD6).
    pub(super) fn contact_y(&self) -> f64 {
        self.play_y - VOLLEY_REACH
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
        (cx - PET_SIZE / 2.0, self.play_y)
    }

    /// 타점을 지난 공이 **모래까지 더 떨어지는 데 걸리는 시간**(초).
    ///
    /// 공은 타점에서 `vy = -g·t/2`로 출발해 `t` 뒤 타점으로 돌아오므로, 그때의
    /// 하강 속도는 `g·t/2`다. 거기서 `H`만큼 더 떨어지는 시간은
    /// `s² + t·s − 2H/g = 0`의 양근이다.
    pub(super) fn fall_after_contact(&self, t: f64) -> f64 {
        let h = (self.sand_y() - VOLLEY_BALL_SIZE / 2.0 - self.contact_y()).max(0.0);
        let c = 2.0 * h / VOLLEY_GRAVITY;
        (-t + (t * t + 4.0 * c).sqrt()) / 2.0
    }

    /// 모래사장의 좌우 끝 (공 중심 기준). 킬샷은 코트 밖 해변에도 떨어진다 —
    /// 백사장이 화면을 가로지르므로 거기까지가 "모래에 닿았다"이다.
    pub(super) fn sand_span(&self) -> (f64, f64) {
        (
            self.left - VOLLEY_COURT_BLEED,
            self.right + PET_SIZE + VOLLEY_COURT_BLEED,
        )
    }

    /// 공이 **보이는 채로** 떨어질 수 있는 x 범위 (공 중심 기준). `sand_span`은
    /// 모래가 화면 밖까지 뻗는 범위라 착지에 쓰면 안 보이는 데서 끝난다.
    pub(super) fn landing_span(&self) -> (f64, f64) {
        (
            self.left + VOLLEY_BALL_SIZE / 2.0,
            self.right + PET_SIZE - VOLLEY_BALL_SIZE / 2.0,
        )
    }

    /// 모래사장 밖으로 나갔는가 — 여기까지 가면 화면 밖이라 더 볼 것이 없다.
    pub(super) fn out_of(&self, cx: f64) -> bool {
        let (lo, hi) = self.sand_span();
        cx < lo || cx > hi
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
    ///
    /// **네트 꼭대기(화면 중앙)부터 모래 아래(화면 밖)까지**를 한 창이 덮는다.
    /// 창을 둘로 나누지 않은 이유: 창이 늘면 capabilities 라벨·창 레벨·클릭
    /// 통과·반쯤 만들어진 창의 되돌리기가 **그만큼 두 벌**이 된다. 투명하고
    /// 클릭을 통과시키므로 커도 비용이 없다.
    /// **네트는 이 창의 가로 한가운데다** — `sand_span`이 `net_cx`를 중심으로
    /// 대칭이라, 웹뷰가 좌표를 하나도 안 받고 `left: 50%`로 정확히 맞춘다.
    pub fn rect(&self) -> (f64, f64, f64, f64) {
        let (lo, hi) = self.sand_span();
        let top = self.net_top_y();
        (lo, top, hi - lo, self.window_bottom() - top)
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

/// 양 팀에 한 마리씩은 있는가. **없으면 판을 열지 않는다** — 한 팀이 통째로
/// 비면 서브 한 번에 공이 빈 코트로 떨어져 "2초짜리 판"이 된다. 마릿수만
/// 세면(`players.len() >= 2`) 들려 있는 마리가 빠져 한쪽으로 몰린 경우를 놓친다.
pub(super) fn both_sides_present(players: &BTreeMap<PetId, Side>) -> bool {
    players.values().any(|s| *s == Side::Left) && players.values().any(|s| *s == Side::Right)
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
    /// 참여 마리 → 팀. id 오름차순이다. **자리는 안 들고 있는다** — 마리가
    /// 뛰어다니므로 "지금 어디 있나"는 `Pet` 쪽이 원천이고, 판이 자리를 함께
    /// 들고 있으면 두 값이 조용히 갈린다.
    players: BTreeMap<PetId, Side>,
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
        players: BTreeMap<PetId, Side>,
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
        self.players.get(&id).copied()
    }

    /// 한 팀의 마리들. id 오름차순이다.
    pub(super) fn ids_on(&self, side: Side) -> Vec<PetId> {
        self.players
            .iter()
            .filter(|(_, s)| **s == side)
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
        // **체공을 먼저 정한다** — 킬샷의 목적지가 체공에 딸린 낙하 거리에
        // 의존하므로 순서가 뒤바뀌면 안 된다.
        let grade = if 마지막 {
            VOLLEY_FLIGHT_MS[0]
        } else {
            VOLLEY_FLIGHT_MS[(self.next_u64() % VOLLEY_FLIGHT_MS.len() as u64) as usize]
        };
        // **킬샷의 착지점은 체공에 딸린 낙하 거리에 의존한다** — 등급만으로 정해지는
        // 이 값(도착 보장을 끄므로 뛸 거리가 0이다)을 먼저 구해야 순서가 안 뒤집힌다.
        let t0 = flight_ms_for(&self.court, 0.0, grade) as f64 / 1000.0;

        let 빈자리 = farthest_from(on_side, lo, hi);
        // **킬샷은 목적지가 아니라 착지점을 고른다.** 아무도 안 치므로 공은
        // 타점을 지나 **화면 바닥의 모래까지** 떨어지고, 그동안 가로 속도를
        // 그대로 갖는다. 판이 화면 세로 중앙으로 올라가면서 그 낙하가 화면
        // 절반만큼 길어져, 타점 높이로 조준하면 해변을 한참 지나쳐 날아간다.
        let target_cx = if 마지막 {
            // **아무 마리에게서도 가장 먼 자리에 꽂는다.** 받는 마리는 뛰지만
            // 못 미치고, 공이 모래에 박힌다.
            self.kill_landing(ball.x, 빈자리, to, t0)
        } else {
            // 균등하게 뽑되 **빈자리 쪽으로 끌어당긴다.** 순수 균등이면 마릿수가
            // 늘수록 뽑힌 자리가 이미 누군가의 사정거리 안이라 아무도 안 뛴다
            // (`VOLLEY_AWAY_BIAS` 참고). 끌어당김이 곧 "뛰는 그림"의 양이다.
            let 균등 = lo + self.fraction() * (hi - lo);
            균등 + (빈자리 - 균등) * VOLLEY_AWAY_BIAS
        };
        let receiver = nearest_to(on_side, target_cx);
        let 뛸_거리 = receiver
            .and_then(|id| on_side.iter().find(|(i, _)| *i == id))
            .map_or(0.0, |(_, cx)| (target_cx - cx).abs());


        // 마지막 왕복만 **도착 보장을 끈다** — 그래서 못 받는다.
        let ms = flight_ms_for(&self.court, if 마지막 { 0.0 } else { 뛸_거리 }, grade);
        let t = ms as f64 / 1000.0;
        // 타점에서 출발해 `t` 뒤 타점으로 돌아오는 포물선. 이 모양이 네트를
        // 넘는 보장(KTD6)을 만든다.
        ball.vy = -VOLLEY_GRAVITY * t / 2.0;
        // 평소 왕복은 받을 마리가 **타점 높이에서** 치므로 `t`가 곧 그 시각이다.
        // **킬샷은 아무도 안 치므로 모래에 닿는 시각으로 나눈다** — 그래야
        // 고른 자리에 실제로 떨어진다.
        let 도달 = if 마지막 {
            t + self.court.fall_after_contact(t)
        } else {
            t
        };
        ball.vx = (target_cx - ball.x) / 도달;

        self.ball = Some(ball);
        self.to_side = to;
        self.receiver = receiver.map(|id| (id, target_cx));
        self.last_flight_ms = ms;
        if 마지막 {
            self.rally_over = true;
        }
    }

    /// 킬샷이 떨어질 자리를 고른다 — **네트를 넘으면서 해변 안에 떨어지는 x.**
    ///
    /// 두 조건이 서로 당긴다. 가까이 떨어뜨리면 공이 네트에 닿기까지 걸리는
    /// 시간이 전체 비행에서 차지하는 비율이 커져 **타점 아래로 내려간 채 네트를
    /// 지난다.** 멀리 떨어뜨리면 해변을 지나 화면 밖으로 나간다.
    ///
    /// 네트를 넘는 조건은 시간으로 쓰면 한 줄이다. 공이 타점보다
    /// `VOLLEY_NET_CLEAR`만큼 내려가는 데 걸리는 시간을 `s₅₀`라 하면
    /// (`s² − t·s − 2·clear/g = 0`의 양근), **네트에 닿는 시각이 그보다 일러야**
    /// 한다. 가로는 등속이라 시간 비율이 곧 거리 비율이므로 최소 비행 거리가 나온다.
    fn kill_landing(&self, x0: f64, 빈자리: f64, to: Side, t: f64) -> f64 {
        let 전체 = t + self.court.fall_after_contact(t);
        let s50 = (t + (t * t + 8.0 * VOLLEY_NET_CLEAR / VOLLEY_GRAVITY).sqrt()) / 2.0;
        // 1.0에 붙으면 그물을 스치듯 지나가므로 10%를 남긴다.
        let 비율 = (s50 * 0.9 / 전체).min(0.9).max(f64::EPSILON);
        // **네트의 먼 쪽 모서리**를 기준으로 잰다 — 공은 거기서 가장 낮다.
        let 방향 = if to == Side::Right { 1.0 } else { -1.0 };
        let 그물_끝 = self.court.net_cx() + 방향 * VOLLEY_NET_HALF_W;
        let 최소_거리 = (그물_끝 - x0).abs() / 비율;
        // **화면 안에 떨어져야 한다.** `sand_span`은 모래가 화면 밖까지 뻗는
        // 범위라 그걸로 자르면 공이 안 보이는 데서 끝난다 — 세계의 좌우 끝을 쓴다.
        let (화면_lo, 화면_hi) = self.court.landing_span();
        // **자르는 순서가 곧 우선순위다.** 네트를 넘는 조건을 나중에 한 번 더
        // 걸어, 두 조건이 부딪히면 **네트 쪽이 이긴다** — 화면 밖에 떨어지는 것은
        // 그림이 아쉬운 것이고, 그물을 뚫는 것은 물리가 거짓말을 하는 것이다.
        match to {
            Side::Right => 빈자리.max(x0 + 최소_거리).min(화면_hi).max(x0 + 최소_거리),
            Side::Left => 빈자리.min(x0 - 최소_거리).max(화면_lo).min(x0 - 최소_거리),
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
