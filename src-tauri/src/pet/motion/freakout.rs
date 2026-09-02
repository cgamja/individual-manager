//! 발작 — **30,000번에 한 번** 이유 없이 터지는 광란.
//!
//! 한 사이클 6.8초 기준 깨어 있는 57시간, 하루 열 시간이면 엿새에 한 번꼴이다.
//! 사방으로 마구 튀다가 바닥으로 돌아와 숨을 고르고 **아무 일 없었다는 듯**
//! 평소로 돌아간다.
//!
//! **원인이 없는 것이 이 동작의 정의다.** 원인이 있으면 그건 화(`Squawk`)다 —
//! PRD Q8을 "순수 저확률 무작위"로 닫은 이유가 그것이다. "아무 일 없었다는 듯
//! 돌아온다"는 원인이 없어야 성립한다.
//!
//! 던지기 물리 대신 헤엄식 목적지를 쓴다 — 중력이 바닥으로 끌고 가면 착지
//! 판정이 철푸덕·널브러짐을 유발한다.
//!
//! 국면 둘(돌진·숨 고르기)로 나눈 것은 CSS 길이 대조와 자세 튐을 함께 풀기
//! 위해서다. 돌진은 무한 반복이라 길이가 코어의 추첨값이고 대조 대상이 아니다.

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
mod tests {
    use super::*;
    use crate::pet::test_support::*;

    // ── 발작 ───────────────────────────────────────────────────────

    /// `pick_next`를 그 자리에서 여러 번 굴려 어떤 동작이 몇 번 나오는지 센다.
    ///
    /// **`step`을 거치지 않는다.** 1/30000짜리 갈래를 실제 시간으로 재현하려면
    /// 며칠치를 돌려야 하는데, 다음 동작을 고르는 것은 `pick_next` 한 번이므로
    /// 그것만 반복하면 같은 확률을 훨씬 싸게 확인할 수 있다.
    ///
    /// **펭귄 하나의 난수열을 계속 쓴다.** 시드를 매번 바꾸면 xorshift의 첫 출력만
    /// 보게 되어 분포가 시드 배열에 딸린다.
    fn 굴려_세기(횟수: u32, air: bool) -> (u32, u32) {
        let mut p = pet();
        let (mut 발작, mut 낚시) = (0, 0);
        for _ in 0..횟수 {
            p.behavior = Behavior::Walk;
            p.air = air;
            p.last_stimulus_ms = 0;
            p.pick_next(0, BOUNDS);
            match p.behavior {
                Behavior::Freakout { .. } => 발작 += 1,
                Behavior::IceFishing { .. } => 낚시 += 1,
                _ => {}
            }
        }
        (발작, 낚시)
    }

    /// 시켜서 발작 중인 펭귄.
    fn 발작하는_펭귄() -> (Pet, u64) {
        let mut p = pet();
        p.step(1_000, &world());
        assert!(p.start_freakout(1_000), "시키면 시작해야 한다");
        (p, 1_000)
    }

    /// 발작이 끝날 때까지 진행시키며 스냅샷을 모은다.
    fn 발작_끝까지(p: &mut Pet, from: u64) -> Vec<Snapshot> {
        let mut out = Vec::new();
        let mut t = from;
        // 예산 + 바닥 복귀 + 숨 고르기를 넉넉히 덮는다
        let 한계 = from + FREAKOUT_MS.1 * 3 + FREAKOUT_PANT_MS + 5_000;
        while t < 한계 {
            t += 50;
            let s = p.step(t, &world());
            let 끝났다 = !matches!(s.behavior, Behavior::Freakout { .. });
            out.push(s);
            if 끝났다 {
                break;
            }
        }
        out
    }

    #[test]
    fn 저절로_발작이_나온다() {
        // 며칠에 한 번이라도 **도달할 수는 있어야** 한다. 기대값은
        // 200_000 / FREAKOUT_ONE_IN ≈ 6.7회다.
        let (발작, _) = 굴려_세기(200_000, false);
        assert!(발작 > 0, "20만 번을 굴려도 발작이 한 번도 안 나온다");
    }

    #[test]
    fn 발작은_얼음낚시보다_훨씬_드물다() {
        // 값을 하드코딩하지 않는다 — 상수를 고치면 같이 따라와야 한다
        let (발작, 낚시) = 굴려_세기(200_000, false);
        assert!(
            낚시 > 발작 * 10,
            "발작({발작})이 얼음낚시({낚시})와 비슷하면 '희귀' 등급이 아니다"
        );
    }

    #[test]
    fn 발작은_공중에서_저절로_시작하지_않는다() {
        let (발작, _) = 굴려_세기(200_000, true);
        assert_eq!(발작, 0, "공중에는 발작할 자리가 없다");
    }

    #[test]
    fn 발작하는_동안_방향이_여러_번_바뀐다() {
        // 광란으로 읽히게 하는 건 속도가 아니라 **방향이 바뀌는 빈도**다
        let (mut p, t) = 발작하는_펭귄();
        let 전체 = 발작_끝까지(&mut p, t);
        let mut 뒤집힘 = 0;
        let mut 직전: Option<f64> = None;
        for w in 전체.windows(2) {
            let dx = w[1].x - w[0].x;
            if dx.abs() < 0.5 {
                continue;
            }
            if let Some(prev) = 직전 {
                if (dx > 0.0) != (prev > 0.0) {
                    뒤집힘 += 1;
                }
            }
            직전 = Some(dx);
        }
        assert!(뒤집힘 >= 3, "방향이 {뒤집힘}번밖에 안 바뀌었다");
    }

    #[test]
    fn 발작은_헤엄보다_빠르다() {
        let (mut p, t) = 발작하는_펭귄();
        let 전체 = 발작_끝까지(&mut p, t);
        let 최대 = 전체
            .windows(2)
            .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
            .fold(0.0f64, f64::max);
        let 헤엄_한_틱 = SWIM_SPEED * 0.05;
        assert!(
            최대 > 헤엄_한_틱,
            "한 틱 최대 이동({최대:.1})이 헤엄 한 틱({헤엄_한_틱:.1})보다 크지 않다"
        );
    }

    #[test]
    fn 발작하는_동안_경계를_넘지_않는다() {
        // 세계는 화면 하나이고 경계는 벽이다 (PRINCIPLE 2)
        let (mut p, t) = 발작하는_펭귄();
        for s in 발작_끝까지(&mut p, t) {
            assert!(
                s.x >= BOUNDS.left && s.x <= BOUNDS.right,
                "가로 경계를 넘었다: {}",
                s.x
            );
            assert!(
                s.y >= BOUNDS.top && s.y <= BOUNDS.floor_y,
                "세로 경계를 넘었다: {}",
                s.y
            );
        }
    }

    #[test]
    fn 발작이_끝나면_바닥에서_숨을_고른다() {
        let (mut p, t) = 발작하는_펭귄();
        let mut 봤다 = false;
        let mut now = t;
        let 한계 = t + FREAKOUT_MS.1 * 3 + 5_000;
        while now < 한계 {
            now += 50;
            let s = p.step(now, &world());
            if matches!(
                s.behavior,
                Behavior::Freakout { freakout: FreakoutPhase::Pant }
            ) {
                assert_eq!(s.y, BOUNDS.floor_y, "숨은 바닥에서 고른다");
                assert!(!s.air, "숨 고르기는 지상 동작이다");
                봤다 = true;
                break;
            }
        }
        assert!(봤다, "숨 고르기 국면을 한 번도 못 봤다");
    }

    #[test]
    fn 발작은_철푸덕이나_널브러짐으로_끝나지_않는다() {
        // "아무 일 없었다는 듯 돌아온다"가 이 동작의 정의다
        let (mut p, t) = 발작하는_펭귄();
        for s in 발작_끝까지(&mut p, t) {
            assert!(
                !s.behavior.is_landing(),
                "착지 단계가 나왔다: {:?}",
                s.behavior
            );
        }
    }

    #[test]
    fn 숨_고르기가_끝나면_유휴로_간다() {
        // 약을 올리며 나가지 않는다 — 아무 일 없었다는 듯 돌아가야 한다
        let (mut p, t) = 발작하는_펭귄();
        let 마지막 = 발작_끝까지(&mut p, t).pop().expect("스냅샷이 있어야 한다");
        assert!(
            matches!(마지막.behavior, Behavior::Idle { .. }),
            "유휴로 나가야 한다 (실제: {:?})",
            마지막.behavior
        );
    }

    #[test]
    fn 발작_한_판은_예산_안에_끝난다() {
        let (mut p, t) = 발작하는_펭귄();
        let 전체 = 발작_끝까지(&mut p, t);
        let 걸린_시간 = 전체.len() as u64 * 50;
        assert!(
            걸린_시간 <= FREAKOUT_MS.1 * 3 + FREAKOUT_PANT_MS + 1_000,
            "발작이 {걸린_시간}ms나 이어졌다"
        );
    }

    #[test]
    fn 걸을_폭이_없는_화면에서도_발작이_갇히지_않는다() {
        // 좌우가 겹치면 목적지가 한 점이라 영원히 도착만 반복할 수 있다
        let 좁은 = World::single(Bounds { left: 0.0, right: 0.0, top: 800.0, floor_y: 800.0 });
        let mut p = Pet::new(42, 0, &좁은);
        assert!(p.start_freakout(1_000));
        let mut t = 1_000;
        let 한계 = t + FREAKOUT_MS.1 * 3 + FREAKOUT_PANT_MS + 5_000;
        while t < 한계 && matches!(p.behavior(), Behavior::Freakout { .. }) {
            t += 50;
            p.step(t, &좁은);
        }
        assert!(
            !matches!(p.behavior(), Behavior::Freakout { .. }),
            "발작에 갇혔다"
        );
    }

    #[test]
    fn 상한을_넘겨도_공중에서_바닥으로_순간이동하지_않는다() {
        // `dt`는 `MAX_STEP_MS`로 잘리지만 상한 비교는 **벽시계**다 — 틱 스레드가
        // 밀리면(절전 복귀 등) 거의 안 움직인 채 상한을 넘긴다. 그때 곧장 숨
        // 고르기로 가면 지상 동작이 되어 clamp가 y를 바닥으로 순간이동시킨다.
        let (mut p, t) = 발작하는_펭귄();
        // 공중에 있는 상태를 **직접 만든다** — 시드가 위로 뛰어 주기를 기다리면
        // 검사 대상(안전판)이 아니라 난수에 딸린 테스트가 된다
        p.y = BOUNDS.floor_y - 300.0;
        p.target = (p.x, BOUNDS.floor_y - 300.0);

        // 벽시계를 상한 한참 뒤로 건너뛴다
        let s = p.step(t + FREAKOUT_MS.1 * 5, &world());
        assert!(
            matches!(
                s.behavior,
                Behavior::Freakout { freakout: FreakoutPhase::Dash }
            ),
            "아직 내려오는 중이어야 한다 (실제: {:?})",
            s.behavior
        );
        assert!(
            s.y < BOUNDS.floor_y,
            "바닥으로 순간이동하면 안 된다 (실제: {})",
            s.y
        );

        // 그다음 틱들에서 평소 경로로 내려와 숨을 고른다
        let mut now = t + FREAKOUT_MS.1 * 5;
        while now < t + FREAKOUT_MS.1 * 10
            && matches!(
                p.behavior(),
                Behavior::Freakout { freakout: FreakoutPhase::Dash }
            )
        {
            now += 50;
            p.step(now, &world());
        }
        assert!(
            matches!(
                p.behavior(),
                Behavior::Freakout { freakout: FreakoutPhase::Pant }
            ),
            "내려와서 숨을 골라야 한다 (실제: {:?})",
            p.behavior()
        );
        assert_eq!(p.snapshot().y, BOUNDS.floor_y, "바닥까지 내려왔어야 한다");
    }

    #[test]
    fn 발작_중에_클릭하면_방망이를_휘두른다() {
        let (mut p, t) = 발작하는_펭귄();
        클릭(&mut p, t + 100);
        assert_eq!(p.behavior(), Behavior::Swing);
    }

    #[test]
    fn 발작_중에_들어_올릴_수_있다() {
        let (mut p, t) = 발작하는_펭귄();
        p.drag_start(t + 100);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 발작은_국면마다_고도가_다르다() {
        let 돌진 = Behavior::Freakout { freakout: FreakoutPhase::Dash };
        let 숨 = Behavior::Freakout { freakout: FreakoutPhase::Pant };
        assert!(돌진.is_airborne(), "사방으로 튀려면 떠 있어야 한다");
        assert!(!숨.is_airborne(), "숨은 바닥에서 고른다");
        assert!(!돌진.is_landing() && !숨.is_landing());
        assert!(돌진.moves_window() && 숨.moves_window());
    }

    #[test]
    fn 시키면_바로_발작한다() {
        let (p, _) = 발작하는_펭귄();
        assert!(matches!(p.behavior(), Behavior::Freakout { .. }));
    }

    #[test]
    fn 들려_있거나_이미_발작_중이면_시켜도_안_한다() {
        let mut p = pet();
        p.drag_start(1_000);
        assert!(!p.start_freakout(1_050), "손에 쥔 채로는 안 된다");

        let (mut q, t) = 발작하는_펭귄();
        assert!(!q.start_freakout(t + 100), "재진입하면 웹뷰가 되감지 못한다");
    }

    #[test]
    fn 세계가_좁아지면_밖에_있던_펭귄이_끌려_들어온다() {
        // **모니터를 뽑았을 때 펭귄을 되찾는 것이 이 clamp에 달려 있다.**
        // 브릿지가 사라진 모니터의 경계를 붙들고 있으면 이게 아무 일도 못 한다 —
        // 그쪽은 `world_to_cache`가 주 모니터로 떨어뜨려 막는다.
        let 넓은 = World::single(Bounds { left: 0.0, right: 3_000.0, ..BOUNDS });
        let mut p = Pet::new(7, 0, &넓은);
        // 사라질 모니터 쪽(오른쪽 끝)으로 옮긴다
        p.drag_start(100);
        p.drag_by(2_900.0, 0.0);
        p.drag_end(200, 0.0, 0.0, &넓은);
        let 멀리 = p.step(250, &넓은).x;
        assert!(멀리 > 1_500.0, "먼저 오른쪽 끝에 가 있어야 한다 (실제: {멀리})");

        // 모니터가 빠져 세계가 좁아졌다
        let 좁은 = World::single(BOUNDS);
        let s = p.step(300, &좁은);
        assert!(
            s.x <= BOUNDS.right,
            "좁아진 세계 안으로 들어와야 한다 (실제: {}, 상한: {})",
            s.x,
            BOUNDS.right
        );
    }

    #[test]
    #[ignore]
    fn 빈도_측정() {
        // 등급이 체감상 구분되는지 재 본다 (TODO "빈도 설계 재조정").
        // `cargo test --release 빈도_측정 -- --ignored --nocapture`
        let w = world();
        let 시간 = 4 * 60 * 60 * 1000u64; // 4시간
        let 시드들 = [1u64, 7, 42, 99, 12345];
        let mut 진입: std::collections::BTreeMap<String, u32> = Default::default();
        let mut 틱: std::collections::BTreeMap<String, u32> = Default::default();
        let mut 총틱 = 0u32;
        let mut 출처: std::collections::BTreeMap<String, u32> = Default::default();
        for seed in 시드들 {
            let mut p = Pet::new(seed, 0, &w);
            let mut 직전 = String::new();
            let mut t = 0u64;
            while t < 시간 {
                t += 50;
                let s = p.step(t, &w);
                let 이름 = match s.behavior {
                    Behavior::Idle { .. } => "Idle".to_string(),
                    Behavior::Sassy { .. } => "Sassy".to_string(),
                    Behavior::IceFishing { .. } => "IceFishing".to_string(),
                    Behavior::Freakout { .. } => "Freakout".to_string(),
                    other => format!("{other:?}"),
                };
                *틱.entry(이름.clone()).or_default() += 1;
                총틱 += 1;
                if 이름 != 직전 {
                    *진입.entry(이름.clone()).or_default() += 1;
                    if matches!(s.behavior, Behavior::Splat | Behavior::Sprawl | Behavior::Land) {
                        // **`직전`이다.** 예전에는 원인과 착지 사이에 `Falling`이
                        // 끼어 있어 `직전전`이 맞았는데, 그게 사라져 지금은
                        // `직전`이 원인이다 — 안 고치면 `Land ← Idle`처럼 찍혀
                        // 다음 감사가 엉뚱한 결론을 낸다.
                        *출처.entry(format!("{이름} ← {직전}")).or_default() += 1;
                    }
                    직전 = 이름;
                }
            }
        }
        let 총_시간 = 시간 as f64 / 3_600_000.0 * 시드들.len() as f64;
        println!("\n=== {총_시간:.0}시간 (시드 {}개 × 4시간) ===", 시드들.len());
        println!("{:<12} {:>8} {:>12} {:>9}", "동작", "진입", "시간당", "화면비율");
        let mut 줄: Vec<_> = 진입.iter().collect();
        줄.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (이름, n) in 줄 {
            let 시간당 = *n as f64 / 총_시간;
            let 간격 = if 시간당 > 0.0 { 3600.0 / 시간당 } else { f64::INFINITY };
            let 비율 = *틱.get(이름).unwrap_or(&0) as f64 / 총틱 as f64 * 100.0;
            println!("{이름:<12} {n:>8} {시간당:>9.1}/h {비율:>8.1}%   (평균 {간격:.0}초에 한 번)");
        }
        println!("\n--- 착지는 무엇 다음에 오나 ---");
        let mut 줄2: Vec<_> = 출처.iter().collect();
        줄2.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (경로, n) in 줄2.iter().take(10) {
            println!("{경로:<28} {n:>6}회  ({:.1}/h)", **n as f64 / 총_시간);
        }
    }

    #[test]
    fn 화면이_하나면_동작_수열이_그대로다() {
        // 좌표계가 화면 목록(`World`)으로 바뀌어도 화면이 하나면 판정이 달라지지
        // 않는다는 증거다.
        //
        // "경계 안에 있는가"만 보면 안 된다 — 판정 사각형이 좁아지거나 밀려도,
        // 심지어 기준점 계산이 망가져도 통과한다. 화면이 하나뿐이면 어떤 기준점이든
        // 결국 같은 화면으로 떨어지기 때문이다. 그래서 **수열을 통째로 못박는다.**
        //
        // 값은 **확률 갈래가 하나 늘 때마다** 다시 뜬다. 갈래는 난수를 하나 더
        // 뽑고, 그러면 그 뒤가 통째로 밀린다. 지금까지 **여섯 번** 재기준화했다 —
        // 벽 굴림(`hit_wall`), 얼음낚시, 슬라이딩, 발작(뒤 셋은 `pick_next`),
        // 헤엄의 종료를 자유낙하에서 내려앉기로 바꾼 것, 그리고 그 종료를 다시
        // **두 갈래로**(`SWIM_FREEFALL_PERCENT`) 나눈 것. 다섯 번째는 갈래가 는
        // 게 아니라 **뒤에 오는 동작이 달라져서** 난수 소비 시점이 밀린 경우다.
        // 전부 의도한 변경이다.
        // **동작을 늘리지 않았는데 이 배열이 흔들리면 그건 의도하지 않은 변경이다.**
        let w = world();
        let mut p = Pet::new(42, 0, &w);
        let seq = drive(&mut p, 0, 60_000, 50, &w);
        assert_eq!(seq.len(), 1_201);

        // (인덱스, 동작, x, y)
        let golden = [
            (0_usize, "Turn", 0.0, 800.0),
            (97, "Slide", 408.5, 800.0),
            (194, "Swim", 459.2, 701.3),
            // 헤엄이 90% 갈래로 나갔다 — 이 자리는 내려앉기를 넣기 전과 같은
            // `Falling`이다
            (291, "Falling", 394.8, 266.2),
            (388, "Idle { idle: Shake }", 394.8, 800.0),
            // **되돌린 결정의 대가가 여기 보인다** — 아무도 안 건드렸는데 철푸덕이다.
            // 266px 위에서 떨어졌으니 `SPLAT_MIN_IMPACT`를 넘는다
            (485, "Splat", 474.6, 800.0),
            (582, "Walk", 487.2, 800.0),
            (679, "Walk", 690.9, 800.0),
            (776, "Idle { idle: LookAround }", 714.0, 800.0),
            (873, "Swim", 427.5, 550.4),
            (970, "Falling", 151.8, 737.6),
            (1067, "Walk", 88.8, 800.0),
            (1164, "Idle { idle: LookAround }", 105.0, 800.0),
        ];
        for (i, behavior, x, y) in golden {
            let s = seq[i];
            assert_eq!(format!("{:?}", s.behavior), behavior, "{i}번째 동작");
            assert_eq!(format!("{:.1}", s.x), format!("{x:.1}"), "{i}번째 x");
            assert_eq!(format!("{:.1}", s.y), format!("{y:.1}"), "{i}번째 y");
        }

        // 사각형이 좁아지거나 밀리면 여기서 걸린다 — 펭귄은 실제로 양 끝과 바닥에
        // 닿는다. 이 시드는 60초 안에 오른쪽 끝까지 가지 않으므로 이 확인만 길게 돈다.
        // **길이는 취향이 아니라 필요다**: 수열이 재기준화되면 벽에 닿는 시점도
        // 함께 밀리므로, 넉넉하지 않으면 좌표계와 무관한 이유로 깨진다.
        let mut 오래 = Pet::new(42, 0, &w);
        let 긴_수열 = drive(&mut 오래, 0, 600_000, 50, &w);
        assert!(긴_수열.iter().any(|s| s.x == BOUNDS.left), "왼쪽 끝에 닿는다");
        assert!(긴_수열.iter().any(|s| s.x == BOUNDS.right), "오른쪽 끝에 닿는다");
        assert!(긴_수열.iter().any(|s| s.y == BOUNDS.floor_y), "바닥에 닿는다");
    }

    #[test]
    fn 발밑이_속한_화면의_바닥을_따른다() {
        let w = 두_화면();
        let mut p = Pet::new(7, 0, &w);
        p.x = 2_500.0;
        p.y = 900.0;
        let s = p.step(1_000, &w);
        assert_eq!(s.y, 900.0, "오른쪽 화면의 바닥을 따른다");
        assert_eq!(p.bounds_in(&w).floor_y, 900.0);
    }

    #[test]
    fn 새_펭귄은_x가_속한_화면에서_시작한다() {
        let w = 두_화면();
        let mut pets = Pets::new();
        let id = pets.add(7, 0, &w, 2_500.0).expect("추가된다");
        let s = pets.get(id).expect("있다").snapshot();
        assert_eq!(s.y, 900.0, "오른쪽 화면의 바닥에서 시작한다");
    }
}
