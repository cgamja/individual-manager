import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * 코트·공 웹뷰는 상태가 거의 없다 — 코트는 그림뿐이고, 공은 클래스 하나를
 * 켜고 끈다. 그래서 여기서 지키는 것은 **그림이 실제로 그려지는가**와
 * **클릭을 통과시키는가** 둘이다.
 */

const 리스너: Array<(ball: { x: number; y: number; flying: boolean }) => void> = [];

vi.mock("../lib/pet", () => ({
  onVolleyState: (cb: (b: { x: number; y: number; flying: boolean }) => void) => {
    리스너.push(cb);
    return Promise.resolve(() => {});
  },
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
    expect(root.innerHTML).toContain("vb-sand");
    expect(root.querySelector(".vb-net-mesh")).not.toBeNull();
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
