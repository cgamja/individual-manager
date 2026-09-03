import React from "react";
import ReactDOM from "react-dom/client";
import { installBatCursor } from "../assets/props/bat";
import { onPetScale } from "../lib/pet";
import { initialScale, loadPetSettings } from "../lib/settings";
import { PetApp } from "./PetApp";
import "./css/index.css";

/** 크기 배율을 건다. 그림은 `--pg-scale`(CSS 변환)이 줄이고, **커서 그림은
 * 변환을 안 따라오므로** 방망이만 따로 그린다. 판 창(`pinball/main.ts`)이
 * 같은 두 줄을 쓴다. */
function 배율(s: number) {
  document.documentElement.style.setProperty("--pg-scale", String(s));
  installBatCursor("grab", document, s);
}

/** 지금까지 본 배율 소식의 세대. **저장소 왕복이 방송보다 늦게 도착하면 옛 배율이
 * 새 배율을 덮는다** — 부팅 직후 사용자가 슬라이더를 밀면 걸린다. 방송이 세대를
 * 올리므로, 자기 세대가 밀린 로드는 아무것도 안 한다. */
let 세대 = 0;

// **첫 페인트부터 배율을 안다** — Rust가 창을 만들며 `window.__PG_SCALE`을 심는다.
installBatCursor("grab", document, initialScale());
배율(initialScale());
const 부팅_세대 = 세대;
void loadPetSettings()
  .then((s) => {
    if (부팅_세대 === 세대) 배율(s.size / 100);
  })
  .catch(() => {});
void onPetScale(({ size }) => {
  세대 += 1;
  배율(size / 100);
}).catch(() => {});

/** **화해자.** 방송은 fire-and-forget이라 유실될 수 있다 — 그러면 Rust는 창을
 * 다시 쟀는데 웹뷰는 옛 배율로 그려 재시작할 때까지 잘린 채 남는다. Rust 쪽이
 * `last_size`로 창 크기를 매 틱 화해시키는 것과 같은 것을 여기에도 둔다.
 *
 * **`resize`가 정확히 그 신호다** — 창 크기를 바꾸는 것은 배율뿐이다. 값은 창
 * 크기에서 역산하지 않고 **저장소에서 다시 읽는다**: 창 크기는 정수로 반올림돼
 * 배율이 미세하게 어긋나고, 그 값이 Rust의 히트 상자와 갈린다. */
window.addEventListener("resize", () => {
  const 내_세대 = ++세대;
  void loadPetSettings()
    .then((s) => {
      if (내_세대 === 세대) 배율(s.size / 100);
    })
    .catch(() => {});
});

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
