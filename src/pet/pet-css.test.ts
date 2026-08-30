import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { behaviorClass, verticalClass, type Behavior, type Vertical } from "../lib/pet";

/**
 * 동작이 CSS에 실제로 그려져 있는지 확인한다.
 *
 * 코어에 동작을 추가하고 CSS를 빠뜨리면 아무것도 실패하지 않는다 — 펭귄이
 * 그 동작 동안 아무 반응 없이 서 있을 뿐이라 눈으로만 잡힌다. 커맨드 등록
 * 누락과 같은 부류의 조용한 실패라 소스를 직접 대조한다.
 */
// vitest는 프로젝트 루트에서 돈다. import.meta.url은 jsdom 환경에서 file 스킴이 아니다
const css = readFileSync(resolve("src/pet/pet.css"), "utf8");

/** 코어가 낼 수 있는 모든 동작. 코어에 추가하면 여기도 늘려야 한다. */
const ALL_BEHAVIORS: Behavior[] = [
  { kind: "walk" },
  { kind: "turn" },
  { kind: "swim" },
  { kind: "sleep" },
  { kind: "dragged" },
  { kind: "falling" },
  { kind: "thrown" },
  { kind: "land" },
  { kind: "idle", idle: "look_around" },
  { kind: "idle", idle: "stretch" },
  { kind: "idle", idle: "shake" },
  { kind: "idle", idle: "shift_feet" },
  { kind: "sassy", sassy: "turn_away" },
  { kind: "sassy", sassy: "head_flick" },
  { kind: "sassy", sassy: "wing_flick" },
  { kind: "sassy", sassy: "eye_roll" },
  { kind: "sassy", sassy: "butt_wiggle" },
];

describe("pet.css 커버리지", () => {
  it.each(ALL_BEHAVIORS.map((b) => [behaviorClass(b)] as const))(
    "%s 동작에 대응하는 규칙이 있다",
    (cls) => {
      expect(css).toContain(`.${cls}`);
    },
  );

  it("세로_방향_클래스가_모두_쓰인다", () => {
    // level은 기본값이라 규칙이 없어도 된다 — 기울이지 않는 것이 정상이다
    for (const v of ["up", "down"] as Vertical[]) {
      expect(css).toContain(`.${verticalClass(v)}`);
    }
  });

  it("CSS에_없어진_동작의_잔재가_남아있지_않다", () => {
    // Startled는 Sassy로 대체됐다. 죽은 규칙이 남으면 다음 사람이 헷갈린다
    expect(css).not.toContain("pg--startled");
  });

  it("정의된_keyframes가_모두_쓰인다", () => {
    const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    expect(defined.length).toBeGreaterThan(0);
    for (const name of defined) {
      // 정의부 1회 + 사용부 최소 1회
      const uses = css.split(name).length - 1;
      expect(uses, `@keyframes ${name}가 정의만 되고 쓰이지 않는다`).toBeGreaterThan(1);
    }
  });
});
