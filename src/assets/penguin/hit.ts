/** 펭귄의 클릭 판정 상자 — **그림의 치수라서 여기 산다.**
 *
 * 창은 `PET_SIZE` 정사각 무대에 방망이·말풍선 여백을 더한 244×220이지만, 펭귄이
 * 실제로 그려진 자리는 그 17%뿐이다. 나머지도 창이 클릭을 먹는 것이 사용자가
 * 겪은 문제였다.
 *
 * 치수는 SVG `viewBox` 단위로만 적고 무대 크기에서 픽셀을 낸다 — 여백이 바뀌어도
 * 무관하고, 크기가 바뀌면 따라간다.
 *
 * **같은 수가 `src-tauri/src/pet_bridge/hit.rs`에도 있다** (Rust는 TS를 못 부른다).
 * `src/pet/pet-css.test.ts`의 "히트 박스 상수 동기화"가 둘을 대조한다.
 *
 * React를 쓰지 않는다 — 순수 산수뿐이다.
 */

/** `Penguin`의 `viewBox`. `index.tsx`의 `viewBox="0 0 100 130"`과 같아야 한다. */
export const PENGUIN_VIEWBOX = { w: 100, h: 130 } as const;

/** 펭귄 실루엣을 감싸는 상자 — **viewBox 단위**.
 *
 * 실측 bbox(14.7..81.3, 12.7..128.8)를 바깥으로 반올림했다. 낚싯대와 방망이는
 * 일부러 뺐다 — 둘 다 `base.css`에서 `pointer-events: none`이라 웹뷰에서도
 * 클릭을 안 받는다. */
export const PENGUIN_HIT_BOX = { l: 14, t: 12, r: 82, b: 130 } as const;

/** 히스테리시스 띠의 폭 — 무대 크기에 대한 비율.
 *
 * 웹뷰는 요청 상자 **밖**에서만 통과를 요청하고 Rust는 히트 상자 **안**에서
 * 되돌린다. 그 사이 띠에서는 어느 쪽도 안 일어나 경계에서 진동하지 않는다. */
export const PENGUIN_HIT_HYSTERESIS_RATIO = 0.02;

/** `[left, top, right, bottom]`, 창 안 client 좌표. */
export type Rect = readonly [number, number, number, number];

/** 창 안에서 무대가 놓인 자리. CSS 변수(`--pg-size`·`--pg-pad-x`·`--pg-pad-top`)에서
 * 읽어 넘기므로 이 파일에도 픽셀 상수가 없다. */
export interface StageMetrics {
  size: number;
  padX: number;
  padTop: number;
}

/** viewBox 한 단위가 몇 px인가. `preserveAspectRatio`의 기본값 `xMidYMid meet`이라
 * **짧은 쪽 배율**이 이긴다. */
function artScale(size: number): number {
  return Math.min(size / PENGUIN_VIEWBOX.w, size / PENGUIN_VIEWBOX.h);
}

/** 펭귄이 실제로 그려진 자리 (창 안 client 좌표). */
export function hitBoxInWindow({ size, padX, padTop }: StageMetrics): Rect {
  const s = artScale(size);
  const ox = padX + (size - PENGUIN_VIEWBOX.w * s) / 2;
  const oy = padTop + (size - PENGUIN_VIEWBOX.h * s) / 2;
  return [
    ox + PENGUIN_HIT_BOX.l * s,
    oy + PENGUIN_HIT_BOX.t * s,
    ox + PENGUIN_HIT_BOX.r * s,
    oy + PENGUIN_HIT_BOX.b * s,
  ];
}

/** 통과를 요청해도 되는 바깥 상자 — 히트 상자를 히스테리시스만큼 부풀린 것. */
export function requestBoxInWindow(m: StageMetrics): Rect {
  const by = m.size * PENGUIN_HIT_HYSTERESIS_RATIO;
  const [l, t, r, b] = hitBoxInWindow(m);
  return [l - by, t - by, r + by, b + by];
}

/** 반열린 구간 — 오른쪽·아래 변은 밖으로 친다 (Rust의 `contains`와 같은 규칙). */
function contains([l, t, r, b]: Rect, x: number, y: number): boolean {
  return x >= l && x < r && y >= t && y < b;
}

/** 이 자리에서 창이 클릭을 통과시켜도 되는가.
 *
 * **요청 상자를 쓴다** — 히트 상자를 쓰면 Rust가 되돌리는 조건과 정확히 맞닿아
 * 경계에서 요청과 되돌리기가 매 틱 번갈아 일어난다. */
export function shouldClickThrough(x: number, y: number, m: StageMetrics): boolean {
  return !contains(requestBoxInWindow(m), x, y);
}
