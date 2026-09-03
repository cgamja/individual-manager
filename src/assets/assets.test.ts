import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render } from "@testing-library/react";
import { createElement } from "react";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Penguin } from "./penguin";
import { BEACH_BALL_SVG } from "./props/beach-ball";
import { batCursorUrl } from "./props/bat";
import { BOWLING_BALL_SVG } from "./props/bowling-ball";
import { NET_SVG, SAND_SVG } from "./props/court";

/**
 * **그림의 정답지.** 에셋을 `src/assets/`로 옮기는 동안 그림이 한 픽셀도 안 바뀌었음을
 * 증명하는 유일한 그물이다.
 *
 * 두 러너와 타입 검사가 **전부 통과하면서 그림만 조용히 바뀔 수 있다** — 지금 있는
 * 그림 검사는 훌라 상의 하나뿐이고, 나머지 도형의 `d` 하나가 틀려도 아무도 모른다.
 * 이 레포가 이미 겪은 실패다
 * (`docs/solutions/ui-bugs/duplicate-keyframes-silently-kills-animation.md` —
 * 굴러떨어지기 그림이 한 PR 내내 죽어 있었고 모든 게이트가 통과했다).
 *
 * **리팩터링용 임시 비계가 아니다.** 끝나고도 남겨서 그림의 회귀 그물로 쓴다.
 * 스냅샷을 `--update`로 덮는 것은 "그림을 바꾸겠다"는 선언이지 통과시키는 방법이 아니다.
 */

afterEach(cleanup);

describe("펭귄 그림", () => {
  /** 렌더된 펭귄의 마크업. `female`이 훌라 상의를 켠다. */
  function 그린다(female: boolean): string {
    const { container } = render(createElement(Penguin, female ? { female: true } : {}));
    return container.innerHTML;
  }

  it("수컷_펭귄_그림이_그대로다", () => {
    expect(그린다(false)).toMatchSnapshot();
  });

  it("암컷_펭귄_그림이_그대로다", () => {
    expect(그린다(true)).toMatchSnapshot();
  });

  it("암수가_서로_다른_그림이다", () => {
    // 위 둘이 같은 스냅샷으로 굳으면 `female` 분기가 죽어도 통과한다.
    // 상의는 CSS(`.pg-female .pg-luau-top`)가 보이게 하므로 **마크업 차이는
    // 클래스 하나뿐이고**, 그 하나가 사라지는 것이 정확히 이 검사가 막는 것이다.
    expect(그린다(false)).not.toBe(그린다(true));
  });
});

describe("소품 그림", () => {
  it("비치볼_그림이_그대로다", () => {
    expect(BEACH_BALL_SVG).toMatchSnapshot();
  });

  it("모래사장_그림이_그대로다", () => {
    expect(SAND_SVG).toMatchSnapshot();
  });

  it("네트_그림이_그대로다", () => {
    expect(NET_SVG).toMatchSnapshot();
  });
});

describe("볼링공 그림", () => {
  it("볼링공_그림이_그대로다", () => {
    expect(BOWLING_BALL_SVG).toMatchSnapshot();
  });

  it("볼링공이_창에_실제로_그려진다", async () => {
    // **정답지가 맞다고 화면에 뜨는 것은 아니다.** 상수만 스냅샷하면
    // `root.innerHTML`이 그 상수를 안 쓰거나 클래스를 바꿔 넣어도 통과한다 —
    // 그러면 공이 `ball.css`의 크기·커서·구르기 규칙을 통째로 잃는다.
    // 코트·비치볼은 `volley.test.ts`가 같은 것을 지킨다.
    vi.resetModules();
    document.body.innerHTML = '<div id="ball-root"></div>';
    mockWindows("bowling-ball");
    (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
    mockIPC((cmd) => (cmd === "plugin:event|listen" ? 1 : undefined));
    await import("../ball/main");
    const ball = document.querySelector("#ball-root .bw-ball");
    expect(ball, "#ball-root 안에 .bw-ball이 안 그려졌다").not.toBeNull();
    // 같은 방식으로 파싱해 대조한다 — 문자열을 직접 비교하면 DOM이 자기닫는
    // 태그를 펼치는 것 때문에 항상 다르다.
    const 정답 = document.createElement("div");
    정답.innerHTML = BOWLING_BALL_SVG;
    expect(ball!.outerHTML, "그려진 것이 BOWLING_BALL_SVG가 아니다").toBe(
      정답.querySelector(".bw-ball")!.outerHTML,
    );
  });
});

describe("커서 방망이", () => {
  it("커서_방망이_그림이_그대로다", () => {
    // **소품 중 유일하게 정답지가 없던 자리다.** 그림·회전 기준점·색이 바뀌어도
    // 두 러너가 전부 통과했다 — 커서는 jsdom이 렌더하지 않아 다른 그물이 없다.
    expect(batCursorUrl(55)).toMatchSnapshot();
    expect(batCursorUrl(-40)).toMatchSnapshot();
  });
});

describe("소품은 React를 쓰지 않는다", () => {
  // **바닐라 창(핀볼·볼링공·코트·비치볼)이 실수로 React를 끌어오면 그 번들에
  // React가 통째로 들어간다** (KTD7·KTD8). 아무도 안 알려 준다.
  //
  // **첫 판은 헛돌았다.** `.ts`만 걸러 `/from "react/`를 봤는데, 위반의 실제
  // 모양은 `.tsx`(JSX를 쓰려는 사람이 만든다)이고 홑따옴표·`require`·부수효과
  // import·`penguin/`을 거친 간접 경로가 전부 통과했다.
  const dir = resolve("src/assets/props");

  it("소품에는_React가_없다", () => {
    const files = readdirSync(dir);
    expect(files.length, "src/assets/props 가 비었다").toBeGreaterThan(0);
    // **확장자를 안 거른다** — `.tsx`가 여기 생기는 것 자체가 위반이다.
    for (const f of files) {
      expect(f, `${f} — 소품에 JSX 파일이 있다. props는 바닐라다`).not.toMatch(/\.tsx$/);
    }
    for (const f of files.filter((n) => n.endsWith(".ts"))) {
      const 코드 = readFileSync(join(dir, f), "utf8")
        .replace(/\/\*[\s\S]*?\*\//g, " ")
        .replace(/\/\/.*/g, " ");
      // 따옴표 종류·`require`·부수효과 import를 전부 덮는다.
      expect(코드, `${f} 가 React를 끌어온다`).not.toMatch(/['"]react['"/]/);
      // **간접 경로도 막는다** — `penguin/`은 TSX라 그걸 import하면 React가 딸려온다.
      expect(코드, `${f} 가 penguin/을 거쳐 React를 끌어온다`).not.toMatch(
        /from ["'][^"']*penguin/,
      );
    }
  });
});
