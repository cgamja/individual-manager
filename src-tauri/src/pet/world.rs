//! 펭귄이 다닐 수 있는 영역 — 화면 사각형과 그 목록.

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
    fn anchor_distance(self, ax: f64, ay: f64) -> f64 {
        let (_, _, y0, y1) = self.anchor_area();
        let dx = self.horizontal_distance(ax);
        let dy = (y0 - ay).max(ay - y1).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}

/// 펭귄이 노는 세계 — 연결된 화면 전부 (PRD §5.2).
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
#[path = "world_tests.rs"]
mod tests;
