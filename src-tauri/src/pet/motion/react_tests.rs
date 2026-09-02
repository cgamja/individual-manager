use crate::pet::test_support::*;
use crate::pet::*;

#[test]
    fn 휘둘러도_날아가지_않는다() {
        let mut p = pet();
        p.step(1_000, &world());
        let before = p.snapshot();
        p.whack(1_000, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭하면 바로 휘두른다");

        let mut t = 1_000;
        for _ in 0..30 {
            t += 50;
            let s = p.step(t, &world());
            assert_eq!(s.x, before.x, "옆으로 밀리면 안 된다");
            assert_eq!(s.y, before.y, "떠오르면 안 된다");
            assert_ne!(s.behavior, Behavior::Thrown, "던져진 상태가 되면 안 된다");
        }
    }

    #[test]
    fn 휘두르고_나면_약을_올린다() {
        let mut p = pet();
        p.step(1_000, &world());
        p.whack(1_000, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
        let after = p.step(1_000 + SWING_MS + 20, &world());
        assert!(
            matches!(after.behavior, Behavior::Sassy { .. }),
            "휘두르고 나면 약이 올라야 한다 (실제: {:?})",
            after.behavior
        );
    }

    #[test]
    fn 빠따는_한_번에_한_번씩_횟수가_는다() {
        let mut p = pet();
        assert_eq!(p.snapshot().whack_seq, 0);
        for i in 1..=5u64 {
            p.whack(1_000 + i * 100, &world(), 0.0, 0.0);
            assert_eq!(p.snapshot().whack_seq, i, "{i}번째 빠따가 안 세어졌다");
        }
    }

    #[test]
    fn 던져서_나는_중에_휘둘러도_그_자리에서_마저_떨어진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 600.0, -400.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown);

        let mut t = 1_100;
        for _ in 0..4 {
            t += 50;
            p.step(t, &world());
        }
        assert_eq!(p.behavior(), Behavior::Thrown, "아직 나는 중이어야 한다");
        assert!(p.snapshot().air, "공중 상태여야 한다");

        p.whack(t, &world(), 0.0, 0.0);
        let hit_y = p.snapshot().y;
        assert_eq!(p.behavior(), Behavior::Swing);
        t += 50;
        let swinging = p.step(t, &world());
        assert_eq!(swinging.y, hit_y, "휘두른다고 솟아오르거나 떨어지면 안 된다");

        let after = p.step(t + SWING_MS + 20, &world());
        assert_eq!(after.behavior, Behavior::Falling, "공중이었으니 마저 떨어진다");
    }

    #[test]
    fn 빠따는_졸고_있어도_깨운다() {
        let mut p = pet();
        let mut t = 100;
        while p.behavior() != Behavior::Sleep && t < SLEEP_AFTER_MS + 60_000 {
            p.step(t, &world());
            t += 250;
        }
        assert_eq!(p.behavior(), Behavior::Sleep);
        p.whack(t, &world(), 0.0, 0.0);
        assert_eq!(p.behavior(), Behavior::Swing, "클릭 즉시 휘두른다");
    }

    #[test]
    fn 휘두른다고_말하지는_않는다() {
        let mut p = pet();
        p.whack(1_000, &world(), 0.0, 0.0);
        assert!(p.snapshot().speech.is_none(), "클릭으로 말이 나오면 안 된다");
        p.whack(1_100, &world(), 0.0, 0.0);
        p.whack(1_200, &world(), 0.0, 0.0);
        assert!(p.snapshot().speech.is_none(), "연타해도 마찬가지다");
    }

    /// 연타로 빽빽거리게 만든 펭귄과 터진 시각.
    fn 빽빽거리는_펭귄() -> (Pet, u64) {
        let mut p = pet();
        p.step(1_000, &world());
        let mut t = 1_000;
        for _ in 0..SQUAWK_WHACK_COUNT {
            t += 150;
            클릭(&mut p, t);
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "연타로 터져야 한다");
        (p, t)
    }

    #[test]
    fn 짧은_간격으로_스무_번_맞으면_빽빽거린다() {
        let mut p = pet();
        p.step(1_000, &world());
        let mut t = 1_000;
        for i in 1..=SQUAWK_WHACK_COUNT {
            t += 150;
            클릭(&mut p, t);
            if i < SQUAWK_WHACK_COUNT {
                assert_eq!(p.behavior(), Behavior::Swing, "{i}번째까지는 휘두른다");
            }
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "문턱을 넘은 클릭에서 터진다");
    }

    #[test]
    fn 띄엄띄엄_때리면_빽빽거리지_않는다() {
        let mut p = pet();
        let mut t = 1_000;
        for _ in 0..6 {
            t += SQUAWK_GAP_MS + 500;
            클릭(&mut p, t);
            assert_eq!(p.behavior(), Behavior::Swing, "간격이 벌어지면 그냥 휘두른다");
        }
    }

    #[test]
    fn 문턱_직전까지는_안_터지고_한_번_더_때리면_터진다() {
        let mut p = pet();
        let mut t = 300;
        for _ in 1..SQUAWK_WHACK_COUNT {
            클릭(&mut p, t);
            t += 100;
        }
        assert_eq!(p.behavior(), Behavior::Swing, "문턱 직전까지는 휘두른다");
        클릭(&mut p, t);
        assert_eq!(p.behavior(), Behavior::Squawk, "한 번 더 때리면 터진다");
    }

    #[test]
    fn 빽빽거리는_중에_맞아도_끊기지_않는다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        클릭(&mut p, t + 200);
        assert_eq!(p.behavior(), Behavior::Squawk, "스윙으로 끊기면 안 된다");
        클릭(&mut p, t + 400);
        assert_eq!(p.behavior(), Behavior::Squawk);
        let mid = p.step(t + 400 + SQUAWK_MS - 50, &world());
        assert_eq!(mid.behavior, Behavior::Squawk, "새 판이 아직 안 끝났다");
        let after = p.step(t + 400 + SQUAWK_MS + 20, &world());
        assert_ne!(after.behavior, Behavior::Squawk, "손을 떼면 제 시간에 끝난다");
    }

    #[test]
    fn 빽빽거리는_중에_맞은_것은_다음_연타로_세지_않는다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        for i in 1..=3 {
            클릭(&mut p, t + i * 100);
        }
        let end = t + 300 + SQUAWK_MS + 20;
        p.step(end, &world());
        클릭(&mut p, end + 40);
        assert_eq!(p.behavior(), Behavior::Swing, "카운터가 초기화돼야 한다");
    }

    #[test]
    fn 빽빽거리는_동안_제자리에_있다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        let before = p.snapshot();
        let mut now = t;
        for _ in 0..10 {
            now += 50;
            let s = p.step(now, &world());
            assert_eq!(s.x, before.x, "옆으로 움직이면 안 된다");
            assert_eq!(s.y, before.y, "떠오르거나 가라앉으면 안 된다");
        }
    }

    #[test]
    fn 빽빽거리기가_끝나면_유휴로_간다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        let after = p.step(t + SQUAWK_MS + 20, &world());
        assert!(
            matches!(after.behavior, Behavior::Idle { .. }),
            "유휴로 나가야 한다 (실제: {:?})",
            after.behavior
        );
    }

    #[test]
    fn 공중에서_빽빽거리면_끝나고_떨어진다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        assert!(p.snapshot().air, "공중이어야 한다");

        let mut t = 1_100;
        for _ in 0..SQUAWK_WHACK_COUNT {
            t += 150;
            p.whack(t, &world(), 0.0, 0.0);
        }
        assert_eq!(p.behavior(), Behavior::Squawk, "공중에서도 터진다");
        assert!(p.snapshot().air, "고도를 물려받아야 한다");

        let after = p.step(t + SQUAWK_MS + 20, &world());
        assert_eq!(after.behavior, Behavior::Falling, "공중이었으니 마저 떨어진다");
    }

    #[test]
    fn 빽빽거리다_던져지면_되돌아오지_않는다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        p.drag_start(t + 100);
        p.drag_by(120.0, -80.0);
        p.drag_end(t + 200, 900.0, -600.0, &world());
        assert_eq!(p.behavior(), Behavior::Thrown, "던져진 상태여야 한다");

        클릭(&mut p, t + 300);
        assert_ne!(p.behavior(), Behavior::Squawk, "예산은 나가는 순간 무효다");
    }

    #[test]
    fn 빽빽거리는_중에_들어_올릴_수_있다() {
        let (mut p, t) = 빽빽거리는_펭귄();
        p.drag_start(t + 100);
        assert_eq!(p.behavior(), Behavior::Dragged);
    }

    #[test]
    fn 빽빽거리기는_제자리_동작이다() {
        assert!(!Behavior::Squawk.is_airborne(), "스스로 뜨지 않는다");
        assert!(!Behavior::Squawk.is_landing(), "바닥에 닿아서 생긴 게 아니다");
        assert!(Behavior::Squawk.moves_window(), "틱을 빠르게 유지해야 한다");
    }

    #[test]
    fn 시키면_바로_빽빽거린다() {
        let mut p = pet();
        p.step(1_000, &world());
        assert!(p.start_squawk(1_000));
        assert_eq!(p.behavior(), Behavior::Squawk);
    }

    #[test]
    fn 공중에서도_시키면_빽빽거린다() {
        let mut p = pet();
        p.drag_start(1_000);
        p.drag_by(0.0, -300.0);
        p.step(1_050, &world());
        p.drag_end(1_100, 0.0, 0.0, &world());
        assert!(p.start_squawk(1_150));
        assert_eq!(p.behavior(), Behavior::Squawk);
        assert!(p.snapshot().air, "바닥으로 끌어내리면 순간이동한다");
    }

    #[test]
    fn 들려_있거나_이미_빽빽거리면_시켜도_안_한다() {
        let mut p = pet();
        p.drag_start(1_000);
        assert!(!p.start_squawk(1_050), "손에 쥔 채로는 안 된다");

        let (mut q, t) = 빽빽거리는_펭귄();
        assert!(!q.start_squawk(t + 100), "재진입하면 웹뷰가 되감지 못한다");
    }
