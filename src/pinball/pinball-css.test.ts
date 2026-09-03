import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { BAT_CURSOR_VAR, BAT_SWING_VAR, batCursorUrl, installBatCursor } from "../assets/props/bat";

/**
 * 판(`pinball.css`)과 펭귄(`pet/css/pinball.css`)의 방망이가 **같은 그림**인지 본다.
 *
 * **묻는 방식이 바뀌었다.** 예전에는 두 CSS에 복붙된 data-URI 집합을 대조했는데,
 * 그림이 `src/assets/props/bat.ts` 하나로 모이면서 "같은가"가 아니라
 * **"하나를 함께 보는가"**가 됐다. 복붙이 없으면 갈라질 곳도 없다.
 */
const board = readFileSync(resolve("src/pinball/pinball.css"), "utf8");
const pet = readFileSync(resolve("src/pet/css/pinball.css"), "utf8");
/** 각 창의 엔트리 — 심는 쪽이다. CSS만 고치고 심는 걸 빠뜨리면 커서가 안 바뀐다.
 *
 * **주석을 걷어낸다.** 안 그러면 호출을 주석 처리해도 이름이 남아 통과한다 —
 * 실제로 그랬다. */
const entries = [
  ["src/pinball/main.ts", readFileSync(resolve("src/pinball/main.ts"), "utf8")],
  ["src/pet/main.tsx", readFileSync(resolve("src/pet/main.tsx"), "utf8")],
].map(([name, code]) => [name, code.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ")]);

describe("핀볼 판", () => {
  it("판과_펭귄의_방망이가_같은_그림이다", () => {
    // 둘 다 같은 이름의 커스텀 프로퍼티를 읽고, 둘 다 그걸 심는다.
    for (const css of [board, pet]) {
      expect(css).toContain(`var(${BAT_CURSOR_VAR}`);
      expect(css).toContain(`var(${BAT_SWING_VAR}`);
    }
    for (const [name, code] of entries) {
      expect(code, `${name}가 커서를 심지 않는다 — CSS만 고치면 커서가 안 바뀐다`).toContain(
        "installBatCursor()",
      );
    }
  });

  it("휘두르는_프레임이_따로_있다", () => {
    // 든 자세와 휘두른 자세는 회전각이 다르다. 같아지면 누를 때 아무 일도 안 난다.
    expect(batCursorUrl(55)).not.toBe(batCursorUrl(-40));
    expect(board).toMatch(/#pinball-root:active/);
    expect(pet).toMatch(/\.pg-pinball:active/);
  });

  it("방망이_복붙이_CSS에_안_남아있다", () => {
    // 예전에는 같은 data-URI가 여섯 벌 있었다. 다시 박아 넣는 것을 막는다.
    for (const css of [board, pet]) {
      expect(css, "커서 그림이 다시 CSS에 박혔다").not.toContain("data:image/svg+xml");
    }
  });

  it("커서에_대체값이_있다", () => {
    // `var()`가 무효가 되면 `cursor` 선언이 **통째로** 죽는다 — 프로퍼티를 심기
    // 전 한 프레임과, 심는 데 실패한 경우가 그렇다.
    const 선언 = [...board.matchAll(/cursor:\s*([^;]+);/g), ...pet.matchAll(/cursor:\s*([^;]+);/g)];
    expect(선언.length, "cursor 선언을 못 찾았다").toBeGreaterThan(0);
    for (const [, 값] of 선언) {
      expect(값, `대체값 없는 var(): ${값}`).toMatch(/var\(--pg-bat-\w+,\s*\w+\)/);
    }
  });

  it("심으면_두_프레임이_문서에_올라간다", () => {
    // 프로퍼티 이름이 CSS와 어긋나면 커서가 조용히 기본값으로 남는다.
    installBatCursor(document);
    const style = document.documentElement.style;
    expect(style.getPropertyValue(BAT_CURSOR_VAR)).toBe(batCursorUrl(55));
    expect(style.getPropertyValue(BAT_SWING_VAR)).toBe(batCursorUrl(-40));
  });

  it("판은_배경을_칠하지_않는다", () => {
    expect(board).toMatch(/background:\s*transparent/);
    expect(board).not.toMatch(/background(-color)?:\s*(#|rgb|hsl)/);
  });
});
