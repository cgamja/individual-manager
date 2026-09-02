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
