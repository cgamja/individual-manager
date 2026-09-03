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
  onPetScale: () => Promise.resolve(() => {}),
}));

vi.mock("../lib/settings", () => ({
  loadPetSettings: () => Promise.resolve({ size: 100 }),
  initialScale: () => 1,
}));

const courtCss = readFileSync(resolve("src/volley/court.css"), "utf8");
const ballCss = readFileSync(resolve("src/volley/ball.css"), "utf8");
const ballTs = readFileSync(resolve("src/volley/ball.ts"), "utf8");
/** 주석을 걷어낸다 — `court.ts` 머리말에 `preserveAspectRatio="none"`이 적혀
 * 있어서 SVG에서 지워도 통과했다.
 * `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만 = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ");
/** **마운트 파일**. 그림은 여기 없다 — 구독하지 않는다는 것만 여기서 본다. */
const courtTs = 코드만(readFileSync(resolve("src/volley/court.ts"), "utf8"));
/** **그림 파일**. 치수·경로를 대조하는 검사들이 읽는다. */
const courtSvg = 코드만(readFileSync(resolve("src/assets/props/court.ts"), "utf8"));

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
    expect(courtSvg).toContain('preserveAspectRatio="none"');
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

describe("코트 크기 배율", () => {
  it("코트도_무대처럼_통째로_줄인다", () => {
    // 네트·모래가 고정 px라 창만 줄이면 코트 안에서 넘친다.
    expect(courtCss, "--pg-scale로 스케일하지 않는다").toMatch(
      /#court-root\s*\{[^}]*transform:\s*scale\(var\(--pg-scale/,
    );
    expect(courtCss, "transform-origin이 좌상단이 아니다").toMatch(
      /#court-root\s*\{[^}]*transform-origin:\s*0 0/,
    );
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
    // **상수가 아니라 식이다** — `VOLLEY_NET_HEIGHT = PET_SIZE - VOLLEY_NET_DROP`.
    // 그물이 모래에 발을 딛는 관계를 구조로 못 박은 결과라, 여기서도 같은
    // 관계로 계산해야 CSS가 따라온다.
    const pet = rustF64("PET_SIZE");
    const drop = rustF64("VOLLEY_NET_DROP");
    expect(pet, "PET_SIZE를 못 찾았다").not.toBeNull();
    expect(drop, "VOLLEY_NET_DROP을 못 찾았다").not.toBeNull();
    expect(cssVar("vb-net-h")).toBe(pet! - drop!);
  });

  it("네트_폭이_VOLLEY_NET_HALF_W의_두_배다", () => {
    const half = rustF64("VOLLEY_NET_HALF_W");
    expect(half).not.toBeNull();
    expect(cssVar("vb-net-w")).toBe(half! * 2);
  });

  it("모래_깊이가_VOLLEY_SAND_DEPTH와_같다", () => {
    expect(cssVar("vb-sand-depth")).toBe(rustF64("VOLLEY_SAND_DEPTH"));
  });

  it("네트_viewBox가_요소_크기와_같다", () => {
    const m = courtSvg.match(/class="vb-net" viewBox="0 0 (\d+) (\d+)"/);
    expect(m, "네트 viewBox를 못 찾았다").not.toBeNull();
    expect(Number(m![1])).toBe(cssVar("vb-net-w"));
    expect(Number(m![2])).toBe(cssVar("vb-net-h"));
  });

  it("모래_viewBox가_요소_높이와_같다", () => {
    // 세로로 안 늘이므로(높이가 CSS로 고정이다) viewBox 세로와 CSS 높이가 같아야
    // 물결선이 정확히 모래 표면에 온다.
    const m = courtSvg.match(/class="vb-sand" viewBox="0 0 \d+ (\d+)"/);
    expect(m, "모래 viewBox를 못 찾았다").not.toBeNull();
    expect(Number(m![1])).toBe(cssVar("vb-sand-wave")! + cssVar("vb-sand-depth")!);
  });

  it("물결이_착지면을_감싼다", () => {
    // **공이 모래 위에 떠 보이면 안 된다.** 물결을 착지면 아래에만 그리면
    // 그렇게 된다 — 실제로 1~18px 떠 있었다. 착지면(viewBox y = wave)이
    // 물결의 위아래 사이에 있어야 한다.
    const 착지면 = cssVar("vb-sand-wave")!;
    const d = courtSvg.match(/d="(M0 [^"]*?)"/);
    expect(d, "모래 경로를 못 찾았다").not.toBeNull();

    // `Q` 구간마다 **실제 곡선**의 y 범위를 구한다 — 앵커만 보면 제어점이 만드는
    // 오버슈트를 놓친다.
    const 앞 = d![1].split("L")[0];
    const nums = [...앞.matchAll(/-?\d+(?:\.\d+)?/g)].map((m) => Number(m[0]));
    const ys: number[] = [];
    let y0 = nums[1];
    for (let i = 2; i + 3 < nums.length; i += 4) {
      const cy = nums[i + 1];
      const y2 = nums[i + 3];
      for (let k = 0; k <= 20; k += 1) {
        const t = k / 20;
        ys.push((1 - t) ** 2 * y0 + 2 * (1 - t) * t * cy + t ** 2 * y2);
      }
      y0 = y2;
    }
    expect(ys.length, "곡선 구간을 못 읽었다").toBeGreaterThan(20);
    expect(Math.min(...ys), "물결이 착지면 위로 안 올라온다").toBeLessThan(착지면);
    expect(Math.max(...ys), "물결이 착지면 아래로 안 내려간다").toBeGreaterThan(착지면);
  });

  it("모래_경로에_T_명령이_없다", () => {
    // `T`는 제어점이 반사돼 누적되면서 실제 곡선이 앵커가 말하는 범위를 넘는다 —
    // 3~12로 적어 두고 1~18로 그려졌다. 구간마다 `Q`로 제어점을 직접 준다.
    const d = courtSvg.match(/d="(M0 [^"]*?)"/);
    expect(d![1]).not.toMatch(/\bT\b/);
  });
});

describe("웹뷰 규약", () => {
  it("공은_자기_창에만_묶인다", () => {
    // 전역 `listen()`은 대상을 `Any`로 등록해 emit 대상과 무관하게 전부
    // 호출된다 — 창이 여럿이면 그때 터진다.
    // **주석을 걷어낸다** — 안 그러면 "쓰지 말라"고 적어 둔 주석이 검사에 걸린다.
    const 코드 = ballTs.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ");
    expect(코드).toContain("onVolleyState");
    expect(코드).not.toMatch(/\blisten\s*\(/);
  });

  it("코트는_구독하지_않는다", () => {
    // 코트는 판이 도는 동안 안 변한다 — 창의 존재 자체가 상태다.
    expect(courtTs).not.toContain("onVolleyState");
    // 전역 `listen(`도 막는다 — 대상을 `Any`로 등록해 emit 대상과 무관하게
    // 전부 호출된다 (위 `공은_자기_창에만_묶인다`와 같은 위험).
    expect(courtTs).not.toMatch(/\blisten\s*\(/);
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
