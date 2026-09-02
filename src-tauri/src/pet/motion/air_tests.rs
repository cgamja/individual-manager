use crate::pet::test_support::*;
use crate::pet::*;
use crate::pet::motion::air::{clamp_throw, throw_max_speed};

/// 헤엄이 끝나며 어느 갈래로 나갔는지를 센다 — (자유낙하, 내려앉음).
fn 헤엄_종료_갈래() -> (u32, u32) {
    let (mut 낙하, mut 내려앉음) = (0, 0);
    for seed in 1u64..6 {
        let mut 직전 = Behavior::Walk;
        for s in 삼십분(seed) {
            if 직전 == Behavior::Swim && s.behavior != Behavior::Swim {
                if s.behavior == Behavior::Falling {
                    낙하 += 1;
                } else {
                    내려앉음 += 1;
                }
            }
            직전 = s.behavior;
        }
    }
    (낙하, 내려앉음)
}

#[test]
fn 헤엄은_대개_자유낙하로_끝난다() {
    let (낙하, 내려앉음) = 헤엄_종료_갈래();
    assert!(
        낙하 > 내려앉음 * 3,
        "낙하({낙하})가 내려앉기({내려앉음})보다 확실히 잦아야 한다"
    );
    assert!(내려앉음 > 0, "내려앉기 갈래가 죽었다 — 헤엄의 끝이 하나뿐이다");
}

#[test]
fn 저절로도_철푸덕하거나_널브러진다() {
    let 나온_시드: Vec<u64> = (1u64..6)
        .filter(|&seed| {
            삼십분(seed)
                .iter()
                .any(|s| matches!(s.behavior, Behavior::Splat | Behavior::Sprawl))
        })
        .collect();
    assert!(
        !나온_시드.is_empty(),
        "저절로 떨어져도 착지 등급이 안 갈린다 — 헤엄이 다시 얌전해졌나?"
    );
}

#[test]
fn 목적지가_바닥_근처여도_내려앉는_속도를_쓰지_않는다() {
    let w = world();
    let mut p = pet();
    p.step(1_000, &w);
    p.enter_swim(1_000, BOUNDS);
    p.x = 0.0;
    p.y = BOUNDS.floor_y - 3.0;
    p.target = (BOUNDS.right, BOUNDS.floor_y - 3.0);
    let 이동 = p.step(1_050, &w).x;
    assert!(
        이동 <= SWIM_SPEED * 0.05 + 0.01,
        "보통 헤엄인데 한 틱에 {이동:.2}px 움직였다 (헤엄 한 틱은 {:.2}px)",
        SWIM_SPEED * 0.05
    );
}

#[test]
fn 헤엄이_내려앉으면_바닥까지_내려와서_끝난다() {
    let w = world();
    let mut p = pet();
    p.step(1_000, &w);
    p.enter_swim(1_000, BOUNDS);
    p.y = BOUNDS.floor_y - 400.0;
    p.swim_descending = true;
    p.target = (p.x, BOUNDS.floor_y);
    let mut t = 1_000;
    while t < 1_000 + 60_000 && p.behavior() == Behavior::Swim {
        t += 50;
        p.step(t, &w);
    }
    assert_eq!(p.behavior(), Behavior::Land, "통, 하고 닿아야 한다");
    assert_eq!(
        p.snapshot().y,
        BOUNDS.floor_y,
        "바닥까지 내려와서 끝나야 한다"
    );
    assert!(!p.snapshot().air, "지상 상태로 끝나야 한다");
}

#[test]
fn 내려앉는_중에는_다시_추첨하지_않는다() {
    let w = world();
    let mut p = pet();
    p.step(1_000, &w);
    p.enter_swim(1_000, BOUNDS);
    p.y = BOUNDS.floor_y - 400.0;
    p.swim_descending = true;
    p.target = (p.x, BOUNDS.floor_y);
    p.behavior_until_ms = 1_000; // 예산 만료 — 도착 분기로 들어간다
    let 난수 = p.rng;
    p.step(1_050, &w);
    assert_eq!(p.behavior(), Behavior::Swim, "내려오다 자유낙하로 새면 안 된다");
    assert_eq!(p.rng, 난수, "추첨을 다시 하면 뒤 수열이 통째로 밀린다");
}

#[test]
    fn 헤엄을_치면_바닥에서_떠오른다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
        assert!(
            seen.iter().any(|s| s.behavior == Behavior::Swim),
            "가끔은 공중으로 떠야 한다"
        );
        let highest = seen.iter().map(|s| s.y).fold(f64::MAX, f64::min);
        assert!(
            highest < BOUNDS.floor_y - 50.0,
            "화면 위쪽을 쓰지 못했다 (최고점 {highest}, 바닥 {})",
            BOUNDS.floor_y
        );
    }

    #[test]
    fn 헤엄은_영역을_벗어나지_않는다() {
        let mut p = pet();
        for s in drive(&mut p, 100, 120_000, 100, &world()) {
            assert!(s.x >= BOUNDS.left && s.x <= BOUNDS.right, "x가 벗어났다: {}", s.x);
            assert!(s.y >= BOUNDS.top && s.y <= BOUNDS.floor_y, "y가 벗어났다: {}", s.y);
        }
    }

    #[test]
    fn 올라갈_때와_내려갈_때의_세로_방향이_다르다() {
        let mut p = pet();
        let seen = drive(&mut p, 100, 120_000, 100, &world());
        assert!(seen.iter().any(|s| s.vertical == Vertical::Up), "오르는 구간이 없다");
        assert!(seen.iter().any(|s| s.vertical == Vertical::Down), "내려가는 구간이 없다");
        for s in &seen {
            if !s.behavior.is_airborne() {
                assert_eq!(s.vertical, Vertical::Level, "지상인데 기울었다: {:?}", s.behavior);
            }
        }
    }

    #[test]
    fn 세게_던지면_포물선을_그린다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 700.0, -400.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown);

        let start_x = p.snapshot().x;
        let mut ys = Vec::new();
        let mut t = 1_100;
        while p.behavior() == Behavior::Thrown && t < 12_000 {
            t += 50;
            ys.push(p.step(t, &world()).y);
        }
        assert!(p.behavior().is_landing(), "결국 착지해야 한다");
        assert!(p.snapshot().x > start_x, "던진 방향으로 나아가야 한다");
        let peak = ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!(peak < ys[0], "위로 솟는 구간이 있어야 한다");
        assert!(*ys.last().unwrap() > peak, "다시 내려와야 한다");
    }

    #[test]
    fn 세게_던질수록_멀리_난다() {
        let throw = |vx: f64| {
            let mut p = pet();
            p.drag_start(1_000);
            p.drag_end(1_000, vx, -200.0, &world());
            let start = p.snapshot().x;
            let mut t = 1_000;
            while p.behavior() == Behavior::Thrown && t < 12_000 {
                t += 50;
                p.step(t, &world());
            }
            p.snapshot().x - start
        };
        assert!(throw(900.0) > throw(350.0), "세기에 비례해 더 멀리 가야 한다");
    }

    #[test]
    fn 살짝_놓으면_던지지_않고_제자리에서_떨어진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        let x = p.snapshot().x;
        p.drag_end(1_100, 20.0, 5.0, &world());
        assert_eq!(p.behavior(), Behavior::Falling);

        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 12_000 {
            t += 50;
            p.step(t, &world());
        }
        assert!((p.snapshot().x - x).abs() < 1.0, "좌우로 날아가면 안 된다");
    }

    #[test]
    fn 바닥보다_아래에서_위로_던져도_삼켜지지_않는다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, 90.0); // 바닥보다 90px 아래로 끌어내림
        p.step(1_050, &world());
        p.drag_end(1_100, 700.0, -400.0, &world()); // 오른쪽 위로 세게
        assert_eq!(p.behavior(), Behavior::Thrown);

        let first = p.step(1_150, &world());
        assert_eq!(
            first.behavior,
            Behavior::Thrown,
            "위로 던졌는데 첫 틱에 착지로 삼켜졌다"
        );
        assert!(first.y > BOUNDS.floor_y - 1.0, "위로 순간이동하면 안 된다");
    }

    /// 폭 1440 화면의 상한. KTD2의 비율(0.9)이 바뀌면 이 값도 함께 움직인다.
    fn 상한(width: f64) -> f64 {
        throw_max_speed(width)
    }

    /// 지정한 높이에서 떨어뜨려 착지 동작을 본다.
    fn 떨어뜨려_착지시킨다(drop_height: f64) -> Behavior {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -drop_height);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world()); // 살짝 놓는다 — 낙하만 시킨다
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        p.behavior()
    }

    #[test]
    fn 세게_떨어지면_철푸덕한다() {
        assert_eq!(떨어뜨려_착지시킨다(350.0), Behavior::Splat);
    }

    #[test]
    fn 아주_세게_떨어지면_널브러진다() {
        assert_eq!(떨어뜨려_착지시킨다(700.0), Behavior::Sprawl);
    }

    #[test]
    fn 살짝_떨어지면_그냥_선다() {
        assert_eq!(떨어뜨려_착지시킨다(5.0), Behavior::Land);
    }

    #[test]
    fn 어중간하게_떨어지면_통통_튄다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -60.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        let mut 닿았다 = false;
        let mut 다시_떠올랐다 = false;
        while p.behavior() == Behavior::Falling && t < 20_000 {
            t += 20;
            let s = p.step(t, &world());
            if s.y >= BOUNDS.floor_y {
                닿았다 = true;
            } else if 닿았다 {
                다시_떠올랐다 = true;
            }
        }
        assert!(닿았다 && 다시_떠올랐다, "바닥을 치고 다시 떠야 통통이다");
    }

    #[test]
    fn 통통은_몇_번_만에_멈춘다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() == Behavior::Falling && t < 12_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(p.behavior().is_landing(), "12초 안에 서야 한다 — {:?}", p.behavior());
    }

    #[test]
    fn 아래로_내리꽂으면_널브러진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -600.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 200.0, 900.0, &world()); // 아래로 세게
        let mut t = 1_100;
        while matches!(p.behavior(), Behavior::Thrown) && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert_eq!(p.behavior(), Behavior::Sprawl);
    }

    #[test]
    fn 던져서_세게_박아도_철푸덕한다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -600.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 300.0, 120.0, &world());
        let mut t = 1_100;
        while matches!(p.behavior(), Behavior::Thrown) && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(
            p.behavior().is_landing() && p.behavior() != Behavior::Land,
            "세게 박았으면 그냥 서면 안 된다 — {:?}",
            p.behavior()
        );
    }

    #[test]
    fn 철푸덕이_끝나면_평소_동작으로_돌아온다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -350.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() != Behavior::Splat && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        let 철푸덕_시작 = t;
        while p.behavior() == Behavior::Splat && t < 철푸덕_시작 + 10_000 {
            t += 20;
            p.step(t, &world());
        }
        assert_ne!(p.behavior(), Behavior::Splat, "영영 퍼져 있으면 안 된다");
        assert!(t - 철푸덕_시작 >= SPLAT_MS, "너무 빨리 일어난다");
    }

    #[test]
    fn 철푸덕_중에는_공중_상태가_아니다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -350.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        let mut t = 1_100;
        while p.behavior() != Behavior::Splat && t < 20_000 {
            t += 20;
            p.step(t, &world());
        }
        assert!(!p.snapshot().air);
    }

    #[test]
    fn 좁은_화면에서는_던지기_상한이_더_낮다() {
        let 좁은_곳 = 상한(1_440.0);
        let 넓은_곳 = 상한(2_880.0);
        assert!(
            (넓은_곳 - 좁은_곳 * 2.0).abs() < 1.0,
            "상한은 세계 폭에 비례해야 한다 — 좁은 곳 {좁은_곳}, 넓은 곳 {넓은_곳}"
        );
    }

    #[test]
    fn 상한_이하의_던지기는_속도가_그대로다() {
        let (vx, vy) = clamp_throw(400.0, -300.0, 1_440.0);
        assert!((vx - 400.0).abs() < f64::EPSILON);
        assert!((vy + 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn 상한은_방향을_유지한_채_속도만_줄인다() {
        let (vx, vy) = clamp_throw(30_000.0, -40_000.0, 1_440.0);
        let speed = (vx * vx + vy * vy).sqrt();
        assert!((speed - 상한(1_440.0)).abs() < 1.0, "상한까지 잘려야 한다");
        assert!((vx / speed - 0.6).abs() < 1e-6);
        assert!((vy / speed + 0.8).abs() < 1e-6);
    }

    #[test]
    fn 화면_폭을_읽지_못하면_기본_폭으로_상한을_잡는다() {
        let flat = World::single(Bounds {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            floor_y: 0.0,
        });
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_end(1_000, 900.0, -500.0, &flat);
        assert_eq!(p.behavior(), Behavior::Thrown, "던지기가 조용히 죽으면 안 된다");
    }

    #[test]
    fn 세계가_너무_좁아도_던지기_문턱_아래로_내려가지_않는다() {
        assert!(
            상한(100.0) >= THROW_MIN_SPEED,
            "상한이 최소 속도보다 낮으면 아무리 세게 던져도 던져지지 않는다"
        );
    }

    #[test]
    fn 던지기_문턱은_화면_폭이_달라져도_같다() {
        let 넓은_세계 = World::single(Bounds {
            left: 0.0,
            right: 4_000.0,
            ..BOUNDS
        });
        for w in [world(), 넓은_세계] {
            let mut p = pet();
            p.drag_start(1_000);
            p.drag_end(1_100, 20.0, 5.0, &w);
            assert_eq!(p.behavior(), Behavior::Falling, "살짝 놓으면 어디서든 떨어진다");
        }
    }

    #[test]
    fn 던지기_속도는_상한을_넘지_않는다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_end(1_000, 500_000.0, -500_000.0, &world());
        let first = p.step(1_050, &world());
        assert!(first.x <= BOUNDS.right && first.x >= BOUNDS.left);
        assert!(first.y >= BOUNDS.top && first.y <= BOUNDS.floor_y);
    }
