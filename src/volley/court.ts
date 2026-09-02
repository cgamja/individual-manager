import "./court.css";

/**
 * 코트 창의 웹뷰 — 화면 세로 중앙의 **네트**와 화면 바닥의 **모래사장**.
 *
 * **입력도 상태도 없다.** 코트는 판이 도는 동안 안 변하므로 구독할 이벤트가
 * 없고(창의 존재 자체가 상태다), 사용자가 만질 것도 없다. 그래서 React를 쓰지
 * 않는다 — 판(`src/pinball`)·공(`src/ball`)이 그런 것과 같은 이유로 그릴 트리가 없다.
 *
 * 나가는 문(Esc)도 두지 않는다. 핀볼 판이 그게 필요했던 이유는 화면 전체의
 * 클릭을 먹어서인데, 코트는 클릭을 통과시키므로 사용자를 가두지 않는다.
 *
 * **좌표를 하나도 안 받는다.** 창이 `net_cx`를 중심으로 대칭이라(`Court::rect`)
 * 네트는 `left: 50%`로 정확히 맞고, 모래는 창 바닥에 붙으면 그만이다.
 */

/** 모래 — 볕에 마른 윗면에서 축축한 아랫면으로. */
const SAND_TOP = "#f4dfae";
const SAND_BOTTOM = "#d9b878";
/** 네트 기둥 — 볕에 바랜 나무. */
const POST = "#a9743c";
/** 그물. */
const MESH = "#f4f1e8";

/**
 * 모래사장. **가로로만 늘어난다**(`preserveAspectRatio="none"`) — 세로는 CSS가
 * 고정하므로 화면이 넓어져도 모래가 두꺼워지지 않는다.
 *
 * `viewBox` 세로 124 = 물결이 솟는 44 + 모래 깊이 80. 위 가장자리만 물결지고
 * 아래는 화면 밖까지 내려간다.
 */
const SAND_SVG = `
<svg class="vb-sand" viewBox="0 0 1000 124" preserveAspectRatio="none"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <linearGradient id="vb-sand-fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="${SAND_TOP}" />
      <stop offset="100%" stop-color="${SAND_BOTTOM}" />
    </linearGradient>
  </defs>
  <path d="M0 46 Q 125 32 250 44 T 500 40 T 750 45 T 1000 36 L1000 124 L0 124 Z"
        fill="url(#vb-sand-fill)" />
  <path d="M0 46 Q 125 32 250 44 T 500 40 T 750 45 T 1000 36"
        stroke="#fff3d4" stroke-width="3" fill="none" opacity="0.75" />
</svg>`;

/**
 * 네트. **판에 매달려 있다** — 모래에 서 있는 것이 아니라 펭귄 머리 바로 밑에
 * 걸린 그물이다 (판이 화면 세로 중앙이라 모래는 저 아래에 있다).
 *
 * `viewBox` 96×120이 `VOLLEY_NET_HALF_W * 2` × `VOLLEY_NET_HEIGHT`와 같고,
 * CSS가 같은 값을 px로 고정한다.
 */
const NET_SVG = `
<svg class="vb-net" viewBox="0 0 96 120"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="vb-net-mesh">
    <path d="M6 18 H90 M6 36 H90 M6 54 H90 M6 72 H90 M6 90 H90
             M20 8 V104 M34 8 V104 M48 8 V104 M62 8 V104 M76 8 V104"
          stroke="${MESH}" stroke-width="1.6" opacity="0.7" />
    <rect x="4" y="6" width="88" height="98" fill="none"
          stroke="${MESH}" stroke-width="2" opacity="0.9" />
    <rect x="4" y="2" width="88" height="9" rx="3.5" fill="${MESH}" />
  </g>
  <rect x="0" y="0" width="7" height="118" rx="3.5" fill="${POST}" />
  <rect x="89" y="0" width="7" height="118" rx="3.5" fill="${POST}" />
</svg>`;

const root = document.getElementById("court-root");
if (root) {
  root.innerHTML = SAND_SVG + NET_SVG;
  // 마우스 오른쪽 메뉴까지 막아 둔다 — 클릭은 통과하지만 컨텍스트 메뉴는
  // 웹뷰가 자기 것으로 잡는 경우가 있다.
  root.addEventListener("contextmenu", (e) => e.preventDefault());
}

export { SAND_SVG, NET_SVG };
