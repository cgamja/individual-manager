import { cleanup, render } from "@testing-library/react";
import { createElement } from "react";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { Penguin } from "./penguin";
import { BEACH_BALL_SVG } from "./props/beach-ball";
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
});

describe("소품은 React를 쓰지 않는다", () => {
  it("소품에는_React가_없다", () => {
    // **바닐라 창(핀볼·볼링공·코트·비치볼)이 실수로 React를 끌어오면 그 번들에
    // React가 통째로 들어간다** (KTD7·KTD8). 아무도 안 알려 주므로 여기서 막는다.
    const dir = resolve("src/assets/props");
    const files = readdirSync(dir).filter((f) => f.endsWith(".ts"));
    expect(files.length, "src/assets/props 가 비었다").toBeGreaterThan(0);
    for (const f of files) {
      const 코드 = readFileSync(join(dir, f), "utf8").replace(/\/\*[\s\S]*?\*\//g, " ");
      expect(코드, `${f} 가 React를 끌어온다`).not.toMatch(/from "react/);
    }
  });
});
