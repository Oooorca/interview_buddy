import React from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import RegionSelector from "./RegionSelector";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isTauri() && getCurrentWindow().label === "region-selector" ? <RegionSelector /> : <App />}
  </React.StrictMode>,
);
