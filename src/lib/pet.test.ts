import { describe, expect, it } from "vitest";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
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
      { kind: "startled" },
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

  it("손이_멈춘_채_떼면_약하게_잡힌다", () => {
    // 마지막 구간이 정지 — 세게 던진 것으로 오인하면 안 된다
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 200, y: 0, t: 200 },
      { x: 200, y: 0, t: 300 },
      { x: 200, y: 0, t: 400 },
    ];
    expect(throwVelocity(samples).vx).toBe(0);
  });
});
