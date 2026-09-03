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

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
