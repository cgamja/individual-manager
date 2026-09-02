use crate::pet::test_support::*;
use crate::pet::*;

// ── 얼음낚시 ──────────────────────────────────────────────────
//
// 이 앱에서 가장 긴 동작이고, **안에서 갈래가 갈리는 첫 동작**이다
// (잡음/꽝). 그래서 "무슨 국면을 거쳤는가"를 통째로 뽑아 놓고 규칙을 건다 —
// 국면마다 펭귄을 따로 만들면 갈래가 늘 때마다 준비 코드가 갈라진다.

/// 얼음낚시 한 판을 처음부터 끝까지 돌린다.
///
/// 거쳐 간 국면(연속 중복은 접는다), 끝난 뒤의 동작, 끝난 시각을 돌려준다.
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

/// 30분치를 돌려 스냅샷을 모은다. 얼음낚시는 십 분에 한 번쯤이라
/// 짧게 돌리면 한 번도 안 나온다.
fn 삼십분(seed: u64) -> Vec<Snapshot> {
    let w = world();
    let mut p = Pet::new(seed, 0, &w);
    drive(&mut p, 100, 30 * 60_000, 100, &w)
}

/// 헤엄이 끝나며 어느 갈래로 나갔는지를 센다 — (자유낙하, 내려앉음).
///
/// 내려앉기는 `Swim`을 유지한 채 목적지만 바닥으로 갈아 끼우므로, 갈래는
/// **헤엄에서 빠져나가는 동작**으로 구분된다: `Falling`이면 떨어진 것이고
/// 그 밖(`Land`)이면 날개를 저어 내려온 것이다.
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

/// 핀볼 모드를 켠 펭귄. 켜는 것은 설정이지만 코어에서는 필드 하나다.
fn 핀볼_펫() -> Pet {
    let mut p = pet();
    p.set_pinball(true);
    p
}

/// 지정한 높이에서 떨어뜨리고, 바닥에 닿은 횟수와 마지막 동작을 센다.
fn 떨어뜨려_세기(p: &mut Pet, 높이: f64, 한계_ms: u64) -> (u32, Behavior) {
    let w = world();
    p.step(0, &w);
    p.y = BOUNDS.floor_y - 높이;
    p.vy = 0.0;
    p.enter(Behavior::Falling, 0);
    let (mut 바닥_접촉, mut 떠_있었나) = (0, true);
    let mut t = 0;
    while t < 한계_ms {
        t += 50;
        let s = p.step(t, &w);
        let 바닥에 = s.y >= BOUNDS.floor_y - 0.01;
        if 바닥에 && 떠_있었나 {
            바닥_접촉 += 1;
        }
        떠_있었나 = !바닥에;
    }
    (바닥_접촉, p.behavior())
}

#[test]
fn 핀볼이면_세게_떨어져도_널브러지지_않는다() {
    // **착지 등급 판정을 통째로 우회한다** — 철푸덕·널브러짐이 있던 속도
    // 구간을 통통이 흡수한다. 지우는 게 아니라 가려두는 것이다.
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    // 널브러짐 문턱(1000px/s)을 확실히 넘는 높이
    p.y = BOUNDS.floor_y - 700.0;
    p.enter(Behavior::Falling, 0);
    let mut t = 0;
    while t < 30_000 {
        t += 50;
        let s = p.step(t, &w);
        assert!(
            !matches!(s.behavior, Behavior::Splat | Behavior::Sprawl),
            "핀볼인데 {:?}가 나왔다",
            s.behavior
        );
    }
}

#[test]
fn 핀볼을_끄면_착지_네_갈래가_그대로다() {
    // **모드를 끄면 원래대로 돌아온다**(R4). 위 테스트와 같은 높이인데
    // 결과가 달라야 한다 — 아니면 가려두는 게 아니라 지운 것이다.
    let w = world();
    let mut p = pet();
    p.step(0, &w);
    p.y = BOUNDS.floor_y - 700.0;
    p.enter(Behavior::Falling, 0);
    let mut 널브러졌나 = false;
    let mut t = 0;
    while t < 30_000 {
        t += 50;
        if p.step(t, &w).behavior == Behavior::Sprawl {
            널브러졌나 = true;
        }
    }
    assert!(널브러졌나, "핀볼을 껐는데도 널브러지지 않았다");
}

#[test]
fn 핀볼이면_바닥에서_한참_튄다() {
    // 감쇠가 거의 없어야 "계속 튕긴다"로 읽힌다. **횟수를 못박지 않는다** —
    // `PINBALL_DAMPING`은 취향 상수라 조정될 값이다. 평소보다 훨씬 오래
    // 튄다는 것만 본다.
    let (핀볼_접촉, _) = 떨어뜨려_세기(&mut 핀볼_펫(), 500.0, 60_000);
    let (평소_접촉, _) = 떨어뜨려_세기(&mut pet(), 500.0, 60_000);
    assert!(
        핀볼_접촉 >= 10,
        "핀볼인데 {핀볼_접촉}번밖에 안 튀었다"
    );
    assert!(
        핀볼_접촉 > 평소_접촉 * 2,
        "핀볼({핀볼_접촉})이 평소({평소_접촉})보다 확연히 오래 튀어야 한다"
    );
}

#[test]
fn 핀볼이라도_결국_선다() {
    // **아래 문턱(`BOUNCE_MIN_SPEED`)을 남긴 이유다.** 없애면 영원히
    // 잔진동하며 다시는 걷지 않고, 20Hz 틱도 영영 안 쉰다.
    let (_, 마지막) = 떨어뜨려_세기(&mut 핀볼_펫(), 500.0, 120_000);
    assert!(
        !matches!(마지막, Behavior::Falling | Behavior::Thrown),
        "2분이 지나도 안 멈춘다 ({마지막:?})"
    );
}

#[test]
fn 핀볼이면_벽에서도_거의_안_죽는다() {
    // 핀볼에서 벽은 범퍼다 — 바닥과 같은 계수를 쓴다.
    let w = world();
    let 남은_속도 = |핀볼: bool| {
        let mut p = pet();
        p.set_pinball(핀볼);
        p.step(0, &w);
        p.x = BOUNDS.right - 1.0;
        p.y = BOUNDS.floor_y - 400.0;
        p.vx = 600.0;
        p.vy = 0.0;
        p.enter(Behavior::Thrown, 0);
        let mut t = 0;
        // 벽에 닿아 되튈 때까지
        while t < 3_000 && p.vx > 0.0 {
            t += 50;
            p.step(t, &w);
        }
        p.vx.abs()
    };
    let 핀볼 = 남은_속도(true);
    let 평소 = 남은_속도(false);
    assert!(
        핀볼 > 평소 * 1.5,
        "핀볼({핀볼:.0})이 평소({평소:.0})보다 훨씬 덜 죽어야 한다"
    );
}

/// 핀볼 펭귄을 한 지점에서 친다. 반환은 그 직후 스냅샷.
fn 쳐본다(nx: f64, ny: f64) -> (Pet, Snapshot) {
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    p.whack(1_000, &w, nx, ny);
    let s = p.snapshot();
    (p, s)
}

#[test]
fn 핀볼에서_아래를_치면_위로_날아간다() {
    // 채는 맞은 지점에서 **중심 쪽으로** 민다 — 아래를 치면 위로 뜬다.
    let (p, s) = 쳐본다(0.0, 0.4);
    assert_eq!(s.behavior, Behavior::Thrown, "쳤으면 날아가야 한다");
    assert!(p.vy < 0.0, "아래를 쳤는데 위로 안 간다 (vy={})", p.vy);
}

#[test]
fn 핀볼에서_왼쪽을_치면_오른쪽으로_간다() {
    let (p, _) = 쳐본다(-0.4, 0.0);
    assert!(p.vx > 0.0, "왼쪽을 쳤는데 오른쪽으로 안 간다 (vx={})", p.vx);
    assert_eq!(p.snapshot().facing, Facing::Right, "가는 쪽을 봐야 한다");
}

#[test]
fn 핀볼에서_정중앙을_치면_위로_뜬다() {
    // 방향 벡터의 길이가 0이다 — 0으로 나누면 NaN이 되어 펭귄이 사라진다.
    let (p, s) = 쳐본다(0.0, 0.0);
    assert_eq!(s.behavior, Behavior::Thrown);
    assert!(p.vy < 0.0, "정중앙을 쳤는데 안 뜬다 (vy={})", p.vy);
    assert!(p.vx.is_finite() && p.vy.is_finite(), "속도가 NaN이다");
}

#[test]
fn 핀볼에서_치는_세기는_세계_폭을_따른다() {
    // 던지기 상한과 같은 근거다(KTD7) — 좁은 화면에서 눈 깜짝할 새
    // 가로지르면 안 된다.
    let 세기 = |폭: f64| {
        let w = World::single(Bounds { left: 0.0, right: 폭, top: 0.0, floor_y: 800.0 });
        let mut p = Pet::new(42, 0, &w);
        p.set_pinball(true);
        p.step(0, &w);
        p.whack(1_000, &w, 0.0, 0.4);
        p.vy.abs()
    };
    assert!(세기(2_000.0) > 세기(500.0) * 2.0, "세계가 넓으면 더 세게 쳐야 한다");
}

#[test]
fn 핀볼에서는_방망이를_휘두르지_않는다() {
    // 핀볼에서 방망이는 펭귄이 아니라 **커서**가 들고 있다.
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    let 전 = p.snapshot().whack_seq;
    p.whack(1_000, &w, 0.0, 0.4);
    assert_eq!(p.snapshot().whack_seq, 전, "핀볼인데 스윙 횟수가 늘었다");
}

#[test]
fn 핀볼에서_스무_번_쳐도_빽빽대지_않는다() {
    // 핀볼에서 연타는 **정상적인 랠리**다. 제자리에 멈춰 화를 내면 판이 끊긴다.
    let w = world();
    let mut p = 핀볼_펫();
    p.step(0, &w);
    let mut t = 1_000;
    for _ in 0..(SQUAWK_WHACK_COUNT + 5) {
        p.whack(t, &w, 0.0, 0.4);
        assert_ne!(p.behavior(), Behavior::Squawk, "핀볼인데 빽빽댄다");
        t += 300;
    }
}

#[test]
fn 핀볼을_끄면_클릭이_빠따다() {
    // 좌표를 줘도 모드가 꺼져 있으면 제자리에서 휘두른다 (회귀 가드).
    let w = world();
    let mut p = pet();
    p.step(0, &w);
    p.whack(1_000, &w, -0.4, 0.4);
    assert_eq!(p.behavior(), Behavior::Swing, "빠따가 아니다");
    assert_eq!((p.vx, p.vy), (0.0, 0.0), "빠따는 날아가지 않는다");
    assert_eq!(p.snapshot().whack_seq, 1, "스윙 횟수가 안 늘었다");
}

#[test]
fn 헤엄은_대개_자유낙하로_끝난다() {
    // **2026-09-01 사용자 지시로 되돌린 결정이다.** `93d419a`는 헤엄이 끝날
    // 때마다 날개를 저어 내려앉게 만들어 저절로 생기는 철푸덕·널브러짐을 0으로
    // 만들었는데, 그러면서 **하늘에서 떨어지는 그림도 함께 사라졌다.**
    // 지금은 `SWIM_FREEFALL_PERCENT`만큼 떨어지고 나머지는 내려앉는다.
    //
    // **비율을 정확히 못박지 않는다** — 표본이 작으면 흔들린다. 방향만 본다:
    // 낙하가 다수이고, 내려앉기 갈래가 살아 있다.
    let (낙하, 내려앉음) = 헤엄_종료_갈래();
    assert!(
        낙하 > 내려앉음 * 3,
        "낙하({낙하})가 내려앉기({내려앉음})보다 확실히 잦아야 한다"
    );
    assert!(내려앉음 > 0, "내려앉기 갈래가 죽었다 — 헤엄의 끝이 하나뿐이다");
}

#[test]
fn 저절로도_철푸덕하거나_널브러진다() {
    // 위 갈래의 **대가**다. 예전에는 "손을 안 대면 세게 부딪힐 일이 없다"가
    // 규칙이었지만, 하늘에서 떨어지는 그림을 되찾으면 그 높이만큼의 착지가
    // 함께 돌아온다. **받아들인 대가라 테스트로 못박는다** — 조용히
    // 사라지거나 조용히 돌아오면 안 된다.
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
    // **"내려오는 중인가"를 목적지 y로 추론하면 안 된다** — `enter_swim`이
    // 우연히 바닥 6px 안쪽을 목적지로 뽑은 **보통 헤엄**(실측 0.53%, 시간당
    // 한 번쯤)이 통째로 2.2배 속도로 날아간다.
    let w = world();
    let mut p = pet();
    // 먼저 한 틱 진행시켜 last_step_ms를 맞춘다 — 안 그러면 dt가 상한
    // (MAX_STEP_MS)으로 잡혀 한 틱 이동량이 다섯 배가 된다
    p.step(1_000, &w);
    p.enter_swim(1_000, BOUNDS);
    // 바닥 바로 위에서 가로로만 이동하게 둔다
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
    // 내려앉는 동안에도 날개를 젓는다 — `MOTIONS.md`가 적어 둔 "내려앉음"이다.
    // **갈래를 추첨에 맡기지 않고 못박는다**: 90%가 낙하로 빠지므로 그냥
    // 돌리면 이 경로를 거의 안 밟는다.
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
    // 내려오는 도중에 예산이 만료되면 목적지만 다시 찍고 계속 내려온다.
    // 여기서 갈래를 다시 뽑으면 **난수를 태워** 뒤 수열이 통째로 밀리고,
    // 절반쯤 내려온 펭귄이 갑자기 자유낙하로 새서 갈래도 무의미해진다.
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
fn 빈도_등급이_순서대로다() {
    // **모션이 늘어날수록 하나하나가 희석된다** — 등급이 조용히 뒤집히는 것을
    // 막는 가드다 (`MOTIONS.md` "빈도 설계"). 절대값이 아니라 **순서**를 잰다:
    // 값은 취향이라 바뀌지만 "기본 > 자주 > 가끔"은 등급의 정의 자체다.
    //
    // 실측 근거는 `빈도_측정`(`#[ignore]`)으로 다시 뜰 수 있다.
    let mut 기본 = 0; // Walk·Idle
    let mut 자주 = 0; // Swim
    let mut 가끔 = 0; // IceFishing
    for seed in 1u64..5 {
        let 전체 = 삼십분(seed);
        let mut 직전 = String::new();
        for s in 전체 {
            let 이름 = match s.behavior {
                Behavior::Walk | Behavior::Idle { .. } => "기본",
                Behavior::Swim => "자주",
                Behavior::IceFishing { .. } => "가끔",
                _ => "",
            }
            .to_string();
            if !이름.is_empty() && 이름 != 직전 {
                match 이름.as_str() {
                    "기본" => 기본 += 1,
                    "자주" => 자주 += 1,
                    _ => 가끔 += 1,
                }
            }
            직전 = 이름;
        }
    }
    assert!(기본 > 자주, "기본({기본})이 자주({자주})보다 잦아야 한다");
    assert!(자주 > 가끔, "자주({자주})가 가끔({가끔})보다 잦아야 한다");
}

#[test]
fn 희귀는_가끔보다_두_자릿수_드물다() {
    // 발작은 시뮬레이션으로 재기엔 너무 드물어서(수십 시간) 상수로 비교한다.
    // 얼음낚시는 `range((0,999)) < 7`이므로 사이클당 7/1000이다.
    let 가끔 = ICE_FISHING_PERMILLE as f64 / 1_000.0;
    let 희귀 = 1.0 / FREAKOUT_ONE_IN as f64;
    assert!(
        가끔 / 희귀 >= 100.0,
        "발작(1/{})이 얼음낚시({}‰)보다 두 자릿수 드물지 않다",
        FREAKOUT_ONE_IN,
        ICE_FISHING_PERMILLE
    );
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
    // 자주 나오면 "가끔 보여서 반가운" 동작이 아니라 기본 동작이 된다
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
                matches!(
                    다음,
                    Some(FishingPhase::Catch) | Some(FishingPhase::Miss)
                ),
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
            // 예산이 다 됐으면 다시 드리우지 않고 접는다
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
    // 잡을 때마다 판을 끝내면 길이가 40% 확률에 좌우돼 중앙값이 20초 아래로
    // 내려간다 — 졸기보다 짧아지면 "가장 긴 동작"이 아니다
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
    // 앉은 자세에서 곧장 유휴로 가면 눌림이 한 프레임 만에 사라져 튄다
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
    // 국면 도중에 자르지 않으므로 상한은 예산 + 마지막 한 바퀴다.
    // 무한히 도는 판을 잡는 게 이 테스트의 목적이다.
    // 판을 끝내는 것은 **예산 하나뿐**이므로 하한이 예산의 하한이다.
    // 잡았다고 끝나던 때는 이 단언이 성립하지 않았고, 실제 중앙값이
    // 18.6초까지 내려가 있었다 (리뷰 실측).
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
    // 넘어졌다 일어난 뒤(get_up)와 출구를 공유하지 않는다 — 30초 얌전히
    // 앉아 있다 갑자기 약을 올리는 건 결이 다르다
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
    // 창은 제자리지만 틱은 빠르게 유지해야 한다 — 느려지면 0.7초짜리
    // 입질 국면이 최대 0.5초 늦게 도착한다
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
fn 시키면_바로_미끄러진다() {
    let mut p = pet();
    p.x = 400.0;
    assert!(p.start_slide(1_000));
    assert_eq!(p.behavior(), Behavior::Slide);
    let 뒤 = p.step(1_200, &world());
    assert_ne!(뒤.x, 400.0, "시켰는데 제자리다");
}

#[test]
fn 공중이거나_들려_있으면_시켜도_미끄러지지_않는다() {
    // 미끄러지는 것은 바닥과 닿아야 성립한다 — 공중에서 배를 깔면 그냥 헤엄이다
    let mut 헤엄 = pet();
    헤엄.air = true;
    assert!(!헤엄.start_slide(1_000));
    assert_ne!(헤엄.behavior(), Behavior::Slide);

    let mut 들림 = pet();
    들림.drag_start(1_000);
    assert!(!들림.start_slide(1_100));
    assert_eq!(들림.behavior(), Behavior::Dragged);
}

#[test]
fn 이미_하는_중이면_다시_시켜도_받지_않는다() {
    // 다시 진입하면 코어는 길이를 늘리는데 웹뷰는 클래스가 그대로라
    // 애니메이션을 되감지 않는다 — 그림과 상태가 어긋난다
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
    // 손에 쥔 채로 시작하면 놓는 순간 낙하와 낚시가 겹친다
    let mut 들림 = pet();
    들림.drag_start(1_000);
    assert!(!들림.start_fishing(1_100));
    assert_eq!(들림.behavior(), Behavior::Dragged);
}

#[test]
fn 공중에서_시키면_허공에서_낚시한다() {
    // 바닥으로 끌어내리면 헤엄치다 순간이동한다
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
    // 유휴로 바로 가면 clamp가 바닥으로 순간이동시킨다
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
    // 자극 시각을 갱신하지 않으면 시켜 놓고 조는 판이 생긴다
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
    // 앉을 자리가 없다. 지금은 pick_next가 지상에서만 불리지만,
    // 그 전제가 깨져도 낚시가 공중에서 시작되면 안 된다
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
