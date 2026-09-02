//! 테스트 공용 픽스처. 모듈이 비공개라 안의 `pub`은 `pet` 밖으로 안 나간다.

#![cfg(test)]

use super::*;

pub const BOUNDS: Bounds = Bounds {
    left: 0.0,
    right: 1000.0,
    top: 0.0,
    floor_y: 800.0,
};

/// 화면 하나짜리 세계. 대부분의 테스트는 지금까지처럼 한 화면만 본다.
pub fn world() -> World {
    World::single(BOUNDS)
}

/// 왼쪽 화면과, 그 오른쪽에 떨어져 놓인 화면. 사이에 빈 공간이 있다.
pub fn 두_화면() -> World {
    World::new(vec![
        Screen {
            id: 1,
            bounds: Bounds { left: 0.0, right: 1_000.0, top: 0.0, floor_y: 800.0 },
        },
        Screen {
            id: 2,
            bounds: Bounds { left: 2_000.0, right: 3_000.0, top: 100.0, floor_y: 900.0 },
        },
    ])
    .expect("화면이 둘이면 세계가 만들어진다")
}

pub fn pet() -> Pet {
    Pet::new(42, 0, &world())
}

/// `from`부터 `to`까지 `dt` 간격으로 진행시키며 스냅샷을 모은다.
pub fn drive(pet: &mut Pet, from: u64, to: u64, dt: u64, world: &World) -> Vec<Snapshot> {
    let mut out = Vec::new();
    let mut t = from;
    while t <= to {
        out.push(pet.step(t, world));
        t += dt;
    }
    out
}

/// **실제 클릭 한 번**을 흉내 낸다. 프론트는 클릭인지 드래그인지 알기 전에
/// 모든 pointerdown에서 `drag_start`를 부르므로, `whack`만 부르면 실제로는
/// 지나지 않는 경로를 테스트하게 된다.
pub fn 클릭(p: &mut Pet, now_ms: u64) {
    p.drag_start(now_ms);
    p.whack(now_ms, &world(), 0.0, 0.0);
}
