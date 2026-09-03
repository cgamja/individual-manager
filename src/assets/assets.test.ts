import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render } from "@testing-library/react";
import { createElement } from "react";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Penguin } from "./penguin";
import { BEACH_BALL_SVG } from "./props/beach-ball";
import {
  BAT_BASE_PX,
  BAT_CURSOR_VAR,
  BAT_SWING_VAR,
  batCursorUrl,
  batCursorValue,
  CURSOR_MAX_PX,
  installBatCursor,
} from "./props/bat";
import { SIZE_MAX } from "../lib/settings";
import { BOWLING_BALL_SVG } from "./props/bowling-ball";
import { NET_SVG, SAND_SVG } from "./props/court";

/**
 * 그림의 정답지 — 도형·좌표·겹침 순서를 마크업으로 굳혀 둔다.
 *
 * 두 러너와 타입 검사가 전부 통과하면서 그림만 바뀔 수 있다. `-u`로 덮는 것은
 * "그림을 바꾸겠다"는 선언이지 통과시키는 방법이 아니다.
 *
 * **CSS는 안 덮는다** — `@keyframes`가 겹쳐 애니메이션이 죽는 것은 여기서 안 잡힌다.
 */

/** 태그마다 줄을 나눈다. 한 줄로 두면 속성 하나만 바뀌어도 diff가 14KB짜리
 * 통짜 줄 교체로 나와, `-u`가 "그림을 바꾸겠다"는 선언이 아니라 도장이 된다. */
const 태그마다_줄바꿈 = (html: string) => html.replace(/></g, ">\n<");

afterEach(cleanup);

describe("펭귄 그림", () => {
  /** 렌더된 펭귄의 마크업. `female`이 훌라 상의를 켠다. */
  function 그린다(female: boolean): string {
    const { container } = render(createElement(Penguin, female ? { female: true } : {}));
    return 태그마다_줄바꿈(container.innerHTML);
  }

  it("수컷_펭귄_그림이_그대로다", () => {
    expect(그린다(false)).toMatchSnapshot();
  });

  it("암컷_펭귄_그림이_그대로다", () => {
    expect(그린다(true)).toMatchSnapshot();
  });

  it("암수가_서로_다른_그림이다", () => {
    // 둘이 같은 스냅샷으로 굳으면 `female` 분기가 죽어도 통과한다.
    // 마크업 차이는 `.pg-female` 클래스 하나뿐이다.
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
    // 상수만 스냅샷하면 `root.innerHTML`이 그걸 안 써도 통과한다 — 그러면 공이
    // `ball.css`의 크기·커서·구르기 규칙을 통째로 잃는다.
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
    // 같은 방식으로 파싱해 대조한다 — DOM이 자기닫는 태그를 펼쳐서 문자열
    // 직접 비교는 항상 다르다.
    const 정답 = document.createElement("div");
    정답.innerHTML = BOWLING_BALL_SVG;
    expect(ball!.outerHTML, "그려진 것이 BOWLING_BALL_SVG가 아니다").toBe(
      정답.querySelector(".bw-ball")!.outerHTML,
    );
  });
});

describe("커서 방망이", () => {
  it("커서_방망이_그림이_그대로다", () => {
    // 커서는 jsdom이 렌더하지 않아 다른 그물이 없다.
    expect(batCursorUrl(55)).toMatchSnapshot();
    expect(batCursorUrl(-40)).toMatchSnapshot();
  });

  it("배율이_그림과_핫스팟에_함께_걸린다", () => {
    // 한쪽만 걸리면 방망이 끝이 포인터에서 어긋나 때리는 자리가 틀어진다.
    const 큰 = batCursorUrl(55, 1.5);
    expect(큰, "그림이 안 커졌다").toContain(`width='${BAT_BASE_PX * 1.5}'`);
    expect(큰, "핫스팟이 안 따라왔다").toMatch(/\)\s15 45$/);
    const 작은 = batCursorUrl(55, 0.5);
    expect(작은).toContain(`width='${BAT_BASE_PX * 0.5}'`);
    expect(작은).toMatch(/\)\s5 15$/);
  });

  it("viewBox는_배율과_무관하게_그대로다", () => {
    // `viewBox`까지 늘리면 그림이 안 커지고 여백만 붙는다.
    for (const scale of [0.5, 1, 1.5]) {
      expect(batCursorUrl(55, scale)).toContain(
        `viewBox='0 0 ${BAT_BASE_PX} ${BAT_BASE_PX}'`,
      );
    }
  });

  it("배율이_1이면_예전과_같은_문자열이다", () => {
    expect(batCursorUrl(55, 1)).toBe(batCursorUrl(55));
  });

  it("커서_이미지가_128px를_넘지_않는다", () => {
    // WebKit은 상한을 넘는 커서 그림을 **버리고** 대체 키워드로 떨어진다 —
    // 증상은 "커서가 갑자기 화살표로 돌아간다" 하나뿐이라 원인이 안 보인다.
    // 상한(SIZE_MAX)을 올리려면 이 검사가 먼저 빨개진다.
    const 최대 = Math.round(BAT_BASE_PX * (SIZE_MAX / 100));
    expect(최대, `가장 큰 커서가 ${최대}px이다`).toBeLessThanOrEqual(CURSOR_MAX_PX);
  });

  it("맨_키워드는_값의_끝에만_온다", () => {
    // `cursor`는 키워드를 목록 맨 끝에만 허용한다. 그림 뒤가 아니면 선언이
    // 통째로 버려진다 (CLAUDE.md 함정).
    const 값 = batCursorValue(55, "grab", 0.6);
    expect(값.endsWith(", grab")).toBe(true);
    expect(값.indexOf("grab")).toBe(값.length - "grab".length);
  });

  it("설치는_두_프레임을_모두_배율로_심는다", () => {
    const doc = document.implementation.createHTMLDocument("t");
    installBatCursor("grab", doc, 1.5);
    const 든 = doc.documentElement.style.getPropertyValue(BAT_CURSOR_VAR);
    const 휘두른 = doc.documentElement.style.getPropertyValue(BAT_SWING_VAR);
    for (const 값 of [든, 휘두른]) {
      expect(값, "프레임 하나가 배율을 안 탔다").toContain(`width='${BAT_BASE_PX * 1.5}'`);
      expect(값.endsWith(", grab")).toBe(true);
    }
    expect(든).not.toBe(휘두른);
  });
});

describe("소품은 React를 쓰지 않는다", () => {
  // 바닐라 창이 React를 끌어오면 그 번들에 React가 통째로 들어간다 (KTD7).
  // 위반의 실제 모양은 `.tsx`이고, 홑따옴표·`require`·간접 경로도 막아야 한다.
  const dir = resolve("src/assets/props");

  it("소품에는_React가_없다", () => {
    const files = readdirSync(dir);
    expect(files.length, "src/assets/props 가 비었다").toBeGreaterThan(0);
    // `.tsx`가 여기 생기는 것 자체가 위반이다.
    for (const f of files) {
      expect(f, `${f} — 소품에 JSX 파일이 있다. props는 바닐라다`).not.toMatch(/\.tsx$/);
    }
    for (const f of files.filter((n) => n.endsWith(".ts"))) {
      const 코드 = readFileSync(join(dir, f), "utf8")
        .replace(/\/\*[\s\S]*?\*\//g, " ")
        .replace(/\/\/.*/g, " ");
      expect(코드, `${f} 가 React를 끌어온다`).not.toMatch(/['"]react['"/]/);
      // `penguin/`은 TSX라 import하면 React가 딸려온다.
      expect(코드, `${f} 가 penguin/을 거쳐 React를 끌어온다`).not.toMatch(
        /from ["'][^"']*penguin/,
      );
    }
  });
});
