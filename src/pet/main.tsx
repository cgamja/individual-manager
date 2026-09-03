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

// **첫 페인트부터 배율을 안다** — Rust가 창을 만들며 `window.__PG_SCALE`을 심는다.
installBatCursor("grab", document, initialScale());
배율(initialScale());
void loadPetSettings()
  .then((s) => 배율(s.size / 100))
  .catch(() => {});
void onPetScale(({ size }) => 배율(size / 100)).catch(() => {});

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
