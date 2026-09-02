//! 테스트 공용 픽스처 — 여러 모듈의 `mod tests`가 함께 쓴다.
//!
//! `pet.rs` 한 파일이던 시절에는 테스트도 하나였고 픽스처가 그 안에 있었다.
//! 모듈을 쪼개면서 형제들이 같은 세계·같은 펭귄을 필요로 하게 됐다.
//!
//! **모듈 자체가 비공개(`mod test_support;`)라 안의 `pub`은 `pet` 밖으로 안 나간다.**
//! 그래서 항목마다 `pub(super)`를 달 필요가 없다 — 경계는 모듈이 잡는다.

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
