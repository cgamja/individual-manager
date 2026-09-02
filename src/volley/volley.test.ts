import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * 코트·공 웹뷰는 상태가 거의 없다 — 코트는 그림뿐이고, 공은 클래스 하나를
 * 켜고 끈다. 그래서 여기서 지키는 것은 **그림이 실제로 그려지는가**와
 * **클릭을 통과시키는가** 둘이다.
 */

type Ball = { x: number; y: number; flying: boolean };

const 리스너: Array<(ball: Ball) => void> = [];
/** 창이 뜰 때 한 번 끌어오는 첫 상태. `null`이면 아직 판이 없다. */
let 첫_상태: Ball | null = null;

vi.mock("../lib/pet", () => ({
  onVolleyState: (cb: (b: Ball) => void) => {
    리스너.push(cb);
    return Promise.resolve(() => {});
  },
  getVolleyState: () => Promise.resolve(첫_상태),
}));

const courtCss = readFileSync(resolve("src/volley/court.css"), "utf8");
const ballCss = readFileSync(resolve("src/volley/ball.css"), "utf8");
const ballTs = readFileSync(resolve("src/volley/ball.ts"), "utf8");
const courtTs = readFileSync(resolve("src/volley/court.ts"), "utf8");

describe("코트", () => {
  beforeEach(() => {
    vi.resetModules();
    document.body.innerHTML = '<div id="court-root"></div>';
  });

  it("모래와_네트가_그려진다", async () => {
    await import("./court");
    const root = document.getElementById("court-root")!;
    expect(root.querySelectorAll("svg").length).toBe(2);
    // 모래는 그러데이션, 네트는 그물이다.
    expect(root.querySelector(".vb-sand")).not.toBeNull();
    expect(root.querySelector(".vb-net-mesh")).not.toBeNull();
  });

  it("모래는_창_바닥에_네트는_창_위_한가운데에_붙는다", () => {
    // **판이 화면 세로 중앙으로 올라가면서 둘이 갈라졌다.** 창 하나가 둘을
    // 함께 덮으므로(`Court::rect`) 각자 자기 변에 붙어야 자리가 맞는다.
    expect(courtCss).toMatch(/\.vb-sand\s*\{[^}]*bottom:\s*0/);
    expect(courtCss).toMatch(/\.vb-net\s*\{[^}]*top:\s*0/);
    // 창이 `net_cx`를 중심으로 대칭이라 좌표 없이 50%로 맞는다.
    expect(courtCss).toMatch(/\.vb-net\s*\{[^}]*left:\s*50%/);
  });

  it("모래는_가로로만_늘어난다", () => {
    // 세로까지 늘면 넓은 화면에서 모래가 두꺼워진다.
    expect(courtTs).toContain('preserveAspectRatio="none"');
    expect(courtCss).toMatch(/--vb-sand-depth/);
  });

  it("붙일_자리가_없으면_터지지_않는다", async () => {
    document.body.innerHTML = "";
    await expect(import("./court")).resolves.toBeDefined();
  });
});

describe("비치볼", () => {
  beforeEach(() => {
    vi.resetModules();
    리스너.length = 0;
    첫_상태 = null;
    document.body.innerHTML = '<div id="vball-root"></div>';
  });

  it("공이_그려진다", async () => {
    await import("./ball");
    expect(document.querySelector(".vb-ball")).not.toBeNull();
  });

  it("날아가는_동안만_도는_클래스가_붙는다", async () => {
    await import("./ball");
    const root = document.getElementById("vball-root")!;
    expect(리스너.length).toBe(1);
    리스너[0]({ x: 0, y: 0, flying: true });
    expect(root.classList.contains("vb-ball--flying")).toBe(true);
    리스너[0]({ x: 0, y: 0, flying: false });
    expect(root.classList.contains("vb-ball--flying")).toBe(false);
  });

  it("창이_뜨자마자_첫_상태를_끌어온다", async () => {
    // **구독만으로는 첫 상태가 안 온다.** 틱이 이 창을 만들고 **같은 호출에서**
    // 첫 상태를 보내는데 그때 이 파일은 아직 실행되지도 않았다. 그 뒤로는
    // "달라진 게 없다"로 걸러져 다시 안 오므로, 끌어오지 않으면 공이 판 내내
    // 안 돈다.
    첫_상태 = { x: 0, y: 0, flying: true };
    await import("./ball");
    const root = document.getElementById("vball-root")!;
    await vi.waitFor(() =>
      expect(root.classList.contains("vb-ball--flying")).toBe(true),
    );
  });
});

describe("클릭을 통과시킨다", () => {
  // **진짜 방어선은 Rust의 `set_ignore_cursor_events`지만 그 세터가 비동기라**
  // 창이 뜬 직후 한 프레임쯤은 아직 안 걸려 있을 수 있다. CSS는 첫 프레임부터
  // 듣는다 — 구간이 서로 달라서 겹쳐 둔 이중 방어이고, 여기서는 CSS 쪽을 지킨다.
  it.each([
    ["코트", courtCss, "#court-root"],
    ["비치볼", ballCss, "#vball-root"],
  ])("%s 은 pointer-events: none 이다", (_이름, css, 선택자) => {
    const re = new RegExp(`\\${선택자}[^{]*\\{[^}]*pointer-events:\\s*none`);
    expect(css).toMatch(re);
    // 자손까지 걸어야 안쪽 SVG가 클릭을 안 먹는다.
    expect(css).toContain(`${선택자} *`);
  });

  it("커서를_바꾸지_않는다", () => {
    // 사용자가 만지는 물건이 아니다 — 볼링 공(`cursor: grab`)과 정반대다.
    expect(courtCss).not.toMatch(/cursor:/);
    expect(ballCss).not.toMatch(/cursor:/);
  });
});

describe("코트 CSS가 Rust 상수와 같다", () => {
  // 창 크기는 Rust가 정하고 그림은 CSS가 그리므로, 둘이 어긋나면 네트가 창
  // 밖으로 나가거나 모래 선이 실제 착지 높이와 다른 자리에 그려진다.
  const tuning = readFileSync(resolve("src-tauri/src/pet/tuning.rs"), "utf8");

  /** `pub(super) const 이름: f64 = 123.0;` 에서 숫자만 꺼낸다. */
  function rustF64(name: string): number | null {
    const m = tuning.match(new RegExp(`const ${name}: f64 = ([0-9._]+)`));
    return m ? Number(m[1].replace(/_/g, "")) : null;
  }

  /** `--이름: 123px;` 에서 숫자만 꺼낸다. */
  function cssVar(name: string): number | null {
    const m = courtCss.match(new RegExp(`--${name}:\\s*([0-9.]+)px`));
    return m ? Number(m[1]) : null;
  }

  it("네트_높이가_VOLLEY_NET_HEIGHT와_같다", () => {
    expect(cssVar("vb-net-h")).toBe(rustF64("VOLLEY_NET_HEIGHT"));
  });

  it("네트_폭이_VOLLEY_NET_HALF_W의_두_배다", () => {
    const half = rustF64("VOLLEY_NET_HALF_W");
    expect(half).not.toBeNull();
    expect(cssVar("vb-net-w")).toBe(half! * 2);
  });

  it("모래_깊이가_VOLLEY_SAND_DEPTH와_같다", () => {
    expect(cssVar("vb-sand-depth")).toBe(rustF64("VOLLEY_SAND_DEPTH"));
  });
});

describe("웹뷰 규약", () => {
  it("공은_자기_창에만_묶인다", () => {
    // 전역 `listen()`은 대상을 `Any`로 등록해 emit 대상과 무관하게 전부
    // 호출된다 — 창이 여럿이면 그때 터진다.
    // **주석을 걷어낸다** — 안 그러면 "쓰지 말라"고 적어 둔 주석이 검사에 걸린다.
    const 코드 = ballTs.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ");
    expect(코드).toContain("onVolleyState");
    expect(코드).not.toMatch(/\blisten\(/);
  });

  it("코트는_구독하지_않는다", () => {
    // 코트는 판이 도는 동안 안 변한다 — 창의 존재 자체가 상태다.
    expect(courtTs).not.toContain("onVolleyState");
  });

  it("배경을_칠하지_않는다", () => {
    // 조금이라도 칠하면 둘레에 네모가 보인다.
    for (const css of [courtCss, ballCss]) {
      expect(css).toContain("background: transparent");
    }
  });

  it("움직임_감소를_지킨다", () => {
    for (const css of [courtCss, ballCss]) {
      expect(css).toMatch(/animation:\s*none\s*!important/);
      expect(css).toMatch(/transition:\s*none\s*!important/);
    }
  });

  it("같은_이름의_keyframes가_두_번_정의되지_않는다", () => {
    const 전부 = courtCss + "\n" + ballCss;
    const defined = [...전부.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
    const 중복 = defined.filter((n, i) => defined.indexOf(n) !== i);
    expect(중복, `중복 정의된 @keyframes: ${중복.join(", ")}`).toEqual([]);
  });
});
