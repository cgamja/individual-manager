import React from "react";
import ReactDOM from "react-dom/client";
import { Penguin } from "./Penguin";
import "./pet.css";

ReactDOM.createRoot(document.getElementById("pet-root") as HTMLElement).render(
  <React.StrictMode>
    <Penguin />
  </React.StrictMode>,
);
