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
 * 판과 펭귄이 **같은 방망이를 함께 보는지** 확인한다. 그림은
 * `assets/props/bat.ts` 하나에 있고 각 창이 `:root`에 심는다.
 */
/** 주석을 걷어낸다 — CSS도 예외가 아니다. 설명 주석에 검사가 걸린다.
 * `docs/solutions/best-practices/source-text-tests-pass-on-comments.md` */
const 코드만_css = (s: string) => s.replace(/\/\*[\s\S]*?\*\//g, " ");
const board = 코드만_css(readFileSync(resolve("src/pinball/pinball.css"), "utf8"));
const pet = 코드만_css(readFileSync(resolve("src/pet/css/pinball.css"), "utf8"));

/** 주석을 걷어낸다 — 호출을 주석 처리해도 이름이 남아 통과한다. */
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
      // 최상위 호출이어야 한다 — 함수 안에 감싸면 아무도 안 부른다.
      expect(code, `${name}가 커서를 최상위에서 심지 않는다`).toMatch(
        /^installBatCursor\(/m,
      );
    }
  });

  it("휘두르는_프레임이_따로_있다", () => {
    // 든 자세와 휘두른 자세는 회전각이 다르다. 같아지면 누를 때 아무 일도 안 난다.
    expect(batCursorUrl(55)).not.toBe(batCursorUrl(-40));

    // 어느 선택자가 어느 프레임을 받는지까지 본다 — 둘이 다른지만 보면
    // 두 프로퍼티를 맞바꿔도 통과한다.
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
    // `cursor` 문법은 키워드를 목록 맨 끝에만 허용한다. `var()` 뒤에 뭔가
    // 붙으면 프로퍼티가 없을 때 `grab, grab`이 되어 선언이 통째로 죽는다.
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
    // 이름이 CSS와 어긋나면 커서가 조용히 기본값으로 남는다.
    installBatCursor("grab", document);
    const style = document.documentElement.style;
    expect(style.getPropertyValue(BAT_CURSOR_VAR)).toBe(batCursorValue(55, "grab"));
    expect(style.getPropertyValue(BAT_SWING_VAR)).toBe(batCursorValue(-40, "grab"));
  });

  it("심는_값이_맨_키워드로_끝난다", () => {
    // 키워드 없는 `url(...) 10 30`도 무효다. 그 키워드는 CSS의 `var()`
    // 대체값과 같아야 심기 전후로 안 깜빡인다.
    for (const [이름, , 키워드] of entries) {
      const 값 = batCursorValue(55, 키워드);
      expect(값, `${이름}의 커서 값이 키워드로 안 끝난다`).toMatch(
        new RegExp(`,\\s*${키워드}$`),
      );
    }
    expect(board, "판의 대체값이 default가 아니다").toContain("var(--pg-bat-cursor, default)");
    expect(pet, "펭귄의 대체값이 grab이 아니다").toContain("var(--pg-bat-cursor, grab)");
  });

  it("판은_배경을_칠하지_않는다", () => {
    // 조금이라도 칠하면 화면 전체에 막이 씌워진 것처럼 보인다.
    expect(board).toMatch(/background:\s*transparent/);
    // **`transparent`·`none` 말고는 아무것도 허용하지 않는다.** 예전에는
    // `#`·`rgb`·`hsl`만 막아서 이름 색(`background: black`)과
    // `background-image: linear-gradient(...)`가 통과했다.
    for (const [, prop, 값] of board.matchAll(/(background(?:-color|-image)?):\s*([^;]+);/g)) {
      expect(
        값.trim(),
        `${prop}에 ${값.trim()} 를 칠했다 — 판은 투명해야 한다`,
      ).toMatch(/^(transparent|none)$/);
    }
  });
});
