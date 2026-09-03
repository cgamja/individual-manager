import React from "react";
import ReactDOM from "react-dom/client";
import { installBatCursor } from "../assets/props/bat";
import { PetApp } from "./PetApp";
import "./css/index.css";

// **커서 방망이를 심는다.** CSS는 TS를 못 부르므로 그림이 `:root`의 커스텀
// 프로퍼티로 들어가야 `css/pinball.css`의 `var()`가 받는다. 창마다 한 번씩
// 부른다 — 판 창은 `src/pinball/main.ts`가 따로 심는다 (KTD8).
installBatCursor();

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
