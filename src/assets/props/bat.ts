/** 커서 방망이 — **이 앱에서 가장 많이 복붙됐던 그림.**
 *
 * 옮기기 전에는 같은 data-URI가 `src/pet/css/pinball.css`에 4번,
 * `src/pinball/pinball.css`에 2번, 총 여섯 벌 박혀 있었다. 위험이 실재했다는
 * 증거는 코드 안에 있다 — `pinball-css.test.ts`의
 * `판과_펭귄의_방망이가_같은_그림이다`는 두 CSS의 그림이 갈라지는 것을 막으려고
 * 붙여 둔 테이프였다. 여기서는 원천이 하나라 갈라질 곳이 없다.
 *
 * **CSS는 TS를 못 부른다.** 그래서 각 창의 엔트리가 시작할 때 `:root`에 커스텀
 * 프로퍼티로 심고, CSS는 `cursor: var(--pg-bat-cursor, grab)`로 받는다.
 * - 인라인 `style`은 **`:active`를 못 써서** 애초에 성립하지 않는다 — 휘두르는
 *   프레임이 바로 `:active`다.
 * - 빌드 타임 생성은 원본과 갈라져도 아무 말 안 하는 **새 조용한 실패 지점**이다.
 *
 * **깜빡임은 없다.** 프로퍼티가 심기기 전에는 `var()`의 대체값이 쓰이는데,
 * 그건 복붙 시절 CSS가 쓰던 대체값과 같다.
 *
 * React를 쓰지 않는다 (KTD7) — 핀볼 판 창이 바닐라다.
 */

import { CURSOR_BAT_EDGE, CURSOR_BAT_GRIP, CURSOR_BAT_WOOD } from "../palette";

/** 든 자세의 회전각. */
const HELD_DEG = 55;
/** 휘두른 자세 — **아래에서 위로 휘두른다.** 커서 이미지에는 CSS 애니메이션이
 * 안 걸리므로 프레임 두 장으로 흉내 낸다. */
const SWING_DEG = -40;
/** 커서의 손잡이 끝 — 그림 안에서 실제 포인터가 놓이는 자리다. */
const HOTSPOT = "10 30";

/** `#` 과 `<`·`>` 만 인코딩한다 — **복붙 시절 CSS에 있던 형태를 그대로 재현한다.**
 * 브라우저는 관대하지만, 다르게 인코딩하면 "같은 그림인가"를 눈으로 대조할 수
 * 없게 된다. */
function encode(svg: string): string {
  return svg.replace(/</g, "%3C").replace(/>/g, "%3E").replace(/#/g, "%23");
}

/** 각도 하나짜리 방망이 그림의 data-URI. */
export function batCursorUrl(deg: number): string {
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' viewBox='0 0 48 48'>` +
    `<g transform='rotate(${deg} ${HOTSPOT})'>` +
    `<path d='M10 30 L17 23' stroke='${CURSOR_BAT_GRIP}' stroke-width='4' stroke-linecap='round'/>` +
    `<path d='M15 25 C20 20 25 14 29 9 C31.5 6.5 34 9 31.5 11.5 C27 17 21 22 17 25 Z' ` +
    `fill='${CURSOR_BAT_WOOD}' stroke='${CURSOR_BAT_EDGE}' stroke-width='2' stroke-linejoin='round'/>` +
    `</g></svg>`;
  return `url("data:image/svg+xml,${encode(svg)}") ${HOTSPOT}`;
}

/** CSS가 읽는 프로퍼티 이름. 두 창이 **같은 이름**을 쓴다. */
export const BAT_CURSOR_VAR = "--pg-bat-cursor";
export const BAT_SWING_VAR = "--pg-bat-swing";

/**
 * 두 프레임을 문서에 심는다. **창마다 한 번씩 부른다** — 펭귄 창은
 * `src/pet/main.tsx`, 핀볼 판은 `src/pinball/main.ts`. 창끼리 CSS를 공유하지
 * 않는 것이 KTD8이라 각자 심는 것이 맞다.
 */
export function installBatCursor(doc: Document = document): void {
  const root = doc.documentElement;
  root.style.setProperty(BAT_CURSOR_VAR, batCursorUrl(HELD_DEG));
  root.style.setProperty(BAT_SWING_VAR, batCursorUrl(SWING_DEG));
}
