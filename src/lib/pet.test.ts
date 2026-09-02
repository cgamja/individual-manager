import { describe, expect, it } from "vitest";
import { voiceOffsetFor } from "../pet/sound";
import {
  isFemalePet,
  DRAG_THRESHOLD_PX,
  DEFAULT_TAUNTS,
  tauntFor,
  behaviorClass,
  isOneShot,
  shouldRestart,
  throwVelocity,
  verticalClass,
  type Behavior,
} from "./pet";

describe("behaviorClass", () => {
  it("동작을_CSS_클래스명으로_매핑한다", () => {
    expect(behaviorClass({ kind: "walk" })).toBe("pg--walk");
    expect(behaviorClass({ kind: "sleep" })).toBe("pg--sleep");
    expect(behaviorClass({ kind: "dragged" })).toBe("pg--dragged");
    expect(behaviorClass({ kind: "land" })).toBe("pg--land");
  });

  it("유휴는_종류까지_내려가야_두리번과_기지개가_구분된다", () => {
    expect(behaviorClass({ kind: "idle", idle: "look_around" })).toBe("pg--idle-look-around");
    expect(behaviorClass({ kind: "idle", idle: "shift_feet" })).toBe("pg--idle-shift-feet");
    expect(behaviorClass({ kind: "idle", idle: "stretch" })).toBe("pg--idle-stretch");
    expect(behaviorClass({ kind: "idle", idle: "shake" })).toBe("pg--idle-shake");
  });

  it("모든_동작이_서로_다른_클래스를_받는다", () => {
    const all: Behavior[] = [
      { kind: "walk" },
      { kind: "turn" },
      { kind: "swim" },
      { kind: "thrown" },
      { kind: "sleep" },
      { kind: "swing" },
      { kind: "sassy", sassy: "turn_away" },
      { kind: "sassy", sassy: "head_flick" },
      { kind: "sassy", sassy: "wing_flick" },
      { kind: "sassy", sassy: "eye_roll" },
      { kind: "sassy", sassy: "butt_wiggle" },
      { kind: "dragged" },
      { kind: "falling" },
      { kind: "land" },
      { kind: "idle", idle: "look_around" },
      { kind: "idle", idle: "stretch" },
      { kind: "idle", idle: "shake" },
      { kind: "idle", idle: "shift_feet" },
      { kind: "ice_fishing", fishing: "dig" },
      { kind: "ice_fishing", fishing: "wait" },
      { kind: "ice_fishing", fishing: "bite" },
      { kind: "ice_fishing", fishing: "catch" },
      { kind: "ice_fishing", fishing: "miss" },
      { kind: "ice_fishing", fishing: "pack" },
      { kind: "slide" },
      { kind: "squawk" },
      { kind: "freakout", freakout: "dash" },
      { kind: "freakout", freakout: "pant" },
    ];
    const classes = all.map(behaviorClass);
    expect(new Set(classes).size).toBe(all.length);
  });

  it("얼음낚시도_국면까지_내려간다", () => {
    expect(behaviorClass({ kind: "ice_fishing", fishing: "dig" })).toBe("pg--fishing-dig");
    expect(behaviorClass({ kind: "ice_fishing", fishing: "catch" })).toBe("pg--fishing-catch");
  });

  it("싸가지_반응도_종류까지_내려간다", () => {
    expect(behaviorClass({ kind: "sassy", sassy: "turn_away" })).toBe("pg--sassy-turn-away");
    expect(behaviorClass({ kind: "sassy", sassy: "butt_wiggle" })).toBe("pg--sassy-butt-wiggle");
  });

  it("모든_클래스명이_CSS_선택자로_쓸_수_있는_형태다", () => {
    const all: Behavior[] = [
      { kind: "swing" },
      { kind: "swim" },
      { kind: "idle", idle: "look_around" },
      { kind: "idle", idle: "shift_feet" },
      { kind: "sassy", sassy: "turn_away" },
      { kind: "sassy", sassy: "butt_wiggle" },
    ];
    for (const b of all) {
      const cls = behaviorClass(b);
      expect(cls, `${JSON.stringify(b)} → ${cls}`).not.toContain("_");
      expect(cls).toMatch(/^pg--[a-z-]+$/);
    }
  });
});

describe("shouldRestart", () => {
  /** 빠따 횟수가 그대로인 평범한 전이. */
  const k = (cls: string, whackSeq = 0) => ({ cls, whackSeq });

  it("동작이_바뀌면_되감는다", () => {
    expect(shouldRestart(k("pg--fishing-bite"), k("pg--fishing-catch"))).toBe(true);
    expect(shouldRestart(null, k("pg--fishing-catch"))).toBe(true);
  });

  it("말풍선_때문에_온_스냅샷은_애니메이션을_건드리지_않는다", () => {
    expect(shouldRestart(k("pg--fishing-catch"), k("pg--fishing-catch"))).toBe(false);
  });

  it("슬라이딩은_한_번짜리다", () => {
    expect(isOneShot("pg--slide")).toBe(true);
  });

  it("빽빽거리기는_한_번짜리다", () => {
    expect(isOneShot("pg--squawk")).toBe(true);
  });

  it("방망이_스윙은_한_번짜리다", () => {
    expect(isOneShot("pg--swing")).toBe(true);
  });

  it("숨_고르기는_한_번짜리고_광란은_아니다", () => {
    expect(isOneShot("pg--freakout-pant")).toBe(true);
    expect(isOneShot("pg--freakout-dash")).toBe(false);
  });

  it("반복_애니메이션은_되감지_않는다", () => {
    expect(shouldRestart(null, k("pg--walk"))).toBe(false);
    expect(shouldRestart(null, k("pg--fishing-wait"))).toBe(false);
  });

  it("한_번짜리_낚시_국면은_모두_되감는다", () => {
    for (const fishing of ["dig", "bite", "catch", "miss", "pack"] as const) {
      const cls = behaviorClass({ kind: "ice_fishing", fishing });
      expect(shouldRestart(k("pg--fishing-wait"), k(cls)), cls).toBe(true);
    }
  });

  it("연타하면_스윙을_되감는다", () => {
    // 360ms 안에 다시 때리면 코어는 `Swing`을 다시 걸지만 클래스가 그대로라
    // 브라우저가 애니메이션을 재생하지 않는다 — 방망이가 한 번만 휘둘러진다.
    expect(shouldRestart(k("pg--swing", 1), k("pg--swing", 2))).toBe(true);
  });

  it("같은_스윙이_다시_와도_되감지_않는다", () => {
    // 말풍선처럼 빠따와 무관한 이유로 온 스냅샷이다.
    expect(shouldRestart(k("pg--swing", 3), k("pg--swing", 3))).toBe(false);
  });

  it("빽빽거리는_중에_때려도_되감지_않는다", () => {
    // 되감으면 판이 계속 연장되는 동안 애니메이션의 첫 조각만 반복하며
    // **영원히 부풀기만 한다** (`MOTIONS.md` 빽빽거리기 절). 이 항목이
    // 고치려던 것과 정확히 반대다.
    expect(shouldRestart(k("pg--squawk", 5), k("pg--squawk", 6))).toBe(false);
  });
});

describe("드래그 임계값", () => {
  it("클릭과_드래그를_가르는_임계값이_양수다", () => {
    expect(DRAG_THRESHOLD_PX).toBeGreaterThan(0);
  });
});

describe("verticalClass", () => {
  it("세로_방향을_CSS_클래스명으로_매핑한다", () => {
    expect(verticalClass("up")).toBe("pg-v--up");
    expect(verticalClass("down")).toBe("pg-v--down");
    expect(verticalClass("level")).toBe("pg-v--level");
  });
});

describe("throwVelocity", () => {
  it("궤적에서_초당_속도를_낸다", () => {
    const v = throwVelocity([
      { x: 0, y: 0, t: 0 },
      { x: 60, y: -30, t: 100 },
    ]);
    expect(v.vx).toBeCloseTo(600, 5);
    expect(v.vy).toBeCloseTo(-300, 5);
  });

  it("샘플이_모자라면_영이다", () => {
    expect(throwVelocity([])).toEqual({ vx: 0, vy: 0 });
    expect(throwVelocity([{ x: 1, y: 1, t: 1 }])).toEqual({ vx: 0, vy: 0 });
  });

  it("창_안에_마지막_샘플뿐이어도_직전_샘플로_속도를_낸다", () => {
    const v = throwVelocity([
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 },
    ]);
    expect(v.vx).toBeCloseTo(1000, 5);
  });

  it("같은_시각_샘플만_있으면_영으로_수렴한다", () => {
    const v = throwVelocity([
      { x: 0, y: 0, t: 5 },
      { x: 100, y: 0, t: 5 },
    ]);
    expect(v).toEqual({ vx: 0, vy: 0 });
  });

  it("최근_구간만_보므로_초반의_느린_구간에_희석되지_않는다", () => {
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 2, y: 0, t: 400 },
      { x: 4, y: 0, t: 800 },
      { x: 104, y: 0, t: 900 },
    ];
    expect(throwVelocity(samples).vx).toBeCloseTo(1000, 5);
  });

  it("세게_뿌린_뒤_멈춘_채_떼면_던져지지_않는다", () => {
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 }, // 마지막 움직임
      { x: 200, y: 0, t: 2200 }, // 2초 멈춰 있다 놓음
    ];
    expect(throwVelocity(samples).vx).toBe(0);
  });

  it("놓는_시점_샘플이_없으면_옛_속도가_그대로_잡힌다", () => {
    const withRelease = [
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 },
      { x: 200, y: 0, t: 2200 },
    ];
    const withoutRelease = withRelease.slice(0, 2);
    expect(throwVelocity(withRelease).vx).toBe(0);
    expect(throwVelocity(withoutRelease).vx).toBeCloseTo(1000, 5);
  });
});

describe("isOneShot", () => {
  it("한_번짜리_동작을_가려낸다", () => {
    expect(isOneShot("pg--turn")).toBe(true);
    expect(isOneShot("pg--land")).toBe(true);
    expect(isOneShot("pg--sassy-eye-roll")).toBe(true);
    expect(isOneShot("pg--walk")).toBe(false);
    expect(isOneShot("pg--swim")).toBe(false);
    expect(isOneShot("pg--sleep")).toBe(false);
  });

  it("드리우기만_반복이고_나머지_낚시_국면은_한_번짜리다", () => {
    expect(isOneShot("pg--fishing-wait")).toBe(false);
    for (const fishing of ["dig", "bite", "catch", "miss", "pack"] as const) {
      expect(isOneShot(behaviorClass({ kind: "ice_fishing", fishing }))).toBe(true);
    }
  });

  it("모든_싸가지_반응이_한_번짜리로_잡힌다", () => {
    const kinds = ["turn_away", "head_flick", "wing_flick", "eye_roll", "butt_wiggle"] as const;
    for (const sassy of kinds) {
      expect(isOneShot(behaviorClass({ kind: "sassy", sassy }))).toBe(true);
    }
  });
});

describe("킹받는 대사", () => {
  it("대사가_비어_있지_않다", () => {
    expect(DEFAULT_TAUNTS.length).toBeGreaterThan(5);
    for (const line of DEFAULT_TAUNTS) expect(line.trim().length).toBeGreaterThan(0);
  });

  it("같은_대사가_두_번_들어있지_않다", () => {
    expect(new Set(DEFAULT_TAUNTS).size).toBe(DEFAULT_TAUNTS.length);
  });

  it("추첨값을_대사로_바꾼다", () => {
    expect(tauntFor(0, DEFAULT_TAUNTS)).toBe(DEFAULT_TAUNTS[0]);
    expect(tauntFor(DEFAULT_TAUNTS.length, DEFAULT_TAUNTS)).toBe(DEFAULT_TAUNTS[0]);
    expect(tauntFor(DEFAULT_TAUNTS.length + 3, DEFAULT_TAUNTS)).toBe(DEFAULT_TAUNTS[3]);
  });

  it("아주_큰_값이나_음수에도_대사를_돌려준다", () => {
    for (const roll of [Number.MAX_SAFE_INTEGER, -7, 0, 1e15]) {
      expect(DEFAULT_TAUNTS).toContain(tauntFor(roll, DEFAULT_TAUNTS));
    }
  });
});

describe("성별은 창 라벨에서 결정적으로 나온다", () => {
  it("같은_라벨은_항상_같은_결과다", () => {
    // 난수를 쓰면 앱을 껐다 켤 때마다 옷이 바뀐다 (PRINCIPLE 3).
    for (const label of ["pet-1", "pet-2", "pet-7", "pet-8"]) {
      const 처음 = isFemalePet(label);
      for (let i = 0; i < 5; i += 1) expect(isFemalePet(label)).toBe(처음);
    }
  });

  it("암수가_둘_다_나온다", () => {
    const 결과 = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => isFemalePet(`pet-${n}`));
    expect(결과.some(Boolean), "암컷이 하나도 없다").toBe(true);
    expect(결과.some((f) => !f), "수컷이 하나도 없다").toBe(true);
  });

  it("홀짝으로_안_갈린다", () => {
    // **팀 배정(`assign_sides`)이 id 홀짝 교대다.** 성별도 홀짝이면 매 판이
    // 여자팀 대 남자팀이 된다 — 실제로 `(id * 11) % 2`로 썼다가 그랬다
    // (그건 `id % 2`와 같은 식이다).
    const 성별 = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => isFemalePet(`pet-${n}`));
    const 홀짝 = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => n % 2 === 1);
    expect(성별, "성별이 id 홀짝과 똑같다 — 팀과 붙어 버린다").not.toEqual(홀짝);
    expect(성별, "성별이 id 홀짝의 반대일 뿐이다").not.toEqual(홀짝.map((x) => !x));
  });

  it("한_팀에_암수가_섞인다", () => {
    // 왼쪽 팀은 id 오름차순의 짝수 번째(= id 1,3,5,7), 오른쪽은 나머지다.
    const 왼쪽 = [1, 3, 5, 7].map((n) => isFemalePet(`pet-${n}`));
    const 오른쪽 = [2, 4, 6, 8].map((n) => isFemalePet(`pet-${n}`));
    for (const [이름, 팀] of [["왼쪽", 왼쪽], ["오른쪽", 오른쪽]] as const) {
      expect(팀.some(Boolean), `${이름} 팀이 전원 수컷이다`).toBe(true);
      expect(팀.some((f) => !f), `${이름} 팀이 전원 암컷이다`).toBe(true);
    }
  });

  it("여덟_마리에서_성비가_한쪽으로_안_쏠린다", () => {
    const 암컷 = [1, 2, 3, 4, 5, 6, 7, 8].filter((n) => isFemalePet(`pet-${n}`)).length;
    expect(암컷).toBeGreaterThanOrEqual(3);
    expect(암컷).toBeLessThanOrEqual(5);
  });

  it("목소리와_따로_논다", () => {
    // 같은 곱수를 쓰면 "높은 목소리 = 암컷"이라는, 아무도 요구하지 않은
    // 규칙이 생긴다.
    const 쌍 = [1, 2, 3, 4, 5, 6, 7, 8].map(
      (n) => `${isFemalePet(`pet-${n}`)}:${voiceOffsetFor(`pet-${n}`)}`,
    );
    expect(new Set(쌍).size).toBeGreaterThan(2);
  });

  it("펫_창이_아니면_수컷으로_떨어진다", () => {
    for (const label of ["main", "volley-court", "", "pet-x"]) {
      expect(isFemalePet(label)).toBe(false);
    }
  });
});
