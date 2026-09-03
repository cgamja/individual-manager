import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  BAT_CURSOR_VAR,
  BAT_SWING_VAR,
  batCursorUrl,
  batCursorValue,
  installBatCursor,
} from "../assets/props/bat";

/**
 * 판(`pinball.css`)과 펭귄(`pet/css/pinball.css`)의 방망이가 **같은 그림**인지 본다.
 *
 * **묻는 방식이 바뀌었다.** 예전에는 두 CSS에 복붙된 data-URI 집합을 대조했는데,
 * 그림이 `src/assets/props/bat.ts` 하나로 모이면서 "같은가"가 아니라
 * **"하나를 함께 보는가"**가 됐다. 복붙이 없으면 갈라질 곳도 없다.
 */
/** **주석을 걷어낸다.** CSS도 예외가 아니다 — 이 파일의 검사를 고치는 동안
 * `cursor: default, default`라고 적어 둔 **설명 주석**에 검사가 걸렸다.
 * 고치고 있는 버그에 고치는 사람이 걸린 셈이다.
 * `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만_css = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ");
const board = 코드만_css(readFileSync(resolve("src/pinball/pinball.css"), "utf8"));
const pet = 코드만_css(readFileSync(resolve("src/pet/css/pinball.css"), "utf8"));

/** **주석을 걷어낸다.** 안 그러면 호출을 주석 처리해도 이름이 남아 통과한다 —
 * 실제로 그랬다. `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만 = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*/g, " ");

/** 각 창의 엔트리 — 심는 쪽이다. CSS만 고치고 심는 걸 빠뜨리면 커서가 안 바뀐다. */
const entries: Array<[string, string, string]> = [
  ["src/pinball/main.ts", 코드만(readFileSync(resolve("src/pinball/main.ts"), "utf8")), "default"],
  ["src/pet/main.tsx", 코드만(readFileSync(resolve("src/pet/main.tsx"), "utf8")), "grab"],
];

describe("핀볼 판", () => {
  it("판과_펭귄의_방망이가_같은_그림이다", () => {
    // 둘 다 같은 이름의 커스텀 프로퍼티를 읽고, 둘 다 그걸 심는다.
    for (const css of [board, pet]) {
      expect(css).toContain(`var(${BAT_CURSOR_VAR}`);
      expect(css).toContain(`var(${BAT_SWING_VAR}`);
    }
    for (const [name, code] of entries) {
      // **최상위 호출이어야 한다.** 함수 안에 감싸 두면 이름은 남지만 아무도
      // 부르지 않아 커서가 영영 안 심긴다 — 들여쓰기 없는 줄로 못 박는다.
      expect(code, `${name}가 커서를 최상위에서 심지 않는다`).toMatch(
        /^installBatCursor\(/m,
      );
    }
  });

  it("휘두르는_프레임이_따로_있다", () => {
    // 든 자세와 휘두른 자세는 회전각이 다르다. 같아지면 누를 때 아무 일도 안 난다.
    expect(batCursorUrl(55)).not.toBe(batCursorUrl(-40));

    // **어느 선택자가 어느 프레임을 받는지까지 본다.** 둘이 다르다는 것만 보면
    // 두 프로퍼티를 통째로 맞바꿔도(가만히 있을 때 휘두른 방망이가 뜬다) 통과한다.
    for (const [이름, css] of [
      ["판", board],
      ["펭귄", pet],
    ] as const) {
      const 규칙 = [...css.matchAll(/([^{}]+)\{([^}]*cursor:[^}]*)\}/g)].map(
        ([, sel, body]) => [sel.trim(), body] as const,
      );
      const 누른것 = 규칙.filter(([sel]) => sel.includes(":active"));
      const 가만히 = 규칙.filter(([sel]) => !sel.includes(":active"));
      expect(누른것.length, `${이름}에 :active 커서 규칙이 없다`).toBeGreaterThan(0);
      expect(가만히.length, `${이름}에 평소 커서 규칙이 없다`).toBeGreaterThan(0);
      for (const [sel, body] of 누른것) {
        expect(body, `${sel} 가 휘두른 프레임을 안 받는다`).toContain(BAT_SWING_VAR);
      }
      for (const [sel, body] of 가만히) {
        expect(body, `${sel} 가 든 프레임을 안 받는다`).toContain(BAT_CURSOR_VAR);
      }
    }
  });

  it("방망이_복붙이_CSS에_안_남아있다", () => {
    // 예전에는 같은 data-URI가 여섯 벌 있었다. 다시 박아 넣는 것을 막는다.
    for (const css of [board, pet]) {
      expect(css, "커서 그림이 다시 CSS에 박혔다").not.toContain("data:image/svg+xml");
    }
  });

  it("커서에_대체값이_있다", () => {
    // **여기가 한 번 틀렸던 자리다.** `cursor` 문법은 키워드를 목록 **맨 끝에만**
    // 허용한다(`[<url> [<x> <y>]?,]* <keyword>`). 그래서
    // `cursor: var(--pg-bat-cursor, grab), grab`으로 쓰면 프로퍼티가 안 심겼을 때
    // `cursor: grab, grab`이 되는데 **무효라 선언이 통째로 버려진다** — 대체값이
    // 아무것도 막지 못하는데 이 검사는 정규식만 보고 초록을 줬다.
    //
    // 지킬 것 둘: (1) `var()` 뒤에 아무것도 안 붙는다, (2) 대체 경로가 맨 키워드
    // 하나다. 그래야 안 심겼을 때의 값이 그 자체로 유효한 선언이다.
    for (const [이름, css] of [
      ["판", board],
      ["펭귄", pet],
    ] as const) {
      const 선언 = [...css.matchAll(/cursor:\s*([^;]+);/g)].map((m) => m[1].trim());
      expect(선언.length, `${이름}에 cursor 선언이 없다`).toBeGreaterThan(0);
      for (const 값 of 선언) {
        expect(값, `${이름}: var() 뒤에 뭔가 더 붙었다 — 무효한 선언이 된다: ${값}`).toMatch(
          /^var\(--pg-bat-\w+,\s*[a-z-]+\)$/,
        );
      }
    }
  });

  it("심으면_두_프레임이_문서에_올라간다", () => {
    // 프로퍼티 이름이 CSS와 어긋나면 커서가 조용히 기본값으로 남는다.
    installBatCursor("grab", document);
    const style = document.documentElement.style;
    expect(style.getPropertyValue(BAT_CURSOR_VAR)).toBe(batCursorValue(55, "grab"));
    expect(style.getPropertyValue(BAT_SWING_VAR)).toBe(batCursorValue(-40, "grab"));
  });

  it("심는_값이_맨_키워드로_끝난다", () => {
    // **키워드가 없으면 `cursor: url(...) 10 30` 이 되는데 이것도 무효다** —
    // 문법이 맨 키워드를 요구한다. 프로퍼티 안에 키워드가 들어 있어야 하고,
    // 그 키워드는 CSS의 `var()` 대체값과 같아야 심기 전후로 안 깜빡인다.
    for (const [이름, , 키워드] of entries) {
      const 값 = batCursorValue(55, 키워드);
      expect(값, `${이름}의 커서 값이 키워드로 안 끝난다`).toMatch(
        new RegExp(`,\\s*${키워드}$`),
      );
    }
    // 심는 쪽과 CSS 대체값이 짝이 맞는지 — 어긋나면 심기 전후로 커서가 바뀐다.
    expect(board, "판의 대체값이 default가 아니다").toContain("var(--pg-bat-cursor, default)");
    expect(pet, "펭귄의 대체값이 grab이 아니다").toContain("var(--pg-bat-cursor, grab)");
  });

  it("판은_배경을_칠하지_않는다", () => {
    expect(board).toMatch(/background:\s*transparent/);
    expect(board).not.toMatch(/background(-color)?:\s*(#|rgb|hsl)/);
  });
});
