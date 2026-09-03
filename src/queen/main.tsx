import React from "react";
import ReactDOM from "react-dom/client";
import { followPetScale } from "../lib/settings";
import { QueenApp } from "./QueenApp";
import "../pet/css/index.css";
import "./queen.css";

/** 그림이 고정 px라 배율을 직접 걸어야 한다.
 *
 * **루트 id가 `pet-root`인 것이 중요하다** — `base.css`의 창 여백(`--pg-pad-*`)과
 * 배율 변환이 전부 `#pet-root`에 걸려 있어서, 다른 id를 쓰면 규칙을 하나도 못
 * 받아 **그림이 창에 잘린다.** 미녀 창은 펫 창과 같은 크기·같은 여백이므로
 * 같은 규칙을 그대로 타는 것이 맞다. 부팅 경쟁과 화해는
 * `followPetScale`이 쥔다 — 펭귄 창·판 창·코트 창이 같은 것을 쓴다.
 *
 * **커서 방망이는 안 건다** — 이 창은 클릭을 통과시키므로 커서가 여기 머무는
 * 일이 없다. */
void followPetScale((s) => {
  document.documentElement.style.setProperty("--pg-scale", String(s));
}).catch(() => {});

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <QueenApp />
  </React.StrictMode>,
);
