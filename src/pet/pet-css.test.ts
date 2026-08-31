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
/** 코어가 낼 수 있는 모든 동작. 코어에 추가하면 여기도 늘려야 한다. */
// vitest는 프로젝트 루트에서 돈다. `?raw`는 vitest의 CSS 처리에 걸려 원본을
// 주지 않으므로 파일을 직접 읽는다 (그래서 @types/node가 dev 의존성에 있다)
const css = readFileSync(resolve("src/pet/pet.css"), "utf8");
// 상수가 어느 모듈에 있든 값만 맞으면 된다 — PET_SIZE는 기준점 계산 때문에
// 코어(pet.rs)로 옮겼고 여백은 창을 만드는 브릿지에 남았다. 둘 다 읽는다.
const petRs =
  readFileSync(resolve("src-tauri/src/pet.rs"), "utf8") +
  readFileSync(resolve("src-tauri/src/pet_bridge.rs"), "utf8");
// 화면에 그리는 곳이 둘이다 — 한쪽만 보면 옮겨간 클래스를 놓친다
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
    // level은 기본값이라 규칙이 없어도 된다 — 기울이지 않는 것이 정상이다
    for (const v of ["up", "down"] as Vertical[]) {
      expect(css).toContain(`.${verticalClass(v)}`);
    }
  });

  it("CSS에_없어진_동작의_잔재가_남아있지_않다", () => {
    // Startled는 Sassy로 대체됐다. 죽은 규칙이 남으면 다음 사람이 헷갈린다
    expect(css).not.toContain("pg--startled");
  });

  it("같은_이름의_keyframes가_두_번_정의되지_않는다", () => {
    // 실제로 겪은 사고: 던져짐의 회전이 `pg-tumble`이라는 이름을 굴러떨어지기와
    // 나눠 써서, 나중 정의가 이겨 **굴러떨어지기 그림이 통째로 죽어 있었다.**
    // 길이 동기화 가드로도 안 잡힌다 — 길이는 각자 맞기 때문이다
    const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    const 중복 = defined.filter((n, i) => defined.indexOf(n) !== i);
    expect(중복, `중복 정의된 @keyframes: ${중복.join(", ")}`).toEqual([]);
  });

  it("정의된_keyframes가_모두_쓰인다", () => {
    const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    expect(defined.length).toBeGreaterThan(0);
    for (const name of defined) {
      // \b는 하이픈 앞에서도 성립해 pg-turn이 pg-turn-away 안에서 잡힌다.
      // 이름 뒤에 이어지는 글자·하이픈이 없어야 진짜 그 이름을 쓴 것이다
      const uses = countExact(css, name);
      expect(uses, `@keyframes ${name}가 정의만 되고 쓰이지 않는다`).toBeGreaterThan(1);
    }
  });
});

describe("창 여백 상수 동기화", () => {
  // 창은 펭귄보다 크다(말풍선·방망이 자리). Rust는 창을 그만큼 물려 놓고,
  // CSS는 그만큼 안으로 들여 펭귄을 그린다. 둘이 어긋나면 펭귄이 화면
  // 경계에서 엉뚱한 자리에 서는데, 눈으로만 잡힌다.
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
  // 코어가 정한 길이와 CSS 길이가 어긋나도 **아무것도 실패하지 않는다.**
  // 짧으면 다 눕기 전에 애니메이션이 끝나 자세가 튀고, 길면 이미 일어나
  // 걷는 펭귄이 아직 넘어져 있다. 둘 다 눈으로만 잡힌다.
  it("굴러떨어지기가 Rust의 TUMBLE_MS와 같다", () => {
    const a = cssDurationMs("pg--tumble");
    const b = rustMs("TUMBLE_MS");
    expect(a, "CSS에서 .pg--tumble .pg-all의 길이를 못 찾았다").not.toBeNull();
    expect(b, "Rust에서 TUMBLE_MS를 못 찾았다").not.toBeNull();
    expect(a).toBe(b);
  });

  // 드리우기(wait)는 입질을 기다리는 무한 반복이라 대조 대상이 아니다 —
  // 길이가 코어의 4~9초 추첨값이지 고정 상수가 아니다.
  it.each([
    ["pg--fishing-dig", "FISHING_DIG_MS"],
    ["pg--fishing-bite", "FISHING_BITE_MS"],
    ["pg--fishing-catch", "FISHING_CATCH_MS"],
    ["pg--fishing-miss", "FISHING_MISS_MS"],
    ["pg--fishing-pack", "FISHING_PACK_MS"],
    ["pg--slide", "SLIDE_MS"],
    ["pg--squawk", "SQUAWK_MS"],
  ])("%s 가 Rust의 %s 와 같다", (cls, konst) => {
    const a = cssDurationMs(cls);
    const b = rustMs(konst);
    expect(a, `CSS에서 .${cls} .pg-all의 길이를 못 찾았다`).not.toBeNull();
    expect(b, `Rust에서 ${konst}를 못 찾았다`).not.toBeNull();
    expect(a).toBe(b);
  });
});

describe("평소 숨기는 도형", () => {
  // **실제로 밟은 함정이다.** 후광은 같은 도형을 한 벌 더 그리는데,
  // `.pg-halo :is(path, circle, ellipse, rect) { opacity: 1 }`이 도형 클래스보다
  // 구체적이라 `opacity: 0`으로 감추면 **후광 한 벌이 화면에 그대로 남는다.**
  // 눈으로만 잡히고 커버리지 테스트도 통과한다 — `display: none`이어야 한다.
  const 숨기는_도형 = ["pg-hole", "pg-rod", "pg-line", "pg-float", "pg-fish", "pg-beak-lower"];

  /** 선택자에 이 클래스가 정확히 등장하는 모든 규칙 블록의 본문.
   *
   * **주석을 먼저 걷어낸다.** 안 걷어내면 주석에 적힌 클래스 이름이 바로 뒤
   * 규칙의 선택자로 읽혀 엉뚱한 규칙이 걸린다 — 이 테스트를 쓰다 실제로 겪었다. */
  function 규칙들(cls: string): string[] {
    const 본문 = css.replace(/\/\*[\s\S]*?\*\//g, " ");
    const found: string[] = [];
    for (const m of 본문.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
      // `@keyframes` 안쪽 블록과 미디어 쿼리 헤더는 선택자가 아니다
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
  // 부위 애니메이션은 짧은 주기를 여러 번 돌린다. **주기 × 횟수**가 코어의
  // `SQUAWK_MS`와 어긋나도 아무것도 실패하지 않는다 — 짧으면 날개가 먼저 멈춘 채
  // 몸만 가라앉고, 길면 퍼덕이다 잘린다. 위의 "동작 길이 동기화"는 `.pg-all`만 본다.
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
  // 실제로 겪은 사고: 말풍선·방망이를 그려 놓고 CSS를 빠뜨렸다. 아무 테스트도
  // 실패하지 않았고, 방망이가 거대한 정지 이미지로 화면에 남았다.
  // 동작 클래스(pg--)는 위에서 따로 보고, 여기서는 UI 클래스(pg-)를 본다.
  const used = new Set<string>();
  for (const m of petApp.matchAll(/\bpg-[a-z0-9-]+/g)) {
    const cls = m[0];
    // 동작·상태 클래스는 코어가 만들어 위 테스트가 담당한다
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
