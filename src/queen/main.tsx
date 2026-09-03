import React from "react";
import ReactDOM from "react-dom/client";
import { followPetScale } from "../lib/settings";
import { QueenApp } from "./QueenApp";
import "../pet/css/index.css";
import "./queen.css";

/** 그림이 고정 px라 배율을 직접 걸어야 한다. 부팅 경쟁과 화해는
 * `followPetScale`이 쥔다 — 펭귄 창·판 창·코트 창이 같은 것을 쓴다.
 *
 * **커서 방망이는 안 건다** — 이 창은 클릭을 통과시키므로 커서가 여기 머무는
 * 일이 없다. */
void followPetScale((s) => {
  document.documentElement.style.setProperty("--pg-scale", String(s));
}).catch(() => {});

ReactDOM.createRoot(document.getElementById("queen-root") as HTMLElement).render(
  <React.StrictMode>
    <QueenApp />
  </React.StrictMode>,
);
