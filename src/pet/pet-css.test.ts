import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup, render } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Penguin } from "../assets/penguin";
import {
  PENGUIN_HIT_ARM_RATIO,
  PENGUIN_HIT_BOX,
  PENGUIN_HIT_HYSTERESIS_RATIO,
  PENGUIN_VIEWBOX,
} from "../assets/penguin/hit";
import { behaviorClass, verticalClass, type Behavior, type Vertical } from "../lib/pet";

/** 동작이 CSS에 실제로 그려져 있는지 확인한다. */

afterEach(cleanup);

/** 훌라 차림 그림. 옷을 뜯어보는 검사들이 이 파일만 읽는다. */
const HULA_SRC = "src/assets/penguin/hula.tsx";
/** 펭귄 그림 전부. `pg-*` 클래스를 훑을 때 쓴다.
 *
 * **손으로 적지 않는다** — 목록이면 새 부위 파일이 스윕 밖으로 조용히 빠진다. */
const PENGUIN_ALL = readdirSync(resolve("src/assets/penguin"))
  .filter((f) => f.endsWith(".tsx"))
  .map((f) => `src/assets/penguin/${f}`);
/** 모든 색이 사는 곳. */
const PALETTE_SRC = "src/assets/palette.ts";

/** 코어가 낼 수 있는 모든 동작. 코어에 추가하면 여기도 늘려야 한다. */
/** CSS 주석을 걷어낸다 — 안 그러면 주석에 적힌 `.pg-stage` 같은 이름이
 * "규칙이 있다"로 통과한다. 실제로 그랬다.
 * `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만_css = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ");

/** 펭귄 창이 실제로 읽는 CSS 전부.
 *
 * **`index.css`의 `@import` 목록에서 뽑는다.** 손으로 적으면 앱이 읽는 것과
 * 검사가 보는 것이 갈라진다 — 새 파일을 `@import`만 하면 그 안의 중복
 * `@keyframes`가 검사 밖이고, `@import` 한 줄을 지우면 앱에서 사라진 스타일을
 * 검사는 디스크에서 읽어 통과시킨다. */
const CSS_DIR = "src/pet/css";
const cssFiles = [
  ...readFileSync(resolve(`${CSS_DIR}/index.css`), "utf8").matchAll(
    /@import\s+"\.\/([\w-]+)\.css"/g,
  ),
].map((m) => m[1]);
const css = 코드만_css(
  cssFiles.map((n) => readFileSync(resolve(`${CSS_DIR}/${n}.css`), "utf8")).join("\n"),
);
/** 주석을 걷어낸다 — 지운 클래스가 주석에 남아 "아직 쓰인다"로 집계된다.
 * `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만 = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ");
/** 대조 대상이 되는 Rust 소스 전부. **주석을 걷어내고 본다** — 안 그러면 주석에
 * 적힌 옛 값이 상수로 읽혀 갈라진 것을 못 잡는다. */
const petRs = 코드만(
  readFileSync(resolve("src-tauri/src/pet/tuning.rs"), "utf8") +
    readFileSync(resolve("src-tauri/src/pet/mod.rs"), "utf8") +
    readFileSync(resolve("src-tauri/src/pet_bridge/window.rs"), "utf8") +
    readFileSync(resolve("src-tauri/src/pet_bridge/hit.rs"), "utf8"),
);
const petApp = 코드만(
  readFileSync(resolve("src/pet/PetApp.tsx"), "utf8") +
    PENGUIN_ALL.map((p) => readFileSync(resolve(p), "utf8")).join("\n"),
);

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
  { kind: "bowling", bowling: "scatter" },
  { kind: "volleyball", volley: "gather" },
  { kind: "volleyball", volley: "ready" },
  { kind: "volleyball", volley: "chase" },
  { kind: "volleyball", volley: "bump" },
  { kind: "volleyball", volley: "cheer" },
  { kind: "volleyball", volley: "sulk" },
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

/** `.이름`이 **정확히 그 이름으로** 쓰인 규칙이 있나.
 *
 * `toContain(".pg-foot")`은 `.pg-foot--far`에도 걸린다 — 그래서 `.pg-foot` 규칙을
 * 통째로 지워도 통과했다. 클래스 이름에 쓸 수 있는 문자가 뒤에 오면 다른 클래스다. */
function hasRule(css: string, cls: string): boolean {
  return new RegExp(`\\.${cls}(?![\\w-])`).test(css);
}

/** `name`이 더 긴 이름의 앞부분으로 끼어든 경우를 빼고 센다. */
function countExact(haystack: string, name: string): number {
  const re = new RegExp(`(?<![\\w-])${name}(?![\\w-])`, "g");
  return [...haystack.matchAll(re)].length;
}

describe("pet.css 커버리지", () => {
  it("index_css가_css_폴더의_파일을_전부_읽는다", () => {
    // 한쪽에만 있으면 갈라진다 — `@import`를 빠뜨리면 앱에서 스타일이 사라지고,
    // 파일이 없으면 `@import`가 죽은 줄이다.
    const 디스크 = readdirSync(resolve(CSS_DIR))
      .filter((f) => f.endsWith(".css") && f !== "index.css")
      .map((f) => f.replace(/\.css$/, ""))
      .sort();
    expect([...cssFiles].sort(), "index.css의 @import 목록과 css 폴더가 다르다").toEqual(디스크);
  });

  it.each(ALL_BEHAVIORS.map((b) => [behaviorClass(b)] as const))(
    "%s 동작에 대응하는 규칙이 있다",
    (cls) => {
      expect(hasRule(css, cls), `.${cls} 규칙이 없다`).toBe(true);
    },
  );

  it("세로_방향_클래스가_모두_쓰인다", () => {
    for (const v of ["up", "down"] as Vertical[]) {
      expect(hasRule(css, verticalClass(v)), `.${verticalClass(v)} 규칙이 없다`).toBe(true);
    }
  });

  it("핀볼_커서_규칙이_실제로_있다", () => {
    expect(petApp).toContain("pg-pinball");
    expect(hasRule(css, "pg-pinball"), ".pg-pinball 규칙이 없다").toBe(true);
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

describe("히트 박스 상수 동기화", () => {
  // 판정이 두 곳에 있다 — Rust가 창의 클릭 통과를, 웹뷰가 통과 요청을 정한다.
  // 갈라지면 "웹뷰는 통과를 요청했는데 Rust는 그 자리를 펭귄으로 아는" 진동이
  // 생기고, 두 러너도 타입 검사도 아무 말을 안 한다.
  it.each([
    ["l", "PET_HIT_L"],
    ["t", "PET_HIT_T"],
    ["r", "PET_HIT_R"],
    ["b", "PET_HIT_B"],
  ])("PENGUIN_HIT_BOX.%s 가 Rust의 %s 와 같다", (key, rustName) => {
    const b = rustConst(rustName);
    expect(b, `Rust에서 ${rustName}를 못 찾았다`).not.toBeNull();
    expect(PENGUIN_HIT_BOX[key as keyof typeof PENGUIN_HIT_BOX]).toBe(b);
  });

  it.each([
    ["w", "PET_VIEWBOX_W"],
    ["h", "PET_VIEWBOX_H"],
  ])("PENGUIN_VIEWBOX.%s 가 Rust의 %s 와 같다", (key, rustName) => {
    const b = rustConst(rustName);
    expect(b, `Rust에서 ${rustName}를 못 찾았다`).not.toBeNull();
    expect(PENGUIN_VIEWBOX[key as keyof typeof PENGUIN_VIEWBOX]).toBe(b);
  });

  // **비율도 빠짐없이 본다.** 셋 중 하나만 갈라져도 `요청 ⊃ 되돌리기 ⊃ 히트`가
  // 깨지는데, 그러면 그 띠에서 통과가 50ms마다 켜졌다 꺼진다 — 두 러너도 타입
  // 검사도 아무 말을 안 한다.
  it.each([
    [PENGUIN_HIT_ARM_RATIO, "PET_HIT_ARM_RATIO"],
    [PENGUIN_HIT_HYSTERESIS_RATIO, "PET_HIT_HYSTERESIS_RATIO"],
  ])("비율 %s 가 Rust의 %s 와 같다", (ts, rustName) => {
    const b = rustConst(rustName);
    expect(b, `Rust에서 ${rustName}를 못 찾았다`).not.toBeNull();
    expect(ts).toBe(b);
  });

  it("히트_박스가_그림과_같은_좌표계다", () => {
    // 상자는 viewBox 단위다. viewBox가 바뀌면 상자가 통째로 어긋나는데,
    // 상수만 대조하면 둘이 사이좋게 틀린 채로 통과한다.
    const { container } = render(createElement(Penguin));
    expect(container.querySelector("svg")?.getAttribute("viewBox")).toBe(
      `0 0 ${PENGUIN_VIEWBOX.w} ${PENGUIN_VIEWBOX.h}`,
    );
  });

  it("히트_박스가_viewBox_안에_들어간다", () => {
    expect(PENGUIN_HIT_BOX.l).toBeGreaterThanOrEqual(0);
    expect(PENGUIN_HIT_BOX.t).toBeGreaterThanOrEqual(0);
    expect(PENGUIN_HIT_BOX.r).toBeLessThanOrEqual(PENGUIN_VIEWBOX.w);
    expect(PENGUIN_HIT_BOX.b).toBeLessThanOrEqual(PENGUIN_VIEWBOX.h);
  });
});

describe("CSS 변수가 팔레트와 같다", () => {
  // CSS는 TS를 못 부르므로 색이 양쪽에 따로 적힌다. 갈라져도 아무도 안 알려 준다.
  it.each([["pg-ink", "INK"]])("--%s 가 팔레트의 %s 와 같다", (cssName, tsName) => {
    const a = css.match(new RegExp(`--${cssName}:\\s*(#[0-9a-fA-F]{6})`))?.[1];
    const b = readFileSync(resolve(PALETTE_SRC), "utf8").match(
      new RegExp(`const ${tsName} = "(#[0-9a-fA-F]{6})"`),
    )?.[1];
    expect(a, `CSS에서 --${cssName}를 못 찾았다`).toBeDefined();
    expect(b, `팔레트에서 ${tsName}을 못 찾았다`).toBeDefined();
    expect(a).toBe(b);
  });
});

/** `const 이름: u64 = 1_100;` 에서 숫자만 꺼낸다 (밑줄 구분자를 지운다). */
function rustMs(name: string): number | null {
  const m = petRs.match(new RegExp(`const ${name}: u64 = ([0-9_]+)`));
  return m ? Number(m[1].replace(/_/g, "")) : null;
}

/** `VOLLEY_CHEER_MS`처럼 **다른 상수를 그대로 받는** 경우를 위한 대체 경로. */
function sassyMs(): number | null {
  return rustMs("SASSY_MS");
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
    ["pg--volley-bump", "VOLLEY_BUMP_MS"],
  ])("%s 가 Rust의 %s 와 같다", (cls, konst) => {
    const a = cssDurationMs(cls);
    const b = rustMs(konst);
    expect(a, `CSS에서 .${cls} .pg-all의 길이를 못 찾았다`).not.toBeNull();
    expect(b, `Rust에서 ${konst}를 못 찾았다`).not.toBeNull();
    expect(a).toBe(b);
  });
});

describe("움직임 감소", () => {
  /** `prefers-reduced-motion` 블록의 본문. **주석을 걷어낸다** — 안 그러면
   * 선택자를 지워도 주석에 남은 이름이 검사를 통과시킨다. */
  const 감소블록 = () =>
    css
      .match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}/)?.[1]
      .replace(/\/\*[\s\S]*?\*\//g, " ") ?? null;

  it("말풍선까지_선택자에_들어간다", () => {
    // 말풍선은 `.penguin`의 자손이 아니라 `.pg-stage`의 형제라, `.penguin *`로는
    // 한 번도 안 걸린다 — 튀어나오기 연출만 그대로 살아남는다.
    const 본문 = 감소블록();
    expect(본문, "prefers-reduced-motion 블록을 못 찾았다").not.toBeNull();
    expect(본문!).toContain(".pg-bubble");
  });

  it("시선은_완충이_아니라_아예_멈춘다", () => {
    // 전환만 끄면 눈동자가 커서를 툭툭 튀며 따라간다 — 완충을 없앤 것이지
    // 움직임을 없앤 게 아니다.
    expect(감소블록()!).toMatch(/\.pg-gaze\s*\{[^}]*transform:\s*none\s*!important/);
  });

  it("애니메이션과_전환을_함께_끈다", () => {
    // `animation`만 끄면 `.penguin`의 `transform 0.25s` 전환이 그대로 남아
    // 방향 전환이 여전히 회전한다 — 절반만 듣는 접근성 설정은 안 듣는 것보다 나쁘다.
    const 본문 = 감소블록();
    expect(본문, "prefers-reduced-motion 블록을 못 찾았다").not.toBeNull();
    expect(본문!).toMatch(/animation:\s*none\s*!important/);
    expect(본문!).toMatch(/transition:\s*none\s*!important/);
  });
});

describe("반복 횟수", () => {
  it("정수가_아닌_반복은_없다", () => {
    // 정수가 아니면 keyframe 중간에서 잘려 **자세가 중간에 멈춘 채 끝난다.**
    // `pg-butt-wiggle`이 0.7s × 1.3(=910ms)이라 30% 지점에서 잘렸다.
    const 어긋난: string[] = [];
    for (const m of css.matchAll(/animation:\s*([^;]+);/g)) {
      // `cubic-bezier(0.22, 1, 0.36, 1)`의 인자는 반복 횟수가 아니다.
      const 값 = m[1].replace(/\([^)]*\)/g, "");
      // 시간(`0.45s`)이 아닌 맨 소수가 반복 횟수다.
      for (const n of 값.matchAll(/(?<![\w.])(\d+\.\d+)(?![\w.]|s\b)/g)) {
        어긋난.push(`${값.trim()} (${n[1]})`);
      }
    }
    expect(어긋난, `정수가 아닌 반복 횟수: ${어긋난.join(" / ")}`).toEqual([]);
  });
});

describe("싸가지 반응 길이 동기화", () => {
  it("엉덩이_흔들기의_주기_×_횟수가_SASSY_MS와_같다", () => {
    // 기존 `동작 길이 동기화` 표는 `.pg-all`에 걸린 것만 본다. 싸가지 반응은
    // 부위별로 걸려서 **한 번도 대조된 적이 없었고**, 그래서 0.7 × 1.3(=910ms)이
    // 조용히 살아남았다. 정수 반복 검사와 짝이 되는 나머지 절반이다.
    const total = rustMs("SASSY_MS");
    expect(total, "Rust에서 SASSY_MS를 못 찾았다").not.toBeNull();
    const re =
      /\.pg--sassy-butt-wiggle\s+(\.[\w-]+)\s*\{[^}]*animation:\s*[\w-]+\s+([0-9.]+)s[^;]*?\s(\d+);/g;
    const 부위 = [...css.matchAll(re)];
    expect(부위.length, ".pg--sassy-butt-wiggle 부위 애니메이션을 못 찾았다").toBeGreaterThan(1);
    for (const [, sel, secs, count] of 부위) {
      const ms = Math.round(Number(secs) * 1000) * Number(count);
      expect(ms, `${sel} 의 주기 × 횟수가 SASSY_MS와 다르다`).toBe(total);
    }
  });
});

describe("비치발리볼", () => {
  it("이긴_쪽과_진_쪽은_싸가지_keyframe을_참조만_한다", () => {
    // 같은 이름을 두 번 정의하면 앞의 애니메이션이 통째로 죽는다. 재사용은
    // **참조**로 해야 하고, `volleyball.css`가 새로 정의하면 안 된다.
    const volley = readFileSync(resolve("src/pet/css/volleyball.css"), "utf8");
    for (const name of ["pg-butt-wiggle", "pg-turn-away", "pg-tail-wag", "pg-paddle"]) {
      expect(volley, `${name}을 다시 정의했다`).not.toContain(`@keyframes ${name}`);
      expect(volley, `${name}을 참조하지 않는다`).toContain(name);
    }
  });

  it("축하_길이가_Rust의_VOLLEY_CHEER_MS와_같다", () => {
    // 재사용한 keyframe이라 주기 × 반복이 상수와 맞아야 자세가 중간에 안 멈춘다.
    const total = rustMs("VOLLEY_CHEER_MS") ?? sassyMs();
    expect(total, "길이 상수를 못 찾았다").not.toBeNull();
    const re = /\.pg--volley-(?:cheer|sulk)\s+(\.[\w-]+)\s*\{[^}]*animation:\s*[\w-]+\s+([0-9.]+)s[^;]*?\s(\d+);/g;
    const 부위 = [...css.matchAll(re)];
    expect(부위.length, "축하·약오름 애니메이션을 못 찾았다").toBeGreaterThan(1);
    for (const [, sel, secs, count] of 부위) {
      const ms = Math.round(Number(secs) * 1000) * Number(count);
      expect(ms, `${sel} 의 주기 × 횟수가 다르다`).toBe(total);
    }
  });

  it("훌라_차림은_pg_all_안에_있다", () => {
    // 밖에 두면 착지 포즈에서 몸만 눌리고 옷이 허공에 남는다.
    //
    // 텍스트가 아니라 렌더한 DOM을 본다 — 부위가 다른 파일로 갈려 소스의
    // 등장 순서 비교가 성립하지 않는다.
    const { container } = render(createElement(Penguin));
    const all = container.querySelector(".pg-all");
    const luau = container.querySelector(".pg-luau");
    expect(all, ".pg-all을 못 찾았다").not.toBeNull();
    expect(luau, ".pg-luau를 못 찾았다").not.toBeNull();
    expect(all!.contains(luau!), "훌라 차림이 .pg-all 밖에 있다").toBe(true);
  });

  it("가까운쪽_날개가_훌라_위에_그려진다", () => {
    // 겹침 순서가 몸통 → 훌라 → 가까운쪽 날개 → 방망이라 장비가 한 덩어리로
    // 안 빠진다. 날개가 훌라보다 앞이면 어깨끈이 덮인다. 스냅샷도 잡지만
    // 무엇이 틀렸는지 말해 주는 것은 이 검사다.
    const { container } = render(createElement(Penguin));
    const 도형 = [...container.querySelectorAll(".pg-all > *")];
    const luau = 도형.findIndex((el) => el.classList.contains("pg-luau"));
    const wing = 도형.findIndex((el) => el.classList.contains("pg-wing-near"));
    expect(luau, ".pg-luau를 .pg-all 바로 아래에서 못 찾았다").toBeGreaterThan(-1);
    expect(wing, ".pg-wing-near를 .pg-all 바로 아래에서 못 찾았다").toBeGreaterThan(-1);
    expect(wing, "가까운쪽 날개가 훌라보다 먼저 그려진다 — 어깨끈이 덮인다").toBeGreaterThan(
      luau,
    );
  });

  it("상의는_암컷만_보인다", () => {
    // 수컷은 아래만 — 상체를 채우는 것은 레이 하나다.
    expect(css).toMatch(/\.pg-luau-top\s*\{[^}]*display:\s*none/);
    expect(css).toMatch(/\.pg-female\s+\.pg-luau-top\s*\{[^}]*display:\s*block/);
  });

  it("상의가_옷으로_읽히게_그려졌다", () => {
    // **덮개형으로 갔다가 "갑바"로 읽혀 비키니로 돌아왔다** (2026-09-02 사용자).
    // 문제는 노출량이 아니라 **옷으로 읽히느냐**였다 — 색이 몸에 가깝고 경계가
    // 없으면 아무리 덮어도 살로 보인다. 그래서 셋을 못 박는다.
    const svg = readFileSync(resolve(HULA_SRC), "utf8");
    const 상의 = svg.slice(
      svg.indexOf('<g className="pg-luau-top">'),
      svg.indexOf("</g>", svg.indexOf('<g className="pg-luau-top">')),
    );
    expect(상의.length, "상의 그림을 못 찾았다").toBeGreaterThan(0);

    // **도형 단위로 쪼갠다.** 아래 검사들이 문자 수나 전체 문자열로 세면
    // 도형 하나를 지워도 옆 도형이 대신 통과시킨다.
    const 도형 = 상의.split("<path").slice(1);

    // (1) 삼각형 두 개 — 닫힌 경로 둘.
    const 삼각형 = [...상의.matchAll(/d="(M[^"]*Z)"/g)].map((m) => m[1]);
    expect(삼각형.length, "삼각형 두 개가 아니다").toBe(2);

    // (2) **끈이 보인다** — 목뒤 V와 등뒤로 도는 가로줄. 사용자가 명시적으로
    // 요구한 형태이면서, 얇아도 옷으로 읽히게 하는 장치이기도 하다.
    //
    // **`stroke={STRAW_DARK}`를 세면 안 된다** — 삼각형의 테두리도 같은 값이라
    // 끈 둘을 통째로 지워도 2가 나와 통과한다(실제로 그랬다). 채우기 없이
    // 선만 있는 도형, 즉 **끈만** 센다.
    const 끈 = 도형.filter((d) => d.includes("stroke={STRAW_DARK}") && !d.includes("fill={"));
    expect(끈.length, "끈이 둘(목뒤 V·등뒤 가로줄)이 아니다").toBe(2);

    // (3) 도형마다 테두리가 있다 — 경계가 없으면 살로 읽힌다.
    // **문자 수로 잘라 보지 않는다** — 여유가 30자뿐이라 첫 삼각형의 테두리를
    // 지우면 두 번째 것을 읽어 통과한다.
    const 채운_도형 = 도형.filter((d) => d.includes("fill={STRAW}"));
    expect(채운_도형.length, "채워진 삼각형이 둘이 아니다").toBe(2);
    for (const d of 채운_도형) {
      expect(d, "테두리 없는 삼각형이 있다").toMatch(/strokeWidth=/);
    }
  });

  it("상의_삼각형이_얕다", () => {
    // **"둘 다 얇게"가 지시다.** 깊게 그리면 다시 덮개가 되고, 덮개는 갑바로
    // 읽혔다. 배(`SNOW` 타원, cy=82 ry=26 → y 56~108)의 위쪽 3분의 1 안에서
    // 끝나야 한다.
    const svg = readFileSync(resolve(HULA_SRC), "utf8");
    const 상의 = svg.slice(
      svg.indexOf('<g className="pg-luau-top">'),
      svg.indexOf("</g>", svg.indexOf('<g className="pg-luau-top">')),
    );
    const 삼각형 = [...상의.matchAll(/d="(M[^"]*Z)"/g)].map((m) => m[1]);
    expect(삼각형.length).toBe(2);
    for (const d of 삼각형) {
      // **`M`/`L`/`Z`로만 그린다.** 곡선(`C`·`Q`)이나 `H`/`V`를 허용하면 좌표를
      // 짝으로 읽는 아래 계산이 깨져 **깊이 0이 나오고 그냥 통과한다** — 곡선
      // 하나로 30 깊이의 덮개를 그려도 못 잡는다.
      expect(d, `삼각형에 M/L/Z 아닌 명령이 있다: ${d}`).toMatch(/^M[\s\d.LZ]+$/);
      const nums = [...d.matchAll(/[\d.]+/g)].map((m) => Number(m[0]));
      const ys = nums.filter((_, i) => i % 2 === 1);
      const 깊이 = Math.max(...ys) - Math.min(...ys);
      expect(깊이, `삼각형이 ${깊이} 로 깊다 — 덮개로 돌아간다`).toBeLessThan(16);
    }
  });

  it("옷_색이_몸_색과_대비된다", () => {
    // 배가 흰색이라 옅은 살구·크림 계열을 쓰면 옷이 아니라 살로 읽힌다.
    // **둘 다 본다** — 팔레트의 값이 대비되는가, 그림이 그 상수를 쓰는가.
    // 팔레트만 보면 그림에 흰색 리터럴을 박아도 통과한다.
    const 옷 = readFileSync(resolve(HULA_SRC), "utf8");
    for (const name of ["STRAW", "STRAW_DARK"]) {
      expect(옷, `훌라 차림이 ${name} 를 안 쓴다 — 색이 그림과 끊겼다`).toContain(
        `{${name}}`,
      );
    }
    // 리터럴 색을 직접 박으면 위 검사를 우회한다.
    const 리터럴 = [...옷.matchAll(/(?:fill|stroke)="(#[0-9a-fA-F]{3,8})"/g)].map((m) => m[1]);
    expect(리터럴, `훌라 차림에 팔레트를 안 거친 색이 있다: ${리터럴.join(", ")}`).toEqual([]);

    const palette = readFileSync(resolve(PALETTE_SRC), "utf8");
    const 색 = (name: string) => {
      const m = palette.match(new RegExp(`const ${name} = "(#[0-9a-fA-F]{6})"`));
      return m ? m[1] : null;
    };
    const 밝기 = (hex: string) =>
      (parseInt(hex.slice(1, 3), 16) +
        parseInt(hex.slice(3, 5), 16) +
        parseInt(hex.slice(5, 7), 16)) /
      3;
    const snow = 색("SNOW");
    expect(snow, "SNOW를 못 찾았다").not.toBeNull();
    // **상의도 하의도 같은 지푸라기다** — 재질이 하나라 볼 색도 하나다.
    const straw = 색("STRAW");
    expect(straw, "STRAW를 못 찾았다").not.toBeNull();
    expect(
      밝기(snow!) - 밝기(straw!),
      `STRAW(${straw})가 흰 배와 너무 가깝다 — 옷이 아니라 살로 읽힌다`,
    ).toBeGreaterThan(60);
  });

  it("중간에_썼던_이름이_안_남아있다", () => {
    // `pg-straw`는 덮개형으로 가던 시절의 이름이다.
    const svg = readFileSync(resolve(HULA_SRC), "utf8");
    const volley = readFileSync(resolve("src/pet/css/volleyball.css"), "utf8");
    for (const [이름, 본문] of [
      [HULA_SRC, svg],
      ["volleyball.css", volley],
    ] as const) {
      expect(본문, `${이름}에 pg-straw가 남아 있다`).not.toContain("pg-straw");
    }
  });
});

describe("클릭은 칠해진 자리에서만 받는다", () => {
  /** 선택자 목록에서 `pointer-events` 값을 꺼낸다. */
  function 포인터규칙(): { sel: string; value: string }[] {
    const found: { sel: string; value: string }[] = [];
    for (const m of css.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
      if (m[1].includes("@")) continue;
      const v = m[2].match(/pointer-events:\s*([\w-]+)/);
      if (v) found.push({ sel: m[1].trim(), value: v[1] });
    }
    return found;
  }

  it("펭귄_루트가_클릭을_안_받는다", () => {
    // 루트는 한 변 --pg-size인 사각형이라 그대로 두면 무대 안 빈 자리가
    // 전부 펭귄이 된다. 사용자가 겪은 "안 눌렀는데 눌린다"가 이것이다.
    const rule = 포인터규칙().find((r) => /^\.penguin$/m.test(r.sel));
    expect(rule, ".penguin 에 pointer-events 규칙이 없다").toBeDefined();
    expect(rule?.value).toBe("none");
  });

  it("도형이_클릭을_되살린다", () => {
    // 루트에서만 끄면 펭귄을 아예 못 누른다.
    const rule = 포인터규칙().find(
      (r) => r.sel.startsWith(".penguin ") && /\bpath\b/.test(r.sel),
    );
    expect(rule, "도형에 pointer-events를 되살리는 규칙이 없다").toBeDefined();
    expect(rule?.value).not.toBe("none");
  });

  /** 네이티브 히트 상자(`pet_bridge/hit.rs`) 밖으로 나가거나 안 보이는 채로
   * 클릭을 먹는 것들. 한쪽 층만 빼면 "웹뷰는 반응하는데 창은 통과시키는"
   * 갈래가 생긴다. */
  const 클릭_없는_부위 = ["pg-bat", "pg-rod", "pg-line", "pg-float", "pg-fish", "pg-hole"];

  it("바닥_그림자는_클릭을_받는다", () => {
    // 그림자는 히트 상자 **안**이다. 빼면 창은 클릭을 먹는데 아무도 반응하지
    // 않는 띠가 발밑에 생긴다 — 사용자가 겪은 것과 같은 증상이다.
    const 뺀_규칙 = 포인터규칙().filter(
      (r) => /\.pg-shadow(?![\w-])/.test(r.sel) && r.value === "none",
    );
    expect(뺀_규칙, ".pg-shadow 가 클릭에서 빠져 있다").toHaveLength(0);
  });

  it.each(클릭_없는_부위.map((c) => [c] as const))("%s 는 클릭을 안 받는다", (cls) => {
    const 규칙 = 포인터규칙().filter(
      (r) => new RegExp(`\\.${cls}(?![\\w-])`).test(r.sel) && r.value === "none",
    );
    expect(
      규칙.length,
      `.${cls} 를 클릭에서 빼는 pointer-events: none 규칙이 없다`,
    ).toBeGreaterThan(0);
  });

  it("방망이는_opacity가_아니라_pointer_events로_뺀다", () => {
    // `.pg-bat`은 `opacity: 0`으로 감춰지는데 **opacity는 히트 테스트를 안 막는다**
    // (`visiblePainted`가 보는 것은 visibility다). 이 검사가 그 착각을 막는다.
    const 규칙 = 포인터규칙().filter(
      (r) => /\.pg-bat(?![\w-])/.test(r.sel) && r.value === "none",
    );
    expect(규칙.length).toBeGreaterThan(0);
  });
});

describe("평소 숨기는 도형", () => {
  const 숨기는_도형 = [
    "pg-hole",
    "pg-rod",
    "pg-line",
    "pg-float",
    "pg-fish",
    "pg-beak-lower",
    "pg-luau",
  ];

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
    expect(hasRule(css, cls), `.${cls} 규칙이 없다`).toBe(true);
  });
});
