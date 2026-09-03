/** 커서 방망이 — 든 자세와 휘두른 자세 두 프레임.
 *
 * CSS는 TS를 못 부르므로 각 창의 엔트리가 `installBatCursor`로 `:root`에 심고,
 * CSS는 `cursor: var(--pg-bat-cursor, grab)`으로 받는다.
 *
 * **맨 키워드까지 프로퍼티에 담는다.** `cursor` 문법은 키워드를 목록 맨
 * 끝에만 허용해서, CSS 쪽에 `, grab`을 덧붙이면 프로퍼티가 없을 때
 * `cursor: grab, grab`이 되어 선언이 통째로 죽는다.
 *
 * React를 쓰지 않는다 (KTD7) — 판 창이 바닐라다.
 */

import { BAT_EDGE, BAT_GRIP, BAT_WOOD } from "../palette";

/** 든 자세의 회전각. */
const HELD_DEG = 55;
/** 휘두른 자세 — 커서에는 애니메이션이 안 걸려 프레임 두 장으로 흉내 낸다. */
const SWING_DEG = -40;
/** 배율 1일 때의 그림 한 변 (px). `viewBox`도 이 값이라 배율은 크기만 늘린다. */
export const BAT_BASE_PX = 48;
/** 손잡이 끝 — 포인터가 놓이는 자리 (배율 1 기준). */
const HOTSPOT_X = 10;
const HOTSPOT_Y = 30;

/** WebKit이 받아 주는 커서 그림의 한 변 상한 (px). **넘으면 그림을 버리고
 * 대체 키워드로 떨어진다** — 증상은 "커서가 갑자기 화살표로 돌아간다" 하나뿐이다. */
export const CURSOR_MAX_PX = 128;

/** 배율에 맞춘 그림 한 변. 소수 px는 커서에 의미가 없어 반올림한다. */
const sizeFor = (scale: number) => Math.round(BAT_BASE_PX * scale);

/** `#`·`<`·`>`만 인코딩한다 — 복붙 시절 CSS와 같은 형태다. */
function encode(svg: string): string {
  return svg.replace(/</g, "%3C").replace(/>/g, "%3E").replace(/#/g, "%23");
}

/** 각도 하나짜리 `url(…) x y`. 맨 키워드는 안 붙는다.
 *
 * **그림과 핫스팟이 함께 배율을 탄다.** 한쪽만 타면 방망이 끝이 포인터에서
 * 어긋나 때리는 자리가 틀어진다. `viewBox`는 그대로라 그림이 늘어난다. */
export function batCursorUrl(deg: number, scale = 1): string {
  const px = sizeFor(scale);
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='${px}' height='${px}' viewBox='0 0 ${BAT_BASE_PX} ${BAT_BASE_PX}'>` +
    `<g transform='rotate(${deg} ${HOTSPOT_X} ${HOTSPOT_Y})'>` +
    `<path d='M10 30 L17 23' stroke='${BAT_GRIP}' stroke-width='4' stroke-linecap='round'/>` +
    `<path d='M15 25 C20 20 25 14 29 9 C31.5 6.5 34 9 31.5 11.5 C27 17 21 22 17 25 Z' ` +
    `fill='${BAT_WOOD}' stroke='${BAT_EDGE}' stroke-width='2' stroke-linejoin='round'/>` +
    `</g></svg>`;
  const hx = Math.round(HOTSPOT_X * scale);
  const hy = Math.round(HOTSPOT_Y * scale);
  return `url("data:image/svg+xml,${encode(svg)}") ${hx} ${hy}`;
}

/** CSS가 읽는 프로퍼티 이름. 두 창이 같은 이름을 쓴다. */
export const BAT_CURSOR_VAR = "--pg-bat-cursor";
export const BAT_SWING_VAR = "--pg-bat-swing";

/** 프로퍼티에 들어갈 값 — 그림 + 맨 키워드. */
export function batCursorValue(deg: number, fallback: string, scale = 1): string {
  return `${batCursorUrl(deg, scale)}, ${fallback}`;
}

/** 두 프레임을 문서에 심는다. 창마다 한 번씩 부르고, 배율이 바뀌면 다시 부른다.
 *
 * `fallback`은 CSS의 `var()` 대체값과 같아야 한다 — 다르면 심기 전후로 깜빡인다.
 *
 * **`scale`을 넘길지는 창마다 다르다** — 웹뷰 줌이 걸린 창은 커서까지 함께
 * 늘어나므로 1로 두어야 두 번 곱해지지 않는다. 각 엔트리의 주석을 본다. */
export function installBatCursor(
  fallback: string,
  doc: Document = document,
  scale = 1,
): void {
  const root = doc.documentElement;
  root.style.setProperty(BAT_CURSOR_VAR, batCursorValue(HELD_DEG, fallback, scale));
  root.style.setProperty(BAT_SWING_VAR, batCursorValue(SWING_DEG, fallback, scale));
}
