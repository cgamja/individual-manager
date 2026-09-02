//! 펭귄이 노는 세계의 좌표계 — [`Bounds`] · [`Screen`] · [`World`].
//!
//! 브릿지가 화면의 기하를 재서 만들고, 코어는 여기 담긴 사각형만 보고 판정한다.
//! 창 크기 보정(펭귄 좌상단 ↔ 발밑 기준점)은 이 모듈이 소유한다.
//!
//! **프로덕션은 화면 하나만 담는다.** 브릿지가 `World::single`만 만들고
//! `available_monitors()`를 한 번도 부르지 않는다 — 모니터 경계 넘기가
//! 2026-08-31에 범위 밖으로 빠졌기 때문이다(PRINCIPLE 개정 이력 v3.2).
//! `screen_at`·`nearest`·`screen_for_x`의 여러 화면 경로는 아래 테스트만
//! 붙들고 있고, 걷어낼지는 `TODO.md`에 열린 항목으로 남아 있다.

use super::tuning::PET_SIZE;

/// 펭귄이 돌아다닐 수 있는 영역 (논리 좌표). `left`/`right`는 창의 좌상단 x가
/// 가질 수 있는 최소·최대값이고, `top`은 올라갈 수 있는 최고점,
/// `floor_y`는 바닥에 섰을 때의 y다.
/// 창 크기 보정은 이 값을 만드는 쪽(브릿지)이 이미 끝낸 상태로 넘긴다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bounds {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub floor_y: f64,
}

/// 화면 식별자. 브릿지가 화면의 기하(위치·크기)에서 만든다 — macOS의
/// `Monitor::name()`은 모델 번호라 같은 모델 두 대를 구분하지 못하고, 고유값인
/// `CGDirectDisplayID`는 Tauri가 노출하지 않는다 (플랜 KTD2).
pub type ScreenId = u64;

/// 화면 하나. 배치가 바뀌면 `id`도 바뀌는데, 그건 버그가 아니라 "배치가 바뀌었다"는
/// 신호로 쓴다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Screen {
    pub id: ScreenId,
    pub bounds: Bounds,
}

impl Screen {
    /// 기준점(펭귄 발밑 중앙)이 이 화면 위에 있다고 볼 범위 — `(x0, x1, y0, y1)`.
    ///
    /// [`Bounds`]는 **펭귄 좌상단**의 범위라 기준점 기준으로는 그만큼 밀려 있다.
    /// 창 크기를 이미 빼 놓은 값이므로, 화면 둘이 실제로 맞닿아 있어도 이 범위
    /// 사이에는 **창 하나만큼 빈틈이 남는다.** 화면이 하나뿐인 지금은 드러나지
    /// 않지만, 경계를 넘나들게 만들 때는 그 빈틈을 메워야 한다.
    fn anchor_area(self) -> (f64, f64, f64, f64) {
        (
            self.bounds.left + PET_SIZE / 2.0,
            self.bounds.right + PET_SIZE / 2.0,
            self.bounds.top + PET_SIZE,
            self.bounds.floor_y + PET_SIZE,
        )
    }

    /// 기준점이 이 화면의 가로 범위에서 얼마나 벗어났나. 안이면 0이다.
    fn horizontal_distance(self, ax: f64) -> f64 {
        let (x0, x1, _, _) = self.anchor_area();
        (x0 - ax).max(ax - x1).max(0.0)
    }

    /// 기준점에서 이 화면까지의 거리. 화면 위에 있으면 0이다.
    ///
    /// 안일 때 **정확히** `0.0`이 나오는 것이 [`World::screen_at`]의 포함 판정을
    /// 떠받친다 — `dx`/`dy`가 각각 `max(0.0)`을 거치므로 부동소수 오차가 낄 자리가
    /// 없다. 여기에 클램프되지 않은 항을 더하면 그 성질이 깨진다.
    fn anchor_distance(self, ax: f64, ay: f64) -> f64 {
        let (_, _, y0, y1) = self.anchor_area();
        let dx = self.horizontal_distance(ax);
        let dy = (y0 - ay).max(ay - y1).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}

/// 펭귄이 노는 세계 — 연결된 화면 전부 (PRD §5.2).
///
/// **불변식: 비어 있지 않다.** 화면이 하나도 없는 세계에는 펭귄이 있을 자리가 없고,
/// 그 상태를 표현할 수 있게 두면 모든 판정에 `Option`이 번진다. 그래서 생성자에서
/// 한 번만 막는다.
#[derive(Clone, PartialEq, Debug)]
pub struct World {
    screens: Vec<Screen>,
}

impl World {
    /// 화면 목록에서 세계를 만든다. 비어 있으면 `None`.
    pub fn new(screens: Vec<Screen>) -> Option<Self> {
        if screens.is_empty() {
            None
        } else {
            Some(World { screens })
        }
    }

    /// 화면 하나짜리 세계. 모니터를 못 읽었을 때와 테스트에서 쓴다.
    pub fn single(bounds: Bounds) -> Self {
        World {
            screens: vec![Screen { id: 0, bounds }],
        }
    }

    /// 목록의 첫 화면. 비어 있지 않다는 불변식 덕에 항상 있다.
    pub fn first(&self) -> Screen {
        self.screens[0]
    }

    /// 기준점이 놓인 화면. 어느 화면에도 없으면 `None`.
    pub fn screen_at(&self, ax: f64, ay: f64) -> Option<Screen> {
        self.screens
            .iter()
            .copied()
            .find(|s| s.anchor_distance(ax, ay) == 0.0)
    }

    /// 기준점에서 가장 가까운 화면. 세계가 비어 있지 않으므로 항상 있다.
    pub fn nearest(&self, ax: f64, ay: f64) -> Screen {
        let mut best = self.screens[0];
        let mut best_d = best.anchor_distance(ax, ay);
        for s in &self.screens[1..] {
            let d = s.anchor_distance(ax, ay);
            if d < best_d {
                best = *s;
                best_d = d;
            }
        }
        best
    }

    /// 펭귄 x좌표가 놓일 화면. 새 펭귄을 어디에 만들지 정할 때 쓴다.
    /// 세로는 아직 정해지지 않았으므로 가로만 본다.
    pub fn screen_for_x(&self, x: f64) -> Screen {
        let ax = x + PET_SIZE / 2.0;
        let mut best = self.screens[0];
        let mut best_d = f64::INFINITY;
        for s in &self.screens {
            let d = s.horizontal_distance(ax);
            if d < best_d {
                best = *s;
                best_d = d;
            }
        }
        best
    }

    /// 세계 전체의 가로 폭 — 던지기 상한이 여기에 비례한다 (KTD7).
    /// 화면이 하나면 그 화면의 이동 폭과 같다.
    pub fn width(&self) -> f64 {
        let left = self
            .screens
            .iter()
            .map(|s| s.bounds.left)
            .fold(f64::INFINITY, f64::min);
        let right = self
            .screens
            .iter()
            .map(|s| s.bounds.right)
            .fold(f64::NEG_INFINITY, f64::max);
        (right - left).max(0.0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::test_support::*;

    #[test]
    fn 빈_화면_목록으로는_세계를_만들_수_없다() {
        assert!(World::new(vec![]).is_none(), "펭귄이 있을 자리가 없다");
    }

    #[test]
    fn 발밑이_속한_화면을_찾는다() {
        let w = 두_화면();
        // 왼쪽 화면 한복판에 선 펭귄
        let left = (500.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.screen_at(left.0, left.1).map(|s| s.id), Some(1));
        // 오른쪽 화면 한복판에 선 펭귄
        let right = (2_500.0 + PET_SIZE / 2.0, 900.0 + PET_SIZE);
        assert_eq!(w.screen_at(right.0, right.1).map(|s| s.id), Some(2));
    }

    #[test]
    fn 화면_사이_빈_공간에는_화면이_없다() {
        let w = 두_화면();
        let gap = (1_500.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert!(w.screen_at(gap.0, gap.1).is_none());
    }

    #[test]
    fn 발밑이_어느_화면에도_없으면_가장_가까운_화면을_준다() {
        let w = 두_화면();
        // 왼쪽 화면 바로 오른쪽 — 1번이 가깝다
        let near_left = (1_100.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.nearest(near_left.0, near_left.1).id, 1);
        // 오른쪽 화면 바로 왼쪽 — 2번이 가깝다
        let near_right = (1_900.0 + PET_SIZE / 2.0, 800.0 + PET_SIZE);
        assert_eq!(w.nearest(near_right.0, near_right.1).id, 2);
    }

    #[test]
    fn 세계_폭은_화면_전체를_덮는다() {
        assert_eq!(두_화면().width(), 3_000.0);
        assert_eq!(
            World::single(BOUNDS).width(),
            BOUNDS.right - BOUNDS.left,
            "화면이 하나면 그 화면의 이동 폭과 같다"
        );
    }

    #[test]
    fn 화면_판정_범위는_기준점만큼_밀려_있다() {
        let w = world();
        // 좌상단이 left에 있는 펭귄의 **발밑**은 left + PET_SIZE/2, floor_y + PET_SIZE에 있다
        assert!(w
            .screen_at(BOUNDS.left + PET_SIZE / 2.0, BOUNDS.floor_y + PET_SIZE)
            .is_some());
        // 보정하지 않고 좌상단 좌표로 물으면 화면 위가 아니다.
        // 기준점 보정이 빠지면 이 단언이 먼저 깨진다.
        assert!(w.screen_at(BOUNDS.left, BOUNDS.top).is_none());
    }

    #[test]
    fn 정확히_같은_거리면_앞_화면이_이긴다() {
        let w = 두_화면();
        // 두 화면의 기준점 가로 범위는 [70,1070]과 [2070,3070] — 그 한가운데
        let mid = (1_070.0 + 2_070.0) / 2.0;
        assert_eq!(w.nearest(mid, 800.0 + PET_SIZE).id, 1, "동거리면 목록 앞이 이긴다");
    }

    #[test]
    fn 틈에_놓인_x는_가까운_화면으로_간다() {
        let w = 두_화면();
        assert_eq!(w.screen_for_x(1_100.0).id, 1, "왼쪽 화면에 더 가깝다");
        assert_eq!(w.screen_for_x(1_950.0).id, 2, "오른쪽 화면에 더 가깝다");
    }

    #[test]
    fn 폭이_0인_화면이_섞여도_세계_폭은_전체를_덮는다() {
        let w = World::new(vec![
            Screen {
                id: 1,
                bounds: Bounds { left: 0.0, right: 0.0, top: 0.0, floor_y: 800.0 },
            },
            Screen {
                id: 2,
                bounds: Bounds { left: 2_000.0, right: 3_000.0, top: 0.0, floor_y: 800.0 },
            },
        ])
        .expect("화면이 둘이면 세계가 만들어진다");
        assert_eq!(w.width(), 3_000.0);
    }}
