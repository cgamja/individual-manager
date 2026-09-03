/** 비치발리볼 코트 — 모래사장과 네트. React를 쓰지 않는다 (KTD7).
 *
 * 치수는 Rust 상수(`VOLLEY_*`)와 `court.css`가 함께 정한다 —
 * `volley.test.ts`의 `코트 CSS가 Rust 상수와 같다`가 셋을 대조한다.
 */

import { NET_MESH, NET_POST, SAND_BOTTOM, SAND_FOAM, SAND_TOP } from "../palette";

/**
 * 모래사장. **가로로만 늘어난다**(`preserveAspectRatio="none"`) — 세로는 CSS가
 * 고정하므로 화면이 넓어져도 모래가 두꺼워지지 않는다.
 *
 * `viewBox` 세로 88 = 물결 여유 8(`--vb-sand-wave`) + 발밑 선 아래 두께
 * 80(`VOLLEY_SAND_DEPTH`).
 *
 * **물결의 한가운데(y=8)가 공이 떨어지는 모래 표면이자 펭귄의 발밑이다.**
 *
 * 아랫단은 가운데가 가장 두껍다(y=88 = 창 바닥). 가장자리는 그보다 얇아 단면이
 * 아래로 볼록한 모래톱으로 읽힌다 — `VOLLEY_SAND_DEPTH`는 그 **가장 두꺼운 데**를
 * 기준으로 창을 잡는다.
 * 물결을 전부 그 아래에 두면 공이 모래 위에 떠 보인다 — 실제로 1~18px 떠 있었다.
 *
 * **`T`(이어지는 매끄러운 곡선)를 안 쓴다.** 제어점이 반사돼 누적되면서 실제
 * 곡선이 앵커가 말하는 범위를 훌쩍 넘는다 — 3~12로 적어 두고 1~18로 그려졌다.
 * 구간마다 `Q`로 제어점을 직접 준다.
 */
export const SAND_SVG = `
<svg class="vb-sand" viewBox="0 0 1000 88" preserveAspectRatio="none"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <linearGradient id="vb-sand-fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="${SAND_TOP}" />
      <stop offset="100%" stop-color="${SAND_BOTTOM}" />
    </linearGradient>
  </defs>
  <path d="M0 9 Q 125 2 250 8 Q 375 14 500 7 Q 625 1 750 9 Q 875 15 1000 8
           L1000 80 Q 500 96 0 80 Z"
        fill="url(#vb-sand-fill)" />
  <path d="M0 9 Q 125 2 250 8 Q 375 14 500 7 Q 625 1 750 9 Q 875 15 1000 8"
        stroke="${SAND_FOAM}" stroke-width="3" fill="none" opacity="0.8" />
</svg>`;

/**
 * 네트. **모래에 발을 딛는다** — 그물 꼭대기가 펭귄 머리 바로 밑이고, 아래끝이
 * 모래 표면에 닿는다 (모래도 판과 함께 화면 세로 중앙에 뜬다).
 *
 * `viewBox` 96×85가 `VOLLEY_NET_HALF_W * 2` × `VOLLEY_NET_HEIGHT`와 같고,
 * CSS가 같은 값을 px로 고정한다 — 어긋나면 `volley.test.ts`가 잡는다.
 */
export const NET_SVG = `
<svg class="vb-net" viewBox="0 0 96 85"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="vb-net-mesh">
    <path d="M6 20 H90 M6 34 H90 M6 48 H90 M6 62 H90
             M20 8 V74 M34 8 V74 M48 8 V74 M62 8 V74 M76 8 V74"
          stroke="${NET_MESH}" stroke-width="1.6" opacity="0.7" />
    <rect x="4" y="6" width="88" height="70" fill="none"
          stroke="${NET_MESH}" stroke-width="2" opacity="0.9" />
    <rect x="4" y="1.5" width="88" height="8" rx="3.5" fill="${NET_MESH}" />
  </g>
  <rect x="0" y="0" width="7" height="84" rx="3.5" fill="${NET_POST}" />
  <rect x="89" y="0" width="7" height="84" rx="3.5" fill="${NET_POST}" />
</svg>`;
