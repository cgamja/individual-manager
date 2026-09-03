/** 비치볼 그림. React를 쓰지 않는다 (KTD7). */

import {
  BEACH_BLUE,
  BEACH_PINK,
  BEACH_RIM,
  BEACH_SEAM,
  BEACH_SHEEN,
  BEACH_WHITE,
  BEACH_YELLOW,
} from "../palette";

/** 비치볼 — 흰 바탕에 색 조각 셋. 펭귄만큼 공들이지 않는다. */
export const BEACH_BALL_SVG = `
<svg class="vb-ball" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="vb-ball-body">
    <circle cx="32" cy="32" r="30" fill="${BEACH_WHITE}" stroke="${BEACH_SEAM}" stroke-width="1.6" />
    <path d="M32 2 A30 30 0 0 1 60 22 Q 36 26 32 2 Z" fill="${BEACH_PINK}" />
    <path d="M60 22 A30 30 0 0 1 46 57 Q 34 36 60 22 Z" fill="${BEACH_BLUE}" />
    <path d="M46 57 A30 30 0 0 1 8 46 Q 28 34 46 57 Z" fill="${BEACH_YELLOW}" />
    <circle cx="32" cy="32" r="30" fill="none" stroke="${BEACH_RIM}" stroke-width="1.6" />
    <ellipse cx="22" cy="18" rx="8" ry="5" fill="${BEACH_SHEEN}" opacity="0.55"
             transform="rotate(-28 22 18)" />
  </g>
</svg>`;
