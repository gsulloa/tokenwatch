import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "@/app/App";
import { APP_DISPLAY_NAME } from "@/platform/app-identity";
import "@/styles/global.css";

document.title = APP_DISPLAY_NAME;

// Apply the color theme. All design tokens in global.css are scoped to
// :root[data-theme="light"|"dark"], so without this attribute every CSS
// variable (--canvas, --text, --surface, --border, …) is undefined and the
// UI renders with no backgrounds/borders (and invisible text in dark mode).
function applyTheme(dark: boolean): void {
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
applyTheme(prefersDark.matches);
prefersDark.addEventListener("change", (e) => {
  applyTheme(e.matches);
});

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("#root not found");

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
