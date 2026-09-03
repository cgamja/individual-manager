/** 볼링 공 그림. **React를 쓰지 않는다** — 공 창(`src/ball/main.ts`)은 바닐라라
 * 여기서 React를 끌어오면 그 번들에 React가 통째로 들어간다 (KTD7).
 */

import { BOWL_BALL, BOWL_HOLE, BOWL_RIM, BOWL_SHEEN } from "../palette";

/** 공 그림 — 원 하나에 손가락 구멍 셋. 펭귄만큼 공들이지 않는다 (A6). */
export const BOWLING_BALL_SVG = `
<svg class="bw-ball" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="bw-ball-body">
    <circle cx="32" cy="32" r="30" fill="${BOWL_BALL}" stroke="${BOWL_RIM}" stroke-width="2" />
    <path d="M12 18 A30 30 0 0 1 34 5" stroke="${BOWL_SHEEN}" stroke-width="5"
          stroke-linecap="round" fill="none" opacity="0.55" />
    <circle cx="25" cy="24" r="4.4" fill="${BOWL_HOLE}" />
    <circle cx="38" cy="21" r="4.4" fill="${BOWL_HOLE}" />
    <circle cx="32" cy="34" r="4.4" fill="${BOWL_HOLE}" />
  </g>
</svg>`;
