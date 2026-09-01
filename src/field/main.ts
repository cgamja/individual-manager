import { setPetPinball } from "../lib/pet";
import { savePetSettings } from "../lib/settings";
import "./field.css";

/**
 * 핀볼 판 — 화면 전체를 덮는 투명 창의 웹뷰.
 *
 * **React를 쓰지 않는다.** 하는 일이 커서 CSS 한 줄과 Esc 핸들러 하나라 상태도
 * 그릴 것도 없다.
 *
 * **클릭을 판정하지 않는다.** 판은 펭귄 창보다 **아래**에 깔리므로 펭귄 위의
 * 클릭·드래그는 지금까지처럼 펭귄 창이 받는다. 판이 위에 있으면 화면 좌표로
 * 히트 판정과 드래그 라우팅을 한 벌 더 써야 하는데, 그 코드(4px 문턱·포인터
 * 캡처·속도 샘플링)는 이미 한 번 어렵게 맞춘 것이라 두 벌이 되면 갈라진다.
 */

/** 핀볼을 끈다 — **거는 것과 저장을 둘 다** 한다.
 *
 * 저장을 빠뜨리면 다음 실행에 다시 켜진 채로 뜬다. 설정 창의 토글이 둘 다 하는
 * 것과 같은 규칙이다. */
async function turnOff(): Promise<void> {
  await setPetPinball(false).catch(() => {});
  await savePetSettings({ pinball: false }).catch(() => {});
}

/**
 * **Esc는 나가는 문 둘 중 하나다** (다른 하나는 트레이 아이콘).
 *
 * 화면 전체의 클릭을 먹는 기능이라 되돌리는 방법이 하나뿐이면 안 된다. 이 창은
 * 포커스를 뺏지 않으므로 **포커스가 없으면 키를 못 듣는다** — 그때는 메뉴바가
 * 이 창보다 높은 레벨이라 트레이 아이콘이 남아 있다.
 *
 * 다른 키는 건드리지 않는다. 판이 키보드를 삼키면 그것대로 "마우스만이 아니라
 * 키보드도 죽었다"가 된다.
 */
window.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  e.preventDefault();
  void turnOff();
});
