//! 단체 야차 판 — 링 기하, 스탠스, 난투 라운드, 다운 일정, 세레모니.
//!
//! **볼링·비치발리볼에 이은 셋째 "집결형 한 판"이고, 앞의 둘과 갈리는 지점이
//! 하나 있다: 여기서는 마리끼리 서로를 친다.** 그래서 이 모듈이 절대 하지 않는
//! 일이 하나 있다 — **좌표를 만들지 않는다.** 라운드가 돌려주는 것은 id 쌍과
//! 피격 수뿐이라 밀어낼 방법 자체가 없다 (R7). 튕겨 나가는 것은 핀볼의 일이다.
//!
//! **판은 `Pets`가 소유하고 난수는 판이 갖는다** — 볼링·발리볼과 같은 근거다.
//!
//! 설계 이력과 "왜 이렇게 나눴나"는 `MOTIONS.md`의 단체 야차 절에 있다.

use serde::Serialize;

use super::*;

/// 판 전체가 거쳐 가는 국면.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RingPhase {
    /// 펭귄들이 링의 자기 자리로 날아가는 중.
    Gathering,
    /// 난투. 다운 일정이 굴러간다.
    Brawl,
    /// 최후의 1인이 양 날개를 든다 — 글자를 못 쓰는 이 앱에서 링 아나운서의
    /// 결과 발표를 대신하는 자리다.
    Victory,
    /// 미녀 펭귄이 오른쪽에서 걸어 들어온다.
    QueenIn,
    /// 벨트를 채운다.
    Belting,
    /// 챔피언 + 미녀 세레모니.
    Ceremony,
    /// 미녀가 오른쪽으로 걸어 나가고 링이 걷힌다.
    Exiting,
    /// 끝났다. `Pets`가 이걸 보고 판을 접는다.
    Done,
}

/// 미녀 펭귄의 자세. 자리는 Rust가 창을 옮겨 정하고, 웹뷰는 이걸로 클래스만 고른다.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueenPose {
    /// 벨트를 들고 걸어 들어온다
    WalkIn,
    /// 벨트를 채워 준다
    Belting,
    /// 옆에서 박수를 친다
    Clap,
    /// 걸어 나간다
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

/// 브릿지가 한 번에 받아 가는 판의 겉모습. 락을 한 번만 잡으려고 묶었다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct YachaSnapshot {
    /// 링 창이 덮을 사각형.
    pub ring: (f64, f64, f64, f64),
    /// 지금 화면에 있어야 하는 미녀. 난투 중에는 없다.
    pub queen: Option<QueenSnapshot>,
}

/// 링 — 판이 열릴 때 경계에서 **한 번** 재고, 판이 도는 동안은 이게 세계다.
/// 도중에 경계가 바뀌어도 펭귄과 링이 서로 다른 좌표계를 보지 않게 한다
/// (볼링의 `lane`·발리볼의 `Court`와 같은 이유).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ring {
    /// 링 가운데의 세계 x.
    cx: f64,
    /// 가운데에서 링 끝까지.
    half: f64,
    /// **펭귄 좌상단** y — 화면 세로 중앙이다. 볼링·발리볼과 같은 자리이고,
    /// 링 매트도 여기 함께 뜬다.
    play_y: f64,
    /// 세계 오른쪽 — 미녀가 여기 **밖에서** 걸어 들어온다.
    world_right: f64,
    /// 판이 열릴 때 잰 경계. 판이 도는 동안 이게 세계다 — 마리를 내려보낼 때
    /// 바닥이 어디인지 알아야 하고, 도중에 경계가 바뀌어도 판 안에서는 한
    /// 좌표계만 본다 (볼링의 `lane`·발리볼의 `Court`와 같은 이유).
    bounds: Bounds,
}

impl Ring {
    /// 경계에서 링을 낸다. **너무 좁으면 `None`** — 판을 아예 안 연다.
    pub(super) fn new(bounds: Bounds) -> Option<Ring> {
        let width = bounds.right - bounds.left;
        let height = bounds.floor_y - bounds.top;
        if width < YACHA_MIN_WORLD_WIDTH || height < YACHA_MIN_WORLD_HEIGHT {
            return None;
        }
        let top = bounds.top.min(bounds.floor_y);
        Some(Ring {
            cx: (bounds.left + bounds.right) / 2.0,
            // 링이 화면보다 넓으면 화면에 맞춘다.
            half: YACHA_RING_HALF.min(width / 2.0),
            play_y: (top + bounds.floor_y) / 2.0,
            world_right: bounds.right,
            bounds,
        })
    }

    /// `n`마리 중 `k`번째가 설 자리 (**좌상단**, `Pet::x`와 같은 관례).
    ///
    /// 가운데를 기준으로 좌우 대칭이라 같은 마릿수는 늘 같은 배치를 낳는다.
    /// 간격은 링을 벗어나지 않도록 눌러 잡는다 — 여덟 마리 × 기본 간격이
    /// 링에 들어가는 것은 `tuning.rs`의 `const assert!`가 보장하지만, 링이
    /// 화면에 맞춰 줄어들면 그 보장이 깨지므로 여기서 한 번 더 잡는다.
    pub(super) fn stance_of(&self, k: usize, n: usize) -> (f64, f64) {
        let n = n.max(1);
        let gap = if n > 1 {
            YACHA_STANCE_GAP.min((2.0 * self.half - PET_SIZE) / (n - 1) as f64)
        } else {
            YACHA_STANCE_GAP
        };
        let 전체 = gap * (n - 1) as f64;
        let x = self.cx - 전체 / 2.0 - PET_SIZE / 2.0 + gap * k as f64;
        (x, self.play_y)
    }

    /// 링 창이 덮을 사각형 (좌상단 x, y, 폭, 높이).
    ///
    /// 매트는 펭귄의 **발밑**에 깔린다 — 좌상단에서 `PET_SIZE`만큼 아래가
    /// 매트 윗면이다.
    pub fn rect(&self) -> (f64, f64, f64, f64) {
        (
            self.cx - self.half,
            self.play_y + PET_SIZE - YACHA_RING_DEPTH / 2.0,
            self.half * 2.0,
            YACHA_RING_DEPTH,
        )
    }

    /// 펭귄 좌상단이 놓이는 y.
    pub(super) fn play_y(&self) -> f64 {
        self.play_y
    }

    /// 판이 열릴 때 잰 경계.
    pub(super) fn bounds(&self) -> Bounds {
        self.bounds
    }
}

/// 한 라운드에서 일어난 일. **좌표가 없다** — 이게 R7의 구조적 보장이다.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RoundOutcome {
    /// (때린 마리, 맞은 마리).
    pub punches: Vec<(PetId, PetId)>,
    /// 이번 라운드의 **대표 타격**. 소리는 이 한 마리만 낸다 — 맞는 마리마다
    /// 내면 여덟 마리에서 라운드당 네 발이 겹쳐 기관총이 된다.
    pub thud: Option<PetId>,
}

/// 한 판.
pub struct Yacha {
    phase: RingPhase,
    ring: Ring,
    /// 참여 마리를 **선 순서대로**. 시드로 섞으므로 같은 마릿수라도 매번 다른
    /// 대진이 나온다.
    order: Vec<PetId>,
    /// 쓰러진 마리 (쓰러진 순서대로). `order`에서 빠지지 않는다 — 링에 계속
    /// 누워 있어야 하므로 참여자이긴 하다.
    downed: Vec<PetId>,
    champion: Option<PetId>,
    /// **다운 일정** — 판이 열릴 때 전부 뽑아 두고 이후 안 바뀐다 (KTD5).
    /// 그래야 테스트가 읽을 수 있고, 판 길이가 마릿수에 안 휘둘린다.
    down_at: Vec<u64>,
    down_k: usize,
    round_seq: u64,
    next_round_ms: u64,
    brawl_until_ms: u64,
    /// 지금 국면이 끝나는 시각 (`Victory` 이후의 세레모니 국면들이 쓴다).
    phase_until_ms: u64,
    /// 미녀의 좌상단 x. 국면에 따라 목표를 향해 걸어간다.
    queen_x: f64,
    queen_target_x: f64,
    belt_on_champion: bool,
    deadline_ms: u64,
    rng: u64,
}

impl Yacha {
    pub(super) fn new(ids: Vec<PetId>, ring: Ring, now_ms: u64, seed: u64) -> Self {
        let mut board = Yacha {
            phase: RingPhase::Gathering,
            ring,
            order: ids,
            downed: Vec::new(),
            champion: None,
            down_at: Vec::new(),
            down_k: 0,
            round_seq: 0,
            next_round_ms: now_ms + YACHA_ROUND_MS,
            brawl_until_ms: 0,
            phase_until_ms: 0,
            queen_x: ring.world_right + PET_SIZE,
            queen_target_x: ring.world_right + PET_SIZE,
            belt_on_champion: false,
            deadline_ms: now_ms + YACHA_MAX_MS,
            // 시드가 0이면 xorshift가 0에 갇힌다 (`Pet::new_at`과 같은 방어).
            rng: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        };
        board.shuffle_order();
        let 예산 = board.range(YACHA_BRAWL_MS);
        board.brawl_until_ms = now_ms + 예산;
        board.plan_downs(now_ms, 예산);
        board
    }

    /// 서는 순서를 섞는다. id 순으로 세우면 늘 같은 이웃끼리 싸운다.
    fn shuffle_order(&mut self) {
        for i in (1..self.order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            self.order.swap(i, j);
        }
    }

    /// **다운 일정을 판 시작에 전부 뽑는다** (KTD5).
    ///
    /// `k`번째 다운은 `[B·(k+1)/n, B·(k+2)/n)` 안에서 균등하게 뽑는다. 이 정의는
    /// **단조 증가가 구조적으로 보장**되고 마지막 다운이 항상 예산 직전에 온다.
    /// 두 마리면 구간이 `[B/2, B)` 하나뿐이라 1대1도 예산을 다 쓴다.
    fn plan_downs(&mut self, now_ms: u64, 예산: u64) {
        let n = self.order.len();
        self.down_at.clear();
        self.down_k = 0;
        if n < 2 {
            return;
        }
        // 둘이 거의 동시에 쓰러지면 "하나씩 줄어드는" 리듬이 죽는다 — 앞의
        // 다운에서 최소 이만큼은 떨어뜨린다.
        let 최소_간격 = YACHA_ROUND_MS * 3;
        let mut 직전 = now_ms;
        for k in 0..n - 1 {
            let lo = 예산 * (k as u64 + 1) / n as u64;
            let hi = 예산 * (k as u64 + 2) / n as u64;
            let 폭 = hi.saturating_sub(lo).max(1);
            let 지터 = self.next_u64() % 폭;
            let at = (now_ms + lo + 지터).max(직전 + 최소_간격);
            직전 = at;
            self.down_at.push(at);
        }
    }

    pub fn phase(&self) -> RingPhase {
        self.phase
    }

    pub fn ring(&self) -> Ring {
        self.ring
    }

    pub fn champion(&self) -> Option<PetId> {
        self.champion
    }

    pub fn belt_on_champion(&self) -> bool {
        self.belt_on_champion
    }

    /// 판에 든 마리 전부 (쓰러진 마리 포함).
    pub fn participants(&self) -> Vec<PetId> {
        self.order.clone()
    }

    /// 아직 서 있는 마리 (선 순서대로).
    pub fn standing(&self) -> Vec<PetId> {
        self.order
            .iter()
            .filter(|id| !self.downed.contains(id))
            .copied()
            .collect()
    }

    /// `id`가 **지금** 서야 하는 자리 (좌상단). 쓰러진 마리는 `None` —
    /// 쓰러진 자리에 그대로 누워 있는다.
    ///
    /// **서 있는 마리만으로 다시 잡는다.** 그래야 한 마리가 쓰러질 때마다 남은
    /// 놈들이 가운데로 붙어 **이웃이 늘 사정거리 안**에 있다 — 안 그러면 여덟
    /// 마리 판의 끝에서 양 끝 둘만 남아 600px 떨어진 채 허공에 주먹질하고,
    /// 아무도 안 맞아 판이 영영 안 끝난다 (종료 증명 ②가 여기 걸려 있다).
    /// 붙는 그림 자체도 "하나 쓰러졌다"를 눈에 보이게 한다.
    pub(super) fn stance_for(&self, id: PetId) -> Option<(f64, f64)> {
        let st = self.standing();
        let k = st.iter().position(|x| *x == id)?;
        Some(self.ring.stance_of(k, st.len()))
    }

    #[cfg(test)]
    pub(super) fn brawl_until_ms(&self) -> u64 {
        self.brawl_until_ms
    }

    /// 판이 어떤 이유로도 마리를 이보다 오래 붙들지 못한다 (종료 증명 ③).
    pub(super) fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    /// 마리 하나가 판에서 빠진다 (드래그·삭제·방망이).
    ///
    /// **남은 다운 일정도 함께 줄인다** — 안 줄이면 다섯 마리 일정으로 세 마리를
    /// 쓰러뜨리려다 판이 예산 끝까지 헛돈다.
    pub(super) fn leave(&mut self, id: PetId) {
        self.order.retain(|x| *x != id);
        self.downed.retain(|x| *x != id);
        if self.champion == Some(id) {
            self.champion = None;
        }
        let 남은_다운 = self.standing().len().saturating_sub(1);
        let 써야_할_길이 = self.down_k + 남은_다운;
        self.down_at.truncate(써야_할_길이.max(self.down_k));
    }

    // ── 난투 ────────────────────────────────────────────────

    pub(super) fn begin_brawl(&mut self, now_ms: u64) {
        self.phase = RingPhase::Brawl;
        self.next_round_ms = now_ms + YACHA_ROUND_MS;
    }

    pub(super) fn round_due(&self, now_ms: u64) -> bool {
        now_ms >= self.next_round_ms
    }

    /// 한 라운드. **서 있는 마리의 절반**(인덱스 홀짝이 라운드 번호와 맞는 쪽)이
    /// 가장 가까운 서 있는 이웃을 친다.
    ///
    /// 대상을 난수가 아니라 **거리**로 정하는 것이 핵심이다 — 난수로 뽑으면
    /// "왜 쟤가?"가 되고 아무도 안 치는 것처럼 보인다 (발리볼이 받을 마리를
    /// 거리로 정한 것과 같은 교훈).
    pub(super) fn plan_round(
        &mut self,
        now_ms: u64,
        standing: &[(PetId, f64)],
    ) -> RoundOutcome {
        self.round_seq += 1;
        self.next_round_ms = now_ms + YACHA_ROUND_MS;
        let mut out = RoundOutcome::default();
        if standing.len() < 2 {
            return out;
        }
        let 홀짝 = (self.round_seq % 2) as usize;
        for (i, (id, cx)) in standing.iter().enumerate() {
            if i % 2 != 홀짝 {
                continue;
            }
            let Some(target) = 가장_가까운_이웃(standing, *id, *cx) else {
                continue;
            };
            out.punches.push((*id, target));
        }
        // **대표 타격은 하나뿐이다.** 라운드마다 돌려 가며 골라 소리가 한쪽에
        // 몰리지 않게 한다.
        if !out.punches.is_empty() {
            let k = (self.round_seq as usize) % out.punches.len();
            out.thud = Some(out.punches[k].1);
        }
        out
    }

    pub(super) fn down_due(&self, now_ms: u64) -> bool {
        self.down_k < self.down_at.len() && now_ms >= self.down_at[self.down_k]
    }

    /// 다운 시각이 됐을 때 **가장 많이 맞은** 마리를 쓰러뜨린다.
    ///
    /// 같은 피격 수면 id가 작은 쪽 — 안 정해 두면 같은 시드가 다른 판을 낳는다.
    /// 최다 피격이 `YACHA_MIN_HITS` 미만이면 미룬다: 한 대도 안 맞고 넘어가는
    /// 그림을 막는다.
    pub(super) fn take_down(&mut self, hits: &[(PetId, u32)]) -> Option<PetId> {
        let (id, most) = hits
            .iter()
            .filter(|(id, _)| !self.downed.contains(id))
            .copied()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
        if most < YACHA_MIN_HITS {
            return None;
        }
        self.downed.push(id);
        self.down_k += 1;
        Some(id)
    }

    /// 최후의 1인이 나왔다. 세레모니를 시작한다.
    pub(super) fn crown(&mut self, now_ms: u64, champion: PetId) {
        self.champion = Some(champion);
        self.phase = RingPhase::Victory;
        self.phase_until_ms = now_ms + YACHA_WIN_MS;
        // 미녀는 **화면 오른쪽 밖**에서 출발한다.
        self.queen_x = self.ring.world_right + PET_SIZE;
        let 챔프_x = self
            .stance_for(champion)
            .map(|(x, _)| x)
            .unwrap_or(self.ring.cx);
        self.queen_target_x = 챔프_x + YACHA_QUEEN_STOP_GAP;
    }

    /// 세레모니를 한 틱 굴린다. `Victory` 이후에만 의미가 있다.
    pub(super) fn step(&mut self, now_ms: u64, dt: f64) {
        match self.phase {
            RingPhase::QueenIn => {
                self.walk_queen(dt);
                if (self.queen_x - self.queen_target_x).abs() <= ARRIVE_EPSILON {
                    self.queen_x = self.queen_target_x;
                    self.phase = RingPhase::Belting;
                    self.phase_until_ms = now_ms + YACHA_BELT_MS;
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
                // 걸어오는 데 걸리는 시간은 거리가 정한다 — 만료로 넘기지 않는다.
                self.phase_until_ms = u64::MAX;
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
                self.queen_target_x = self.ring.world_right + PET_SIZE;
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

    pub fn snapshot(&self) -> YachaSnapshot {
        YachaSnapshot {
            ring: self.ring.rect(),
            queen: self.queen_pose().map(|pose| QueenSnapshot {
                x: self.queen_x,
                y: self.ring.play_y(),
                // 걸어 들어올 때는 왼쪽(챔피언 쪽)을 보고, 나갈 때는 오른쪽.
                facing: if self.phase == RingPhase::Exiting {
                    Facing::Right
                } else {
                    Facing::Left
                },
                pose,
            }),
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

    // ── 테스트가 읽는 것 ─────────────────────────────────────

    #[cfg(test)]
    pub(super) fn order(&self) -> Vec<PetId> {
        self.order.clone()
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

    fn range(&mut self, (lo, hi): (u64, u64)) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }
}

/// 자기를 뺀 서 있는 마리 중 **가로로 가장 가까운** 마리. 사정거리 밖이면 `None`.
///
/// 같은 거리면 id가 작은 쪽 — 결정성을 위해서다.
fn 가장_가까운_이웃(standing: &[(PetId, f64)], me: PetId, my_cx: f64) -> Option<PetId> {
    standing
        .iter()
        .filter(|(id, _)| *id != me)
        .filter(|(_, cx)| (cx - my_cx).abs() <= YACHA_REACH_X)
        .min_by(|a, b| {
            (a.1 - my_cx)
                .abs()
                .total_cmp(&(b.1 - my_cx).abs())
                .then(a.0.cmp(&b.0))
        })
        .map(|(id, _)| *id)
}

#[cfg(test)]
#[path = "yacha_tests.rs"]
mod tests;
