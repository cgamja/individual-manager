import { installBatCursor } from "../assets/props/bat";
import { onPetScale, setPetPinball } from "../lib/pet";
import { initialScale, loadPetSettings, savePetSettings } from "../lib/settings";
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

// 커서 방망이를 심는다. 인자는 `pinball.css`의 `var()` 대체값과 같아야 한다.
// **Esc 등록보다 뒤다** — 앞이면 여기서 던졌을 때 나가는 문 하나가 사라진다.
//
// 판에는 그림이 없어 `--pg-scale`을 걸 것이 없다. 커서만 배율을 탄다.
installBatCursor("default", document, initialScale());
// 저장소 왕복이 방송보다 늦게 오면 옛 배율이 새 배율을 덮는다 (펭귄 창과 같은 장치).
let 세대 = 0;
const 부팅_세대 = 세대;
void loadPetSettings()
  .then((s) => {
    if (부팅_세대 === 세대) installBatCursor("default", document, s.size / 100);
  })
  .catch(() => {});
void onPetScale(({ size }) => {
  세대 += 1;
  installBatCursor("default", document, size / 100);
}).catch(() => {});
