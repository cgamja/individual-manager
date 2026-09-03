import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Penguin } from "./penguin";
import { BALL_SVG as VOLLEY_BALL_SVG } from "../volley/ball";
import { NET_SVG, SAND_SVG } from "../volley/court";

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

afterEach(() => {
  cleanup();
  clearMocks();
  document.body.innerHTML = "";
});

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
    expect(VOLLEY_BALL_SVG).toMatchSnapshot();
  });

  it("모래사장_그림이_그대로다", () => {
    expect(SAND_SVG).toMatchSnapshot();
  });

  it("네트_그림이_그대로다", () => {
    expect(NET_SVG).toMatchSnapshot();
  });
});

describe("볼링공 그림", () => {
  // **볼링공만 접근이 다르다.** `ball/main.ts`는 그림을 export하지 않고, import
  // 시점에 `#ball-root`를 잡아 리스너를 걸고 Tauri를 부른다. 부작용을 피하는 대신
  // **통과시켜 실제 렌더 결과를 본다** — 그림이 옮겨진 뒤에도 지켜야 하는 것이
  // 바로 그 결과다. 막는 방식은 `ball.test.ts`와 같다.
  beforeEach(async () => {
    vi.resetModules();
    document.body.innerHTML = '<div id="ball-root"></div>';
    mockWindows("bowling-ball");
    (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      return undefined;
    });
    await import("../ball/main");
  });

  it("볼링공_그림이_그대로다", () => {
    const ball = document.querySelector(".bw-ball");
    expect(ball, "#ball-root 안에 공이 안 그려졌다").not.toBeNull();
    expect(ball!.outerHTML).toMatchSnapshot();
  });
});
