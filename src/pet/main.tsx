import React from "react";
import ReactDOM from "react-dom/client";
import { PetApp } from "./PetApp";
import "./css/index.css";

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <PetApp />
  </React.StrictMode>,
);
