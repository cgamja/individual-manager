import { describe, expect, it } from "vitest";
import { DRAG_THRESHOLD_PX, behaviorClass, type Behavior } from "./pet";

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
