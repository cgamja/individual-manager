import { getVolleyState, onVolleyState, type VolleyBallSnapshot } from "../lib/pet";
import { BEACH_BALL_SVG } from "../assets/props/beach-ball";
import "./ball.css";

/**
 * 비치볼 창의 웹뷰.
 *
 * **볼링 공과 정반대다.** 그쪽은 사용자가 집어 굴리므로 포인터 셋과 드래그
 * 정산이 있지만, 여기서는 사용자가 아무것도 안 한다 — 위치는 Rust 틱이 창을
 * 옮겨서 정하고, 이 파일이 하는 일은 "날아가는 중인가"에 따라 클래스를 켜고
 * 끄는 것뿐이다.
 */

const root = document.getElementById("vball-root");

if (root) {
  root.innerHTML = BEACH_BALL_SVG;
  root.addEventListener("contextmenu", (e) => e.preventDefault());

  const paint = (ball: VolleyBallSnapshot) => {
    root.classList.toggle("vb-ball--flying", ball.flying);
  };

  // **받는 쪽을 창에 묶는다.** 전역 `listen()`은 대상을 `Any`로 등록해서
  // emit 대상과 무관하게 전부 호출된다 — 창이 여럿이면 그때 터진다
  // (`docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`).
  void onVolleyState(paint).catch(() => {});

  // **첫 상태는 구독으로 안 온다.** 틱이 이 창을 만들고 같은 호출에서 보내므로
  // 위 리스너가 붙기 전에 지나간다 — 받아 오지 않으면 공이 판 내내 안 돈다.
  void getVolleyState()
    .then((ball) => {
      if (ball) paint(ball);
    })
    .catch(() => {});
}
