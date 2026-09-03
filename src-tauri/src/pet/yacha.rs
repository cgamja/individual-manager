//! 단체 야차 판 — 마리마다 제 상태 기계를 돌리는 난투, 다운 일정, 세레모니.
//!
//! **경기장을 그리지 않고 대형도 없다.** 볼링·발리볼은 판을 깔고 자리를 정해
//! 줬지만, 여기서는 각자 상대를 골라 사방으로 붙었다 빠진다 (2026-09-03 사용자
//! 지시). 그래서 이 판이 만드는 창은 미녀 펭귄 하나뿐이다.
//!
//! **명세는 `docs/plans/2026-09-03-027-feat-yacha-brawl-plan.md`의 "난투 명세"
//! 절이다** — 확률표·거리·계수가 거기 그대로 적혀 있고, 이 파일과 어긋나면
//! 그쪽이 이긴다.
//!
//! **박자에 맞춰 다 같이 치지 않는다.** 마리마다 제 상태 기계를 돌리므로
//! 타이밍이 제각각이고, 그래서 퍽퍽퍽이 몰렸다 끊겼다 한다. 상대도 각자 고르는데
//! **절반은 가까운 놈 절반은 아무나**라서 1:1도 1:N도 규칙 없이 생긴다.
//!
//! **"튕겨나가지 않는다"와 "스스로 움직인다"는 모순이 아니다.** 금지되는 것은
//! 남이 준 속도(넉백·충돌 튕김)이고, 제 발로 다가가고 맴돌고 빼는 것은 오히려
//! 이 동작의 전부다. 겹침 해소도 **밀려나는 게 아니라 비켜서는 것**이다.

use std::collections::BTreeMap;

use serde::Serialize;

use super::*;

/// 판 전체가 거쳐 가는 국면.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RingPhase {
    /// 하던 짓을 멈추고 화면 가운데로 날아온다. 장갑을 낀다.
    Gathering,
    /// 난투. 각자 붙었다 빠진다.
    Brawl,
    /// 최후의 1인이 양 날개를 든다 — 글자를 못 쓰는 이 앱에서 링 아나운서의
    /// 결과 발표를 대신하는 자리다.
    Victory,
    /// 미녀 펭귄이 오른쪽 화면 밖에서 벨트를 들고 걸어 들어온다.
    QueenIn,
    /// 벨트를 채운다.
    Belting,
    /// 챔피언이 벨트를 들어 보이고 미녀가 박수를 친다.
    Ceremony,
    /// 미녀가 나가고 쓰러진 놈들이 일어난다.
    Exiting,
    /// 끝났다. `Pets`가 이걸 보고 판을 접는다.
    Done,
}

/// 난투 중 마리 하나가 하는 짓.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Act {
    /// 상대에게 다가간다
    Hunt,
    /// 상대를 축으로 맴돈다
    Circle,
    /// 뒤로 뺀다
    Back,
    /// 막는다 — **실제로 막는다**: 가드 중인 놈을 치면 피격으로 안 친다
    Guard,
    /// 친다
    Swing,
    /// 맞고 휘청인다
    Hurt,
    /// 혼자 남았다 — 아무도 안 친다
    Idle,
}

impl Act {
    /// 웹뷰가 받을 국면. `Down`은 판이 따로 정한다.
    pub(super) fn phase(self) -> YachaPhase {
        match self {
            Act::Hunt => YachaPhase::Hunt,
            Act::Circle => YachaPhase::Circle,
            Act::Back => YachaPhase::Back,
            Act::Guard => YachaPhase::Guard,
            Act::Swing => YachaPhase::Punch,
            Act::Hurt => YachaPhase::Hurt,
            Act::Idle => YachaPhase::Guard,
        }
    }
}

/// 미녀 펭귄의 자세.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueenPose {
    WalkIn,
    Belting,
    Clap,
    WalkOut,
}

/// 미녀 펭귄의 겉모습. 브릿지가 창을 옮기고 웹뷰가 자세를 그린다.
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct QueenSnapshot {
    /// 좌상단 세계 좌표 (`Pet::x`와 같은 관례).
    pub x: f64,
    pub y: f64,
    pub facing: Facing,
    pub pose: QueenPose,
}

/// 브릿지가 한 번에 받아 가는 판의 겉모습.
#[derive(Clone, Debug)]
pub struct YachaSnapshot {
    /// 지금 화면에 있어야 하는 미녀. 난투 중에는 없다.
    /// **판이 만드는 창은 이것 하나뿐이다** — 경기장을 안 그린다.
    pub queen: Option<QueenSnapshot>,
    /// **뒤에서 앞** 순서. 브릿지가 창 레벨로 옮겨 적는다 (`pet_bridge/depth.rs`).
    pub depth: Vec<PetId>,
}

/// 싸움판 — 판이 열릴 때 경계에서 **한 번** 재고, 판이 도는 동안은 이게 세계다.
///
/// **그림이 없다.** 경기장을 안 그리므로 창도 사각형도 없고, 이 값이 하는 일은
/// "어디까지 다닐 수 있나"를 정하는 것뿐이다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Arena {
    cx: f64,
    half: f64,
    /// 발밑 기준선 — 세로 오프셋 0인 자리다.
    floor_y: f64,
    world_right: f64,
    bounds: Bounds,
}

impl Arena {
    /// 경계에서 판을 낸다. **너무 좁으면 `None`** — 붙었다 빠질 자리가 안 나온다.
    pub(super) fn new(bounds: Bounds) -> Option<Arena> {
        let width = bounds.right - bounds.left;
        let height = bounds.floor_y - bounds.top;
        if width < YACHA_MIN_WORLD_WIDTH || height < YACHA_MIN_WORLD_HEIGHT {
            return None;
        }
        let top = bounds.top.min(bounds.floor_y);
        Some(Arena {
            cx: (bounds.left + bounds.right) / 2.0,
            half: YACHA_ARENA_HALF.min(width / 2.0 - PET_SIZE / 2.0),
            floor_y: (top + bounds.floor_y) / 2.0 + PET_SIZE / 2.0,
            world_right: bounds.right,
            bounds,
        })
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// 세로 오프셋을 **펭귄 좌상단 y**로. 발밑에서 몸 하나만큼 위다.
    fn top_y(&self, y_off: f64) -> f64 {
        self.floor_y + y_off - PET_SIZE
    }

    fn clamp_x(&self, x: f64) -> f64 {
        x.clamp(self.cx - self.half, self.cx + self.half)
    }

    /// 세로도 화면 밖으로 못 나간다 — 판이 화면보다 높을 수 있다.
    fn clamp_y(&self, y: f64) -> f64 {
        let lo = YACHA_ARENA_Y
            .0
            .max(self.bounds.top + PET_SIZE - self.floor_y);
        let hi = YACHA_ARENA_Y.1.min(self.bounds.floor_y - self.floor_y);
        y.clamp(lo.min(hi), hi)
    }
}

/// 싸우는 마리 하나. **좌표는 판이 갖는다** — 마리가 아니라.
#[derive(Clone, Copy, Debug)]
struct Fighter {
    /// 몸통 가운데 x.
    x: f64,
    /// 발밑 기준 세로 오프셋. 작을수록(위) 멀고, 클수록(아래) 가깝다.
    y: f64,
    act: Act,
    until_ms: u64,
    target: Option<PetId>,
    side: f64,
    face: Facing,
    /// 이번 스윙의 판정이 이미 났는가.
    swung: bool,
    hits: u32,
    down_at: Option<u64>,
}

/// 이번 틱에 난 주먹 하나.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Punch {
    pub from: PetId,
    pub to: PetId,
    /// 화남 표시가 뜰 자리 — 둘 사이의 64:36 지점 (맞은 쪽에 가깝게).
    pub mx: f64,
    pub my: f64,
    /// 막혔는가. 막힌 주먹은 피격으로 안 세고 소리도 둔탁하다.
    pub blocked: bool,
}

/// 한 판.
pub struct Yacha {
    phase: RingPhase,
    arena: Arena,
    /// 참여 마리. **좌표까지 여기가 갖는다** — 난투는 마리 하나의 `step`으로는
    /// 답할 수 없는 질문(누구를 노리나)이라 판이 전부 몬다.
    fighters: BTreeMap<PetId, Fighter>,
    order: Vec<PetId>,
    champion: Option<PetId>,
    /// 다운 일정 — 판이 열릴 때 전부 뽑아 두고 이후 안 바뀐다 (KTD5).
    /// 난투 시작으로부터의 **상대 시각**이다.
    down_at: Vec<u64>,
    down_k: usize,
    brawl_from_ms: u64,
    /// 아직 안 굴린 시뮬레이션 시각 — 20ms 걸음으로 따라잡는다.
    sim_ms: u64,
    phase_until_ms: u64,
    queen_x: f64,
    queen_target_x: f64,
    belt_on_champion: bool,
    /// 이번 틱에 난 주먹들. 브릿지가 읽어 가고 다음 틱에 비워진다.
    punches: Vec<Punch>,
    deadline_ms: u64,
    rng: u64,
}

impl Yacha {
    pub(super) fn new(ids: Vec<PetId>, arena: Arena, now_ms: u64, seed: u64) -> Self {
        let mut board = Yacha {
            phase: RingPhase::Gathering,
            arena,
            fighters: BTreeMap::new(),
            order: ids,
            champion: None,
            down_at: Vec::new(),
            down_k: 0,
            brawl_from_ms: now_ms,
            sim_ms: now_ms,
            phase_until_ms: now_ms + YACHA_GATHER_MS,
            queen_x: arena.world_right + PET_SIZE,
            queen_target_x: arena.world_right + PET_SIZE,
            belt_on_champion: false,
            punches: Vec::new(),
            deadline_ms: now_ms + YACHA_MAX_MS,
            // 시드가 0이면 xorshift가 0에 갇힌다 (`Pet::new_at`과 같은 방어).
            rng: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        };
        board.shuffle_order();
        board.scatter();
        board.plan_downs();
        board
    }

    fn shuffle_order(&mut self) {
        for i in (1..self.order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            self.order.swap(i, j);
        }
    }

    /// 처음 자리를 뿌린다. 가운데 근처에 퍼뜨리고 세로는 판 전체에서 뽑는다.
    /// **대형이 아니다.**
    fn scatter(&mut self) {
        let n = self.order.len();
        let ids = self.order.clone();
        let (cx, half) = (self.arena.cx, self.arena.half);
        for (i, id) in ids.into_iter().enumerate() {
            let 폭 = (2.0 * half).min(340.0);
            let jitter = (self.fraction() - 0.5) * 190.0;
            let x = cx - 폭 / 2.0 + (i as f64 + 0.5) * (폭 / n as f64) + jitter;
            let y = YACHA_ARENA_Y.0
                + self.fraction() * (YACHA_ARENA_Y.1 - YACHA_ARENA_Y.0);
            let face = if x < cx { Facing::Right } else { Facing::Left };
            let spot = (self.arena.clamp_x(x), self.arena.clamp_y(y));
            self.fighters.insert(
                id,
                Fighter {
                    x: spot.0,
                    y: spot.1,
                    act: Act::Hunt,
                    until_ms: 0,
                    target: None,
                    side: 1.0,
                    face,
                    swung: false,
                    hits: 0,
                    down_at: None,
                },
            );
        }
    }

    /// **다운 일정을 판 시작에 전부 뽑는다** (KTD5).
    ///
    /// `k`번째 다운은 `[예산·(k+1)/(n+0.35), 예산·(k+2)/(n+0.35))`의 앞쪽 60%에서
    /// 뽑는다. 단조 증가가 구조적으로 보장되고 마지막이 예산 안에 든다.
    fn plan_downs(&mut self) {
        let n = self.order.len();
        self.down_at.clear();
        self.down_k = 0;
        if n < 2 {
            return;
        }
        let budget = YACHA_BRAWL_MS as f64;
        let d = n as f64 + 0.35;
        for k in 0..n - 1 {
            let lo = budget * (k as f64 + 1.0) / d;
            let hi = budget * (k as f64 + 2.0) / d;
            let at = lo + self.fraction() * (hi - lo) * 0.6;
            self.down_at.push(at as u64);
        }
    }

    // ── 읽기 ────────────────────────────────────────────────

    pub fn phase(&self) -> RingPhase {
        self.phase
    }

    pub fn arena(&self) -> Arena {
        self.arena
    }

    pub fn champion(&self) -> Option<PetId> {
        self.champion
    }

    pub fn belt_on_champion(&self) -> bool {
        self.belt_on_champion
    }

    pub fn participants(&self) -> Vec<PetId> {
        self.order.clone()
    }

    /// 아직 서 있는 마리.
    pub fn standing(&self) -> Vec<PetId> {
        self.order
            .iter()
            .filter(|id| self.fighters.get(id).is_some_and(|f| f.down_at.is_none()))
            .copied()
            .collect()
    }

    /// `id`가 지금 있어야 하는 자리 (**펭귄 좌상단**)와 국면·시선.
    pub(super) fn pose_of(&self, id: PetId) -> Option<((f64, f64), YachaPhase, Facing)> {
        let f = self.fighters.get(&id)?;
        let phase = if f.down_at.is_some() {
            YachaPhase::Down
        } else {
            f.act.phase()
        };
        Some((
            (f.x - PET_SIZE / 2.0, self.arena.top_y(f.y)),
            phase,
            f.face,
        ))
    }

    /// 이번 틱에 난 주먹들.
    pub fn punches(&self) -> &[Punch] {
        &self.punches
    }

    pub(super) fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    /// **뒤에서 앞** 순서 — y가 작을수록(위) 뒤다.
    fn depth_order(&self) -> Vec<PetId> {
        let mut v: Vec<(PetId, f64)> =
            self.fighters.iter().map(|(id, f)| (*id, f.y)).collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        v.into_iter().map(|(id, _)| id).collect()
    }

    pub fn snapshot(&self) -> YachaSnapshot {
        YachaSnapshot {
            queen: self.queen_pose().map(|pose| QueenSnapshot {
                x: self.queen_x,
                y: self.arena.top_y(0.0),
                facing: if self.phase == RingPhase::Exiting {
                    Facing::Right
                } else {
                    Facing::Left
                },
                pose,
            }),
            depth: self.depth_order(),
        }
    }

    fn queen_pose(&self) -> Option<QueenPose> {
        match self.phase {
            RingPhase::Victory | RingPhase::QueenIn => Some(QueenPose::WalkIn),
            RingPhase::Belting => Some(QueenPose::Belting),
            RingPhase::Ceremony => Some(QueenPose::Clap),
            RingPhase::Exiting => Some(QueenPose::WalkOut),
            _ => None,
        }
    }

    // ── 판 굴리기 ───────────────────────────────────────────

    /// 마리 하나가 판에서 빠진다 (드래그·삭제·방망이).
    pub(super) fn leave(&mut self, id: PetId) {
        self.order.retain(|x| *x != id);
        self.fighters.remove(&id);
        if self.champion == Some(id) {
            self.champion = None;
        }
        for f in self.fighters.values_mut() {
            if f.target == Some(id) {
                f.target = None;
            }
        }
        // 남은 마리가 줄면 남은 다운도 줄인다 — 안 줄이면 판이 예산 끝까지 헛돈다.
        let 남은 = self.standing().len().saturating_sub(1);
        self.down_at.truncate((self.down_k + 남은).max(self.down_k));
    }

    /// 모이기가 끝났다 — 난투를 시작한다.
    pub(super) fn begin_brawl(&mut self, now_ms: u64) {
        self.phase = RingPhase::Brawl;
        self.brawl_from_ms = now_ms;
        self.sim_ms = now_ms;
        self.phase_until_ms = now_ms + YACHA_BRAWL_MS;
    }

    /// 난투를 **20ms 걸음으로** 지금까지 따라잡는다. 틱(50ms)보다 잘게 도는
    /// 이유는 주먹 판정(진행률 42%)이 틱 간격보다 촘촘해서다.
    pub(super) fn step_brawl(&mut self, now_ms: u64) {
        self.punches.clear();
        // 밀린 틱이 한 번에 몰아치지 않게 상한을 둔다 (`MAX_STEP_MS`와 같은 뜻).
        let 끝 = now_ms.min(self.sim_ms + MAX_STEP_MS);
        while self.sim_ms + YACHA_DT_MS <= 끝 {
            self.sim_ms += YACHA_DT_MS;
            let t = self.sim_ms;
            self.one_step(t);
        }
    }

    fn one_step(&mut self, t: u64) {
        self.settle_down(t);
        let up = self.standing();
        // **혼자 남으면 아무도 안 친다.** 허공에 주먹을 내지르면 이긴 게 아니라
        // 이상해 보인다.
        let 혼자 = up.len() <= 1;

        for id in &up {
            let id = *id;
            if 혼자 {
                if let Some(f) = self.fighters.get_mut(&id) {
                    f.act = Act::Idle;
                    f.target = None;
                    f.until_ms = t + YACHA_DT_MS;
                }
                continue;
            }
            if t >= self.fighters[&id].until_ms {
                self.pick_act(id, t, &up);
            }
            self.move_one(id, t);
        }

        if !혼자 {
            self.separate(&up);
        }
    }

    /// 상태와 상대를 고른다. **닿을 때만 친다** — 헛스윙은 화면에서 아무 일도
    /// 아니라 밀도만 깎는다 (v5에서 4마리 14초에 29발뿐이었던 원인).
    fn pick_act(&mut self, id: PetId, t: u64, up: &[PetId]) {
        let 상대가_없다 = {
            let f = &self.fighters[&id];
            match f.target {
                None => true,
                Some(j) => j == id || !up.contains(&j),
            }
        };
        let 주사위 = self.fraction();
        if 상대가_없다 || 주사위 < YACHA_P_RETARGET {
            let picked = self.pick_target(id, up);
            self.fighters.get_mut(&id).unwrap().target = picked;
        }
        let Some(tgt) = self.fighters[&id].target else {
            return;
        };
        let d = self.dist(id, tgt);
        let r = self.fraction();
        let 방금_쳤다 = self.fighters[&id].act == Act::Swing;
        let (act, hold) = if d > YACHA_REACH {
            // 멀면 붙는 데 쓴다.
            let a = if r < YACHA_P_HUNT {
                Act::Hunt
            } else {
                Act::Circle
            };
            let h = YACHA_HOLD_FAR_MS.0
                + (self.fraction() * YACHA_HOLD_FAR_MS.1 as f64) as u64;
            (a, h)
        } else {
            // 붙어 있는 동안은 계속 친다. **방금 쳤으면 더 친다 — 퍽퍽퍽 연타다.**
            let p = if 방금_쳤다 {
                YACHA_P_SWING.1
            } else {
                YACHA_P_SWING.0
            };
            let a = if r < p {
                Act::Swing
            } else if r < p + YACHA_P_GUARD_ADD {
                Act::Guard
            } else if r < p + YACHA_P_BACK_ADD {
                Act::Back
            } else {
                Act::Circle
            };
            let h = if a == Act::Swing {
                YACHA_SWING_MS
            } else {
                YACHA_HOLD_NEAR_MS.0
                    + (self.fraction() * YACHA_HOLD_NEAR_MS.1 as f64) as u64
            };
            (a, h)
        };
        let side = if self.fraction() < 0.5 { -1.0 } else { 1.0 };
        let f = self.fighters.get_mut(&id).unwrap();
        f.act = act;
        f.until_ms = t + hold;
        f.swung = false;
        if act == Act::Circle {
            f.side = side;
        }
    }

    /// **절반은 제일 가까운 놈, 절반은 아무나** — 이것 하나로 1:1도 1:N도 생긴다.
    fn pick_target(&mut self, id: PetId, up: &[PetId]) -> Option<PetId> {
        let foes: Vec<PetId> = up.iter().copied().filter(|j| *j != id).collect();
        if foes.is_empty() {
            return None;
        }
        if self.fraction() < 0.5 {
            let mut best = foes[0];
            for j in &foes {
                if self.dist(id, *j) < self.dist(id, best) {
                    best = *j;
                }
            }
            Some(best)
        } else {
            let k = (self.next_u64() as usize) % foes.len();
            Some(foes[k])
        }
    }

    /// 화면 거리. 세로는 원근이라 가로보다 무겁게 센다.
    fn dist(&self, i: PetId, j: PetId) -> f64 {
        match (self.fighters.get(&i), self.fighters.get(&j)) {
            (Some(a), Some(b)) => (b.x - a.x).hypot((b.y - a.y) * YACHA_YW),
            _ => f64::INFINITY,
        }
    }

    fn move_one(&mut self, id: PetId, t: u64) {
        let Some(tgt) = self.fighters[&id].target else {
            return;
        };
        if !self.fighters.contains_key(&tgt) {
            return;
        }
        let (ax, ay) = (self.fighters[&id].x, self.fighters[&id].y);
        let (bx, by) = (self.fighters[&tgt].x, self.fighters[&tgt].y);
        let len = (bx - ax).hypot(by - ay).max(1.0);
        let (ux, uy) = ((bx - ax) / len, (by - ay) / len);
        let step = YACHA_STEP_PER_MS * YACHA_DT_MS as f64;
        let act = self.fighters[&id].act;
        let side = self.fighters[&id].side;

        match act {
            Act::Hunt => {
                let f = self.fighters.get_mut(&id).unwrap();
                f.x += ux * step;
                f.y += uy * step * 0.72;
            }
            Act::Circle => {
                {
                    let f = self.fighters.get_mut(&id).unwrap();
                    // 상대를 축으로 돈다 — 수직 성분이라 저절로 대각선으로 흐른다.
                    f.x += -uy * side * step * 0.7;
                    f.y += ux * side * step * 0.5;
                }
                let d = self.dist(id, tgt);
                let f = self.fighters.get_mut(&id).unwrap();
                if d < YACHA_NEAR - 8.0 {
                    f.x -= ux * step * 0.5;
                    f.y -= uy * step * 0.4;
                }
                if d > YACHA_REACH + 66.0 {
                    f.x += ux * step * 0.5;
                    f.y += uy * step * 0.4;
                }
            }
            Act::Back => {
                let f = self.fighters.get_mut(&id).unwrap();
                f.x -= ux * step * 1.4;
                f.y -= uy * step * 1.0;
            }
            Act::Swing => self.try_hit(id, tgt, t),
            _ => {}
        }

        let arena = self.arena;
        let f = self.fighters.get_mut(&id).unwrap();
        // **맴돌 때는 시선을 안 돌린다** — 돌면 도망가는 그림이 된다.
        if act != Act::Circle {
            f.face = if bx >= ax { Facing::Right } else { Facing::Left };
        }
        f.x = arena.clamp_x(f.x);
        f.y = arena.clamp_y(f.y);
    }

    /// 주먹 진행률 42%에서 **한 번만** 판정한다.
    fn try_hit(&mut self, id: PetId, tgt: PetId, t: u64) {
        let f = self.fighters[&id];
        if f.swung {
            return;
        }
        let 남은 = f.until_ms.saturating_sub(t) as f64;
        let 진행 = 1.0 - 남은 / YACHA_SWING_MS as f64;
        if 진행 < YACHA_SWING_HIT_AT {
            return;
        }
        self.fighters.get_mut(&id).unwrap().swung = true;
        if self.dist(id, tgt) > YACHA_REACH {
            return;
        }
        let Some(target) = self.fighters.get(&tgt) else {
            return;
        };
        if target.down_at.is_some() {
            return;
        }
        // **가드는 실제로 막는다.**
        let blocked = target.act == Act::Guard;
        let (tx, ty) = (target.x, target.y);
        if !blocked {
            let target = self.fighters.get_mut(&tgt).unwrap();
            target.hits += 1;
            if target.act != Act::Hurt {
                target.act = Act::Hurt;
                target.until_ms = t + YACHA_HURT_MS;
                target.swung = false;
            }
        }
        self.punches.push(Punch {
            from: id,
            to: tgt,
            mx: f.x * 0.36 + tx * 0.64,
            my: f.y * 0.36 + ty * 0.64,
            blocked,
        });
    }

    /// 완전히 포개지지 않는다. **밀려나는 게 아니라 비켜서는 것**이다 — 맞아서
    /// 튕기는 것과 다르다 (플랜의 "움직임의 두 종류" 절).
    fn separate(&mut self, up: &[PetId]) {
        for a in 0..up.len() {
            for b in (a + 1)..up.len() {
                let (i, j) = (up[a], up[b]);
                let (ix, iy) = (self.fighters[&i].x, self.fighters[&i].y);
                let (jx, jy) = (self.fighters[&j].x, self.fighters[&j].y);
                let (dx, dy) = (jx - ix, (jy - iy) * YACHA_YW);
                let d = dx.hypot(dy);
                if d >= YACHA_SEP || d <= 0.001 {
                    continue;
                }
                let push = (YACHA_SEP - d) * 0.2;
                let (px, py) = (dx / d * push, dy / d / YACHA_YW * push);
                let arena = self.arena;
                for (who, sign) in [(i, -1.0f64), (j, 1.0)] {
                    let f = self.fighters.get_mut(&who).unwrap();
                    f.x = arena.clamp_x(f.x + px * sign);
                    f.y = arena.clamp_y(f.y + py * sign);
                }
            }
        }
    }

    /// 다운 시각이 됐으면 **가장 많이 맞은 서 있는 마리**를 넘어뜨린다.
    fn settle_down(&mut self, t: u64) {
        let 지난 = t.saturating_sub(self.brawl_from_ms);
        while self.down_k < self.down_at.len() && 지난 >= self.down_at[self.down_k] {
            let up = self.standing();
            if up.len() <= 1 {
                return;
            }
            let mut who = up[0];
            let mut most = self.fighters[&who].hits;
            for id in &up {
                let h = self.fighters[id].hits;
                if h > most {
                    most = h;
                    who = *id;
                }
            }
            if most < YACHA_MIN_HITS {
                return; // 덜 맞았다 — 다음 걸음으로 미룬다
            }
            self.fighters.get_mut(&who).unwrap().down_at = Some(t);
            // **한 놈이 넘어가면 남은 놈들이 상대를 다시 고른다.** 쓰러진 놈을
            // 노리던 놈은 반드시, 나머지는 절반. 판을 가로지르는 이동이 여기서
            // 나온다 — 안 그러면 붙어 있던 짝이 그대로 굳는다.
            let 남은: Vec<PetId> = self.standing();
            for id in 남은 {
                let 노리던 = self.fighters[&id].target == Some(who);
                let 다시 = 노리던 || self.fraction() < YACHA_P_RESHUFFLE_ON_DOWN;
                if 다시 {
                    self.fighters.get_mut(&id).unwrap().target = None;
                    // 다음 걸음에 새로 고르도록 상태도 만료시킨다.
                    self.fighters.get_mut(&id).unwrap().until_ms = t;
                }
            }
            self.down_k += 1;
        }
    }

    /// 최후의 1인이 나왔다. 세레모니를 시작한다.
    pub(super) fn crown(&mut self, now_ms: u64, champion: PetId) {
        self.champion = Some(champion);
        self.phase = RingPhase::Victory;
        self.phase_until_ms = now_ms + YACHA_WIN_MS;
        self.queen_x = self.arena.world_right + PET_SIZE;
        let 챔프_x = self
            .fighters
            .get(&champion)
            .map(|f| f.x - PET_SIZE / 2.0)
            .unwrap_or(self.arena.cx);
        self.queen_target_x = 챔프_x + YACHA_QUEEN_STOP_GAP;
    }

    /// 세레모니를 한 틱 굴린다.
    pub(super) fn step_ceremony(&mut self, now_ms: u64, dt: f64) {
        match self.phase {
            RingPhase::QueenIn => {
                self.walk_queen(dt);
                if (self.queen_x - self.queen_target_x).abs() <= ARRIVE_EPSILON {
                    self.queen_x = self.queen_target_x;
                    self.phase = RingPhase::Belting;
                    self.phase_until_ms = now_ms + YACHA_BELT_MS;
                    return;
                }
            }
            RingPhase::Exiting => self.walk_queen(dt),
            _ => {}
        }
        if now_ms < self.phase_until_ms {
            return;
        }
        match self.phase {
            RingPhase::Victory => {
                self.phase = RingPhase::QueenIn;
                // 걸어오는 시간은 거리가 정한다 — 만료로 넘기지 않는다. 다만
                // 못 닿는 경우를 대비해 상한은 둔다.
                self.phase_until_ms = now_ms + YACHA_QUEEN_MS * 4;
            }
            RingPhase::QueenIn => {
                // 상한에 걸렸다 — 그냥 도착한 것으로 친다.
                self.queen_x = self.queen_target_x;
                self.phase = RingPhase::Belting;
                self.phase_until_ms = now_ms + YACHA_BELT_MS;
            }
            RingPhase::Belting => {
                // **벨트는 여기서 넘어간다.** 채우기가 끝나야 챔피언이 찬다.
                self.belt_on_champion = true;
                self.phase = RingPhase::Ceremony;
                self.phase_until_ms = now_ms + YACHA_CEREMONY_MS;
            }
            RingPhase::Ceremony => {
                self.phase = RingPhase::Exiting;
                self.phase_until_ms = now_ms + YACHA_EXIT_MS;
                self.queen_target_x = self.arena.world_right + PET_SIZE;
            }
            RingPhase::Exiting => self.phase = RingPhase::Done,
            _ => {}
        }
    }

    fn walk_queen(&mut self, dt: f64) {
        let 남은 = self.queen_target_x - self.queen_x;
        let 걸음 = YACHA_QUEEN_SPEED * dt;
        if 남은.abs() <= 걸음 {
            self.queen_x = self.queen_target_x;
        } else {
            self.queen_x += 걸음 * 남은.signum();
        }
    }

    /// 지금 국면의 예산이 끝났는가.
    pub(super) fn phase_over(&self, now_ms: u64) -> bool {
        now_ms >= self.phase_until_ms
    }

    // ── 테스트가 읽는 것 ─────────────────────────────────────

    #[cfg(test)]
    pub(super) fn hits_of(&self, id: PetId) -> u32 {
        self.fighters.get(&id).map(|f| f.hits).unwrap_or(0)
    }

    #[cfg(test)]
    pub(super) fn xy_of(&self, id: PetId) -> Option<(f64, f64)> {
        self.fighters.get(&id).map(|f| (f.x, f.y))
    }

    #[cfg(test)]
    pub(super) fn act_of(&self, id: PetId) -> Option<Act> {
        self.fighters.get(&id).map(|f| f.act)
    }

    #[cfg(test)]
    pub(super) fn set_act(&mut self, id: PetId, act: Act, until_ms: u64) {
        if let Some(f) = self.fighters.get_mut(&id) {
            f.act = act;
            f.until_ms = until_ms;
        }
    }

    #[cfg(test)]
    pub(super) fn down_schedule(&self) -> Vec<u64> {
        self.down_at.clone()
    }

    /// xorshift64 — **판이 자기 난수를 소유한다.** `Pet::rng`를 태우면 판에
    /// 참여한 마리만 이후 동작 시퀀스가 밀린다.
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
}

#[cfg(test)]
#[path = "yacha_tests.rs"]
mod tests;
