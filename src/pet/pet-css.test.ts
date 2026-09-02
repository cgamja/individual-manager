import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { behaviorClass, verticalClass, type Behavior, type Vertical } from "../lib/pet";

/** 동작이 CSS에 실제로 그려져 있는지 확인한다. */
/** 코어가 낼 수 있는 모든 동작. 코어에 추가하면 여기도 늘려야 한다. */
const css = ["base","ground","rest","react","pinball","drag","air","speech","fishing","freakout","bowling"]
  .map((n) => readFileSync(resolve(`src/pet/css/${n}.css`), "utf8"))
  .join("\n");
const petRs =
  readFileSync(resolve("src-tauri/src/pet/tuning.rs"), "utf8") +
  readFileSync(resolve("src-tauri/src/pet/mod.rs"), "utf8") +
  readFileSync(resolve("src-tauri/src/pet_bridge/window.rs"), "utf8");
const petApp =
  readFileSync(resolve("src/pet/PetApp.tsx"), "utf8") +
  readFileSync(resolve("src/pet/Penguin.tsx"), "utf8");

/** `--이름: 123px;` 에서 숫자만 꺼낸다. */
function cssVar(name: string): number | null {
  const m = css.match(new RegExp(`--${name}:\\s*([0-9.]+)px`));
  return m ? Number(m[1]) : null;
}

/** `pub const 이름: f64 = 123.0;` 에서 숫자만 꺼낸다. */
function rustConst(name: string): number | null {
  const m = petRs.match(new RegExp(`pub const ${name}: f64 = ([0-9.]+)`));
  return m ? Number(m[1]) : null;
}

const ALL_BEHAVIORS: Behavior[] = [
  { kind: "walk" },
  { kind: "turn" },
  { kind: "swim" },
  { kind: "sleep" },
  { kind: "dragged" },
  { kind: "falling" },
  { kind: "thrown" },
  { kind: "land" },
  { kind: "splat" },
  { kind: "sprawl" },
  { kind: "tumble" },
  { kind: "slide" },
  { kind: "squawk" },
  { kind: "freakout", freakout: "dash" },
  { kind: "freakout", freakout: "pant" },
  { kind: "bowling", bowling: "gather" },
  { kind: "bowling", bowling: "ready" },
  { kind: "bowling", bowling: "struck" },
  { kind: "bowling", bowling: "scatter" },
  { kind: "ice_fishing", fishing: "dig" },
  { kind: "ice_fishing", fishing: "wait" },
  { kind: "ice_fishing", fishing: "bite" },
  { kind: "ice_fishing", fishing: "catch" },
  { kind: "ice_fishing", fishing: "miss" },
  { kind: "ice_fishing", fishing: "pack" },
  { kind: "idle", idle: "look_around" },
  { kind: "idle", idle: "stretch" },
  { kind: "idle", idle: "shake" },
  { kind: "idle", idle: "shift_feet" },
  { kind: "swing" },
  { kind: "sassy", sassy: "turn_away" },
  { kind: "sassy", sassy: "head_flick" },
  { kind: "sassy", sassy: "wing_flick" },
  { kind: "sassy", sassy: "eye_roll" },
  { kind: "sassy", sassy: "butt_wiggle" },
];

/** `name`이 더 긴 이름의 앞부분으로 끼어든 경우를 빼고 센다. */
function countExact(haystack: string, name: string): number {
  const re = new RegExp(`(?<![\\w-])${name}(?![\\w-])`, "g");
  return [...haystack.matchAll(re)].length;
}

describe("pet.css 커버리지", () => {
  it.each(ALL_BEHAVIORS.map((b) => [behaviorClass(b)] as const))(
    "%s 동작에 대응하는 규칙이 있다",
    (cls) => {
      expect(css).toContain(`.${cls}`);
    },
  );

  it("세로_방향_클래스가_모두_쓰인다", () => {
    for (const v of ["up", "down"] as Vertical[]) {
      expect(css).toContain(`.${verticalClass(v)}`);
    }
  });

  it("핀볼_커서_규칙이_실제로_있다", () => {
    expect(petApp).toContain("pg-pinball");
    expect(css).toContain(".pg-pinball");
    expect(css).not.toMatch(/\.pg-pinball:not\(/);
  });

  it("CSS에_없어진_동작의_잔재가_남아있지_않다", () => {
    expect(css).not.toContain("pg--startled");
  });

  it("같은_이름의_keyframes가_두_번_정의되지_않는다", () => {
    const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    const 중복 = defined.filter((n, i) => defined.indexOf(n) !== i);
    expect(중복, `중복 정의된 @keyframes: ${중복.join(", ")}`).toEqual([]);
  });

  it("정의된_keyframes가_모두_쓰인다", () => {
    const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    expect(defined.length).toBeGreaterThan(0);
    for (const name of defined) {
      const uses = countExact(css, name);
      expect(uses, `@keyframes ${name}가 정의만 되고 쓰이지 않는다`).toBeGreaterThan(1);
    }
  });
});

describe("창 여백 상수 동기화", () => {
  it.each([
    ["pg-size", "PET_SIZE"],
    ["pg-pad-x", "PET_PAD_X"],
    ["pg-pad-top", "PET_PAD_TOP"],
  ])("--%s 가 Rust의 %s 와 같다", (cssName, rustName) => {
    const a = cssVar(cssName);
    const b = rustConst(rustName);
    expect(a, `CSS에서 --${cssName}를 못 찾았다`).not.toBeNull();
    expect(b, `Rust에서 ${rustName}를 못 찾았다`).not.toBeNull();
    expect(a).toBe(b);
  });
});

/** `const 이름: u64 = 1_100;` 에서 숫자만 꺼낸다 (밑줄 구분자를 지운다). */
function rustMs(name: string): number | null {
  const m = petRs.match(new RegExp(`const ${name}: u64 = ([0-9_]+)`));
  return m ? Number(m[1].replace(/_/g, "")) : null;
}

/** `.pg--이름 .pg-all { animation: ... 1.1s ... }` 의 길이를 ms로 꺼낸다. */
function cssDurationMs(cls: string): number | null {
  const m = css.match(
    new RegExp(`\\.${cls}\\s+\\.pg-all\\s*\\{[^}]*animation:[^;]*?\\s([0-9.]+)s`),
  );
  return m ? Math.round(Number(m[1]) * 1000) : null;
}

describe("동작 길이 동기화", () => {
  it("굴러떨어지기가 Rust의 TUMBLE_MS와 같다", () => {
    const a = cssDurationMs("pg--tumble");
    const b = rustMs("TUMBLE_MS");
    expect(a, "CSS에서 .pg--tumble .pg-all의 길이를 못 찾았다").not.toBeNull();
    expect(b, "Rust에서 TUMBLE_MS를 못 찾았다").not.toBeNull();
    expect(a).toBe(b);
  });

  it.each([
    ["pg--fishing-dig", "FISHING_DIG_MS"],
    ["pg--fishing-bite", "FISHING_BITE_MS"],
    ["pg--fishing-catch", "FISHING_CATCH_MS"],
    ["pg--fishing-miss", "FISHING_MISS_MS"],
    ["pg--fishing-pack", "FISHING_PACK_MS"],
    ["pg--slide", "SLIDE_MS"],
    ["pg--squawk", "SQUAWK_MS"],
    ["pg--freakout-pant", "FREAKOUT_PANT_MS"],
    ["pg--bowling-scatter", "BOWLING_SCATTER_MS"],
  ])("%s 가 Rust의 %s 와 같다", (cls, konst) => {
    const a = cssDurationMs(cls);
    const b = rustMs(konst);
    expect(a, `CSS에서 .${cls} .pg-all의 길이를 못 찾았다`).not.toBeNull();
    expect(b, `Rust에서 ${konst}를 못 찾았다`).not.toBeNull();
    expect(a).toBe(b);
  });
});

describe("평소 숨기는 도형", () => {
  const 숨기는_도형 = ["pg-hole", "pg-rod", "pg-line", "pg-float", "pg-fish", "pg-beak-lower"];

  /** 선택자에 이 클래스가 정확히 등장하는 모든 규칙 블록의 본문. */
  function 규칙들(cls: string): string[] {
    const 본문 = css.replace(/\/\*[\s\S]*?\*\//g, " ");
    const found: string[] = [];
    for (const m of 본문.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
      if (m[1].includes("@")) continue;
      if (new RegExp(`\\.${cls}(?![\\w-])`).test(m[1])) found.push(m[2]);
    }
    return found;
  }

  it.each(숨기는_도형.map((c) => [c] as const))("%s 는 display로 감춘다", (cls) => {
    const blocks = 규칙들(cls);
    expect(blocks.length, `.${cls} 규칙을 못 찾았다`).toBeGreaterThan(0);
    expect(
      blocks.some((b) => /display:\s*none/.test(b)),
      `.${cls} 를 감추는 display: none 규칙이 없다`,
    ).toBe(true);
    for (const b of blocks) {
      expect(
        /opacity:\s*0\s*(?:;|$)/m.test(b),
        `.${cls} 를 opacity: 0으로 감추면 후광이 남는다`,
      ).toBe(false);
    }
  });
});

describe("빽빽거리기 부위 애니메이션 길이", () => {
  it("부위마다_주기_×_횟수가_SQUAWK_MS와_같다", () => {
    const total = rustMs("SQUAWK_MS");
    expect(total, "Rust에서 SQUAWK_MS를 못 찾았다").not.toBeNull();
    const re = /\.pg--squawk\s+(\.[\w-]+)\s*\{[^}]*animation:\s*[\w-]+\s+([0-9.]+)s[^;]*?(?:\s(\d+))?;/g;
    const 부위 = [...css.matchAll(re)];
    expect(부위.length, ".pg--squawk 부위 애니메이션을 못 찾았다").toBeGreaterThan(1);
    for (const [, sel, secs, count] of 부위) {
      const ms = Math.round(Number(secs) * 1000) * Number(count ?? 1);
      expect(ms, `${sel} 의 주기 × 횟수가 SQUAWK_MS와 다르다`).toBe(total);
    }
  });
});

describe("PetApp이 쓰는 클래스에 스타일이 있다", () => {
  const used = new Set<string>();
  for (const m of petApp.matchAll(/\bpg-[a-z0-9-]+/g)) {
    const cls = m[0];
    if (cls.startsWith("pg--") || cls.startsWith("pg-v--")) continue;
    used.add(cls);
  }

  it("검사할 클래스를 찾았다", () => {
    expect(used.size).toBeGreaterThan(0);
  });

  it.each([...used].map((c) => [c] as const))("%s 에 규칙이 있다", (cls) => {
    expect(css).toContain(`.${cls}`);
  });
});
