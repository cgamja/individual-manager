import { NET_SVG, SAND_SVG } from "../assets/props/court";
import { followPetScale } from "../lib/settings";
import "./court.css";

/**
 * 코트 창의 웹뷰 — 화면 세로 중앙에 뜬 **모래톱**과 그 위에 선 **네트**.
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
 *
 * **모래는 화면 바닥이 아니라 판에 있다.** 펭귄이 딛고 서는 면이라 판을 따라
 * 뜬다 — 한 번 화면 바닥의 배경 해변으로 그렸다가 되돌렸다.
 */

/** 네트·모래는 고정 px라 배율을 직접 걸어야 한다. 부팅 경쟁과 화해는
 * `followPetScale`이 쥔다 — 펭귄 창·판 창이 같은 것을 쓴다. */
void followPetScale((s) => {
  document.documentElement.style.setProperty("--pg-scale", String(s));
}).catch(() => {});

const root = document.getElementById("court-root");
if (root) {
  root.innerHTML = SAND_SVG + NET_SVG;
  // 마우스 오른쪽 메뉴까지 막아 둔다 — 클릭은 통과하지만 컨텍스트 메뉴는
  // 웹뷰가 자기 것으로 잡는 경우가 있다.
  root.addEventListener("contextmenu", (e) => e.preventDefault());
}
