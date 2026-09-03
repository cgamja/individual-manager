import { installBatCursor } from "../assets/props/bat";
import { setPetPinball } from "../lib/pet";
import { savePetSettings } from "../lib/settings";
import "./pinball.css";

/** 핀볼 판 — 화면 전체를 덮는 투명 창의 웹뷰. */

/** 핀볼을 끈다 — **거는 것과 저장을 둘 다** 한다. */
async function turnOff(): Promise<void> {
  await savePetSettings({ pinball: false }).catch(() => {});
  await setPetPinball(false).catch(() => {});
}

/** **Esc는 나가는 문 둘 중 하나다** (다른 하나는 트레이 아이콘). */
window.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  e.preventDefault();
  void turnOff();
});

// **커서 방망이를 심는다.** 펭귄 창과 같은 그림을 같은 이름으로 받지만,
// 창끼리 CSS를 공유하지 않으므로(KTD8) 각자 심는다. 인자 "default"는 CSS의
// `var()` 대체값과 같아야 한다.
//
// **Esc 등록보다 뒤다.** 판은 화면 전체의 클릭을 먹으므로 나가는 문이 둘 있어야
// 하는데(Esc와 트레이), 이게 앞에 있으면 여기서 던졌을 때 모듈이 멈춰 Esc가
// 영영 안 걸린다. 커서가 안 예뻐지는 것과 갇히는 것은 급이 다르다.
installBatCursor("default");
