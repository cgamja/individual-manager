use crate::pet::test_support::*;
use crate::pet::*;

/// 얼음낚시 한 판을 처음부터 끝까지 돌린다.
fn 낚시_한_판(seed: u64) -> (Vec<FishingPhase>, Behavior, u64) {
    let w = world();
    let mut p = Pet::new(seed, 0, &w);
    p.enter_ice_fishing(0);
    let mut 국면 = Vec::new();
    let mut t = 0;
    loop {
        match p.step(t, &w).behavior {
            Behavior::IceFishing { fishing } => {
                if 국면.last() != Some(&fishing) {
                    국면.push(fishing);
                }
            }
            other => return (국면, other, t),
        }
        t += 50;
        assert!(t < 300_000, "시드 {seed}: 낚시가 끝나지 않는다");
    }
}

#[test]
fn 가끔_얼음낚시를_한다() {
    let 나온_시드: Vec<u64> = (1u64..7)
        .filter(|s| {
            삼십분(*s)
                .iter()
                .any(|s| matches!(s.behavior, Behavior::IceFishing { .. }))
        })
        .collect();
    assert!(
        !나온_시드.is_empty(),
        "30분을 돌려도 얼음낚시가 한 번도 안 나온다"
    );
}

#[test]
fn 얼음낚시는_드물다() {
    for seed in 1u64..7 {
        let 전체 = 삼십분(seed);
        let 낚시 = 전체
            .iter()
            .filter(|s| matches!(s.behavior, Behavior::IceFishing { .. }))
            .count();
        assert!(
            낚시 * 100 < 전체.len() * 30,
            "시드 {seed}: 30분 중 {낚시}/{} 이 낚시다",
            전체.len()
        );
    }
}

#[test]
fn 얼음낚시는_구멍_뚫기부터_시작한다() {
    let (국면, _, _) = 낚시_한_판(42);
    assert_eq!(국면.first(), Some(&FishingPhase::Dig));
}

#[test]
fn 구멍을_뚫고_나면_드리운다() {
    let (국면, _, _) = 낚시_한_판(42);
    assert_eq!(국면.get(1), Some(&FishingPhase::Wait), "{국면:?}");
}

#[test]
fn 입질_뒤에는_잡거나_꽝이다() {
    for seed in 1u64..40 {
        let (국면, _, _) = 낚시_한_판(seed);
        for (i, phase) in 국면.iter().enumerate() {
            if *phase != FishingPhase::Bite {
                continue;
            }
            let 다음 = 국면.get(i + 1);
            assert!(
                matches!(다음, Some(FishingPhase::Catch) | Some(FishingPhase::Miss)),
                "시드 {seed}: 입질 뒤가 {다음:?}다 — {국면:?}"
            );
        }
    }
}

#[test]
fn 꽝이면_다시_드리운다() {
    let mut 봤다 = false;
    for seed in 1u64..40 {
        let (국면, _, _) = 낚시_한_판(seed);
        for (i, phase) in 국면.iter().enumerate() {
            if *phase != FishingPhase::Miss {
                continue;
            }
            let 다음 = 국면.get(i + 1);
            if 다음 == Some(&FishingPhase::Wait) {
                봤다 = true;
            }
            assert!(
                matches!(다음, Some(FishingPhase::Wait) | Some(FishingPhase::Pack)),
                "시드 {seed}: 꽝 뒤가 {다음:?}다 — {국면:?}"
            );
        }
    }
    assert!(봤다, "꽝 뒤에 다시 드리우는 판이 하나도 없다");
}

#[test]
fn 물고기를_잡아도_예산이_남으면_다시_드리운다() {
    let mut 다시_드리운_적 = false;
    for seed in 1u64..40 {
        let (국면, _, _) = 낚시_한_판(seed);
        for (i, phase) in 국면.iter().enumerate() {
            if *phase != FishingPhase::Catch {
                continue;
            }
            let 다음 = 국면.get(i + 1);
            if 다음 == Some(&FishingPhase::Wait) {
                다시_드리운_적 = true;
            }
            assert!(
                matches!(다음, Some(FishingPhase::Wait) | Some(FishingPhase::Pack)),
                "시드 {seed}: 잡은 뒤가 {다음:?}다 — {국면:?}"
            );
        }
    }
    assert!(다시_드리운_적, "잡고 나서 다시 드리우는 판이 하나도 없다");
}

#[test]
fn 모든_판은_낚싯대를_접고_끝난다() {
    for seed in 1u64..40 {
        let (국면, _, _) = 낚시_한_판(seed);
        assert_eq!(
            국면.last(),
            Some(&FishingPhase::Pack),
            "시드 {seed}: {국면:?}"
        );
    }
}

#[test]
fn 얼음낚시_한_판은_예산_안에_끝난다() {
    let 상한 = FISHING_SESSION_MS.1
        + FISHING_WAIT_MS.1
        + FISHING_BITE_MS
        + FISHING_MISS_MS.max(FISHING_CATCH_MS)
        + FISHING_PACK_MS;
    for seed in 1u64..40 {
        let (국면, _, 끝) = 낚시_한_판(seed);
        assert!(
            끝 >= FISHING_SESSION_MS.0,
            "시드 {seed}: {끝}ms 만에 끝났다 — 예산보다 짧다 — {국면:?}"
        );
        assert!(끝 <= 상한, "시드 {seed}: {끝}ms 나 걸렸다 — {국면:?}");
    }
}

#[test]
fn 얼음낚시_중에는_위치가_변하지_않는다() {
    let w = world();
    let mut p = Pet::new(42, 0, &w);
    p.x = 400.0;
    p.enter_ice_fishing(0);
    let (시작_x, 시작_y) = (p.x, p.y);
    let mut t = 0;
    while let Behavior::IceFishing { .. } = p.step(t, &w).behavior {
        assert_eq!((p.x, p.y), (시작_x, 시작_y), "{t}ms 에서 움직였다");
        t += 50;
    }
}

#[test]
fn 얼음낚시가_끝나면_유휴로_간다() {
    for seed in 1u64..40 {
        let (국면, 뒤, _) = 낚시_한_판(seed);
        assert!(
            matches!(뒤, Behavior::Idle { .. }),
            "시드 {seed}: 낚시 뒤가 {뒤:?}다 — {국면:?}"
        );
    }
}

#[test]
fn 얼음낚시_중에_클릭하면_방망이를_휘두른다() {
    let mut p = pet();
    p.enter_ice_fishing(0);
    p.whack(300, &world(), 0.0, 0.0);
    assert_eq!(p.behavior(), Behavior::Swing);
}

#[test]
fn 얼음낚시_중에_들어_올릴_수_있다() {
    let mut p = pet();
    p.enter_ice_fishing(0);
    p.drag_start(300);
    assert_eq!(p.behavior(), Behavior::Dragged);
}

#[test]
fn 얼음낚시는_지상_동작이다() {
    let 낚시 = Behavior::IceFishing {
        fishing: FishingPhase::Wait,
    };
    assert!(!낚시.is_airborne());
    assert!(!낚시.is_landing(), "바닥에 닿아서 생긴 게 아니다");
    assert!(낚시.moves_window(), "틱이 느려지면 국면이 늦게 도착한다");

    let mut p = pet();
    p.enter_ice_fishing(0);
    let s = p.step(50, &world());
    assert!(!s.air);
    assert_eq!(s.y, BOUNDS.floor_y, "바닥에 앉는다");
}

#[test]
fn 시키면_바로_낚시를_시작한다() {
    let mut p = pet();
    assert!(p.start_fishing(1_000));
    assert_eq!(
        p.behavior(),
        Behavior::IceFishing {
            fishing: FishingPhase::Dig
        }
    );
}

#[test]
fn 이미_하는_중이면_다시_시켜도_받지_않는다() {
    let mut 낚시 = pet();
    assert!(낚시.start_fishing(1_000));
    assert!(!낚시.start_fishing(1_200), "낚시 중에 또 받았다");

    let mut 슬라이딩 = pet();
    슬라이딩.x = 400.0;
    assert!(슬라이딩.start_slide(1_000));
    assert!(!슬라이딩.start_slide(1_200), "미끄러지는 중에 또 받았다");
}

#[test]
fn 들려_있으면_시켜도_낚시하지_않는다() {
    let mut 들림 = pet();
    들림.drag_start(1_000);
    assert!(!들림.start_fishing(1_100));
    assert_eq!(들림.behavior(), Behavior::Dragged);
}

#[test]
fn 공중에서_시키면_허공에서_낚시한다() {
    let w = world();
    let mut p = pet();
    p.air = true;
    p.y = 300.0;
    assert!(p.start_fishing(1_000));

    let s = p.step(1_050, &w);
    assert!(matches!(s.behavior, Behavior::IceFishing { .. }));
    assert!(s.air, "고도를 잃으면 안 된다");
    assert_eq!(s.y, 300.0, "그 높이에 그대로 앉는다");
}

#[test]
fn 허공에서_낚시가_끝나면_떨어진다() {
    let w = world();
    let mut p = pet();
    p.air = true;
    p.y = 300.0;
    assert!(p.start_fishing(0));
    let 동작: Vec<Behavior> = drive(&mut p, 0, 120_000, 50, &w)
        .iter()
        .map(|s| s.behavior)
        .collect();
    let 낚시_뒤 = 동작
        .iter()
        .skip_while(|b| matches!(b, Behavior::IceFishing { .. }))
        .next()
        .copied();
    assert_eq!(낚시_뒤, Some(Behavior::Falling), "{:?}", &동작[..5]);
}

#[test]
fn 시켜서_시작한_낚시_중에는_졸지_않는다() {
    let w = world();
    let mut p = Pet::new(42, 0, &w);
    let 시작 = SLEEP_AFTER_MS + 10_000;
    assert!(p.start_fishing(시작));
    let 동작: Vec<Behavior> = drive(&mut p, 시작, 시작 + 90_000, 100, &w)
        .iter()
        .map(|s| s.behavior)
        .collect();
    assert!(!동작.contains(&Behavior::Sleep), "낚시하다 졸았다");
}

#[test]
fn 공중에서는_얼음낚시를_시작하지_않는다() {
    for seed in 1u64..200 {
        let mut p = Pet::new(seed, 0, &world());
        p.air = true;
        for i in 0..40u64 {
            p.pick_next(i * 100, BOUNDS);
            assert!(
                !matches!(p.behavior, Behavior::IceFishing { .. }),
                "시드 {seed}: 공중에서 낚시를 시작했다"
            );
            p.air = true;
        }
    }
}
