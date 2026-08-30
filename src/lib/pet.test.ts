import { describe, expect, it } from "vitest";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
  isOneShot,
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
    // 매핑이 겹치면 두 동작이 같은 애니메이션으로 보인다
    const all: Behavior[] = [
      { kind: "walk" },
      { kind: "turn" },
      { kind: "swim" },
      { kind: "thrown" },
      { kind: "sleep" },
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
    ];
    const classes = all.map(behaviorClass);
    expect(new Set(classes).size).toBe(all.length);
  });

  it("싸가지_반응도_종류까지_내려간다", () => {
    expect(behaviorClass({ kind: "sassy", sassy: "turn_away" })).toBe("pg--sassy-turn-away");
    expect(behaviorClass({ kind: "sassy", sassy: "butt_wiggle" })).toBe("pg--sassy-butt-wiggle");
  });

  it("클래스명은_CSS_선택자로_쓸_수_있는_형태다", () => {
    // 밑줄이 남아 있으면 pet.css의 하이픈 선택자와 어긋난다
    for (const cls of [
      behaviorClass({ kind: "idle", idle: "look_around" }),
      behaviorClass({ kind: "idle", idle: "shift_feet" }),
    ]) {
      expect(cls).not.toContain("_");
      expect(cls).toMatch(/^pg--[a-z-]+$/);
    }
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
    // 100ms 동안 x로 60px 움직였다 = 600px/s
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
    // 포인터 보고가 드물면 최근 120ms 안에 마지막 샘플만 남는다.
    // 기준점을 마지막 샘플로 잡으면 dt가 0이 되어 던지기가 통째로 죽는다
    const v = throwVelocity([
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 },
    ]);
    expect(v.vx).toBeCloseTo(1000, 5);
  });

  it("같은_시각_샘플만_있으면_영으로_수렴한다", () => {
    // 0으로 나누면 Infinity가 나와 펭귄이 화면 밖으로 날아간다
    const v = throwVelocity([
      { x: 0, y: 0, t: 5 },
      { x: 100, y: 0, t: 5 },
    ]);
    expect(v).toEqual({ vx: 0, vy: 0 });
  });

  it("최근_구간만_보므로_초반의_느린_구간에_희석되지_않는다", () => {
    // 앞에서 오래 머뭇대다 마지막에 확 뿌린 궤적
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 2, y: 0, t: 400 },
      { x: 4, y: 0, t: 800 },
      { x: 104, y: 0, t: 900 },
    ];
    // 전체 평균이면 약 115px/s, 최근 100ms만 보면 1000px/s
    expect(throwVelocity(samples).vx).toBeCloseTo(1000, 5);
  });

  it("세게_뿌린_뒤_멈춘_채_떼면_던져지지_않는다", () => {
    // 생산자(PetApp)는 움직임이 없는 pointermove에는 샘플을 남기지 않고,
    // 놓는 시점에만 한 번 더 남긴다. 정지 구간이 "같은 좌표의 연속 샘플"로
    // 나타난다고 가정하면 실제로 만들 수 없는 입력을 검사하게 된다.
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 }, // 마지막 움직임
      { x: 200, y: 0, t: 2200 }, // 2초 멈춰 있다 놓음
    ];
    expect(throwVelocity(samples).vx).toBe(0);
  });

  it("놓는_시점_샘플이_없으면_옛_속도가_그대로_잡힌다", () => {
    // 위 테스트가 무엇을 지키는지 반대편에서 고정한다 — 놓는 샘플을 빠뜨리면
    // 2초를 멈춰 있었어도 그때의 속도로 던져진다
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
    // 이걸 놓치면 연타했을 때 애니메이션이 되감기지 않아 첫 번만 반응한 것처럼 보인다
    expect(isOneShot("pg--turn")).toBe(true);
    expect(isOneShot("pg--land")).toBe(true);
    expect(isOneShot("pg--sassy-eye-roll")).toBe(true);
    expect(isOneShot("pg--walk")).toBe(false);
    expect(isOneShot("pg--swim")).toBe(false);
    expect(isOneShot("pg--sleep")).toBe(false);
  });

  it("모든_싸가지_반응이_한_번짜리로_잡힌다", () => {
    const kinds = ["turn_away", "head_flick", "wing_flick", "eye_roll", "butt_wiggle"] as const;
    for (const sassy of kinds) {
      expect(isOneShot(behaviorClass({ kind: "sassy", sassy }))).toBe(true);
    }
  });
});
