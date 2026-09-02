import "./court.css";

/**
 * 코트 창의 웹뷰 — 모래사장과 네트.
 *
 * **입력도 상태도 없다.** 코트는 판이 도는 동안 안 변하므로 구독할 이벤트가
 * 없고(창의 존재 자체가 상태다), 사용자가 만질 것도 없다. 그래서 React를 쓰지
 * 않는다 — 판(`src/pinball`)·공(`src/ball`)이 그런 것과 같은 이유로 그릴 트리가 없다.
 *
 * 나가는 문(Esc)도 두지 않는다. 핀볼 판이 그게 필요했던 이유는 화면 전체의
 * 클릭을 먹어서인데, 코트는 클릭을 통과시키므로 사용자를 가두지 않는다.
 */

/** 모래 — 위 가장자리만 물결지고 아래는 화면 밖까지 내려간다. */
const SAND_TOP = "#f4dfae";
const SAND_BOTTOM = "#d9b878";
/** 네트 기둥 — 볕에 바랜 나무. */
const POST = "#a9743c";
/** 네트 그물. */
const MESH = "#f4f1e8";

/**
 * 코트 그림. **`viewBox`를 `preserveAspectRatio="none"`으로 늘린다** — 창 크기가
 * 화면 폭에 따라 달라지는데, 비율을 지키면 넓은 화면에서 모래가 세로로 늘어난다.
 * 네트는 늘어나면 안 되므로 가운데에 따로 겹쳐 그린다.
 */
const COURT_SVG = `
<svg class="vb-court" viewBox="0 0 1000 200" preserveAspectRatio="none"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <linearGradient id="vb-sand" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="${SAND_TOP}" />
      <stop offset="100%" stop-color="${SAND_BOTTOM}" />
    </linearGradient>
  </defs>

  <!-- 모래사장. 위 가장자리를 물결지게 해서 "쌓인 모래"로 보이게 한다 -->
  <path d="M0 128 Q 125 118 250 126 T 500 124 T 750 127 T 1000 122 L1000 200 L0 200 Z"
        fill="url(#vb-sand)" />
  <path d="M0 128 Q 125 118 250 126 T 500 124 T 750 127 T 1000 122"
        stroke="#fff3d4" stroke-width="2.5" fill="none" opacity="0.7" />
</svg>

<svg class="vb-court vb-court--net" viewBox="0 0 1000 200"
     preserveAspectRatio="xMidYMax meet"
     xmlns="http://www.w3.org/2000/svg" aria-hidden="true"
     style="position:absolute;inset:0;">
  <!-- 네트. 가운데 고정이고 늘어나지 않는다 -->
  <g class="vb-net-mesh">
    <rect x="452" y="18" width="96" height="86" fill="none"
          stroke="${MESH}" stroke-width="2" opacity="0.9" />
    <path d="M452 36 H548 M452 54 H548 M452 72 H548 M452 90 H548
             M470 18 V104 M488 18 V104 M506 18 V104 M524 18 V104"
          stroke="${MESH}" stroke-width="1.2" opacity="0.65" />
    <rect x="452" y="12" width="96" height="8" rx="3" fill="${MESH}" />
  </g>
  <rect x="448" y="10" width="7" height="118" rx="3" fill="${POST}" />
  <rect x="545" y="10" width="7" height="118" rx="3" fill="${POST}" />
</svg>`;

const root = document.getElementById("court-root");
if (root) {
  root.innerHTML = COURT_SVG;
  // 마우스 오른쪽 메뉴까지 막아 둔다 — 클릭은 통과하지만 컨텍스트 메뉴는
  // 웹뷰가 자기 것으로 잡는 경우가 있다.
  root.addEventListener("contextmenu", (e) => e.preventDefault());
}

export { COURT_SVG };
