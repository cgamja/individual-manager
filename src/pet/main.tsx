import React from "react";
import ReactDOM from "react-dom/client";
import { installBatCursor } from "../assets/props/bat";
import { followPetScale } from "../lib/settings";
import { PetApp } from "./PetApp";
import "./css/index.css";

/** 크기 배율을 건다. 그림은 `--pg-scale`(CSS 변환)이 줄이고, **커서 그림은
 * 변환을 안 따라오므로** 방망이만 따로 그린다. 부팅 경쟁과 화해는
 * `followPetScale`이 쥔다 — 판 창·코트 창이 같은 것을 쓴다. */
function 배율(s: number) {
  document.documentElement.style.setProperty("--pg-scale", String(s));
  installBatCursor("grab", document, s);
}

void followPetScale(배율).catch(() => {});

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
