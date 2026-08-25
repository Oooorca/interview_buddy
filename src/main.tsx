import React from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./app/App";
import RegionSelector from "./region/RegionSelector";
import "./i18n";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isTauri() && getCurrentWindow().label === "region-selector" ? <RegionSelector /> : <App />}
  </React.StrictMode>,
);
