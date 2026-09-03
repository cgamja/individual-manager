/** 펭귄의 클릭 판정 상자 — **그림의 치수라서 여기 산다.** React를 안 쓴다.
 *
 * 치수는 SVG `viewBox` 단위로만 적고 무대 크기에서 픽셀을 낸다. 같은 수가
 * `src-tauri/src/pet_bridge/hit.rs`에도 있고 `src/pet/pet-css.test.ts`가 대조한다.
 *
 * 배경과 설계 근거: `MOTIONS.md` "클릭의 경계는 창이 아니라 히트 상자다".
 */

/** `Penguin`의 `viewBox`. `index.tsx`의 `viewBox="0 0 100 130"`과 같아야 한다. */
export const PENGUIN_VIEWBOX = { w: 100, h: 130 } as const;

/** 펭귄 실루엣을 감싸는 상자 — **viewBox 단위**. 실측 bbox(14.7..81.3,
 * 12.7..128.8)를 바깥으로 반올림하고 좌우를 중심(50)에 대칭으로 맞췄다
 * (`.pg-stage--flip`이 그림만 뒤집는다). 낚싯대·방망이는 뺐다 — `base.css`의
 * `pointer-events` 목록과 같아야 한다. */
export const PENGUIN_HIT_BOX = { l: 14, t: 12, r: 86, b: 130 } as const;

/** 되돌리기를 미리 거는 여유 — 무대 크기에 대한 비율. 커서가 그림에 닿기
 * **전에** 창이 클릭을 도로 먹기 시작한다. */
export const PENGUIN_HIT_ARM_RATIO = 0.1;

/** 히스테리시스 띠 — 웹뷰가 요청하는 경계와 Rust가 되돌리는 경계 사이의 간격.
 * 둘이 맞닿으면 그 위에서 요청과 되돌리기가 번갈아 일어난다. */
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

function inflate([l, t, r, b]: Rect, by: number): Rect {
  return [l - by, t - by, r + by, b + by];
}

/** Rust가 통과를 되돌리는 상자 — 히트 상자에 여유를 더한 것. */
export function revertBoxInWindow(m: StageMetrics): Rect {
  return inflate(hitBoxInWindow(m), m.size * PENGUIN_HIT_ARM_RATIO);
}

/** 통과를 요청해도 되는 바깥 상자 — 되돌리기 상자 밖이어야 한다. */
export function requestBoxInWindow(m: StageMetrics): Rect {
  return inflate(revertBoxInWindow(m), m.size * PENGUIN_HIT_HYSTERESIS_RATIO);
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
