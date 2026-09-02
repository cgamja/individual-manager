import { onVolleyState, type VolleyBallSnapshot } from "../lib/pet";
import "./ball.css";

/**
 * 비치볼 창의 웹뷰.
 *
 * **볼링 공과 정반대다.** 그쪽은 사용자가 집어 굴리므로 포인터 셋과 드래그
 * 정산이 있지만, 여기서는 사용자가 아무것도 안 한다 — 위치는 Rust 틱이 창을
 * 옮겨서 정하고, 이 파일이 하는 일은 "날아가는 중인가"에 따라 클래스를 켜고
 * 끄는 것뿐이다.
 */

/** 비치볼 — 흰 바탕에 색 조각 셋. 펭귄만큼 공들이지 않는다. */
const BALL_SVG = `
<svg class="vb-ball" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="vb-ball-body">
    <circle cx="32" cy="32" r="30" fill="#fdfcf7" stroke="#c9c2b0" stroke-width="1.6" />
    <path d="M32 2 A30 30 0 0 1 60 22 Q 36 26 32 2 Z" fill="#ff6f9c" />
    <path d="M60 22 A30 30 0 0 1 46 57 Q 34 36 60 22 Z" fill="#3fb8d8" />
    <path d="M46 57 A30 30 0 0 1 8 46 Q 28 34 46 57 Z" fill="#ffd35c" />
    <circle cx="32" cy="32" r="30" fill="none" stroke="#b9b1a0" stroke-width="1.6" />
    <ellipse cx="22" cy="18" rx="8" ry="5" fill="#ffffff" opacity="0.55"
             transform="rotate(-28 22 18)" />
  </g>
</svg>`;

const root = document.getElementById("vball-root");

if (root) {
  root.innerHTML = BALL_SVG;
  root.addEventListener("contextmenu", (e) => e.preventDefault());

  const paint = (ball: VolleyBallSnapshot) => {
    root.classList.toggle("vb-ball--flying", ball.flying);
  };

  // **받는 쪽을 창에 묶는다.** 전역 `listen()`은 대상을 `Any`로 등록해서
  // emit 대상과 무관하게 전부 호출된다 — 창이 여럿이면 그때 터진다
  // (`docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`).
  void onVolleyState(paint).catch(() => {});
}

export { BALL_SVG };
