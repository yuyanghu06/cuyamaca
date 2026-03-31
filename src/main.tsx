import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Restore accessibility preferences before first render
if (localStorage.getItem("cuyamaca-reduce-transparency") === "true") {
  document.documentElement.setAttribute("data-reduce-transparency", "true");
}
if (localStorage.getItem("cuyamaca-reduce-motion") === "true") {
  document.documentElement.setAttribute("data-reduce-motion", "true");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
