import React from "react";
import ReactDOM from "react-dom/client";
import RuntimeView from "./views/RuntimeView";
import "./styles/globals.css";

// Restore accessibility preferences
if (localStorage.getItem("cuyamaca-reduce-transparency") === "true") {
  document.documentElement.setAttribute("data-reduce-transparency", "true");
}
if (localStorage.getItem("cuyamaca-reduce-motion") === "true") {
  document.documentElement.setAttribute("data-reduce-motion", "true");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RuntimeView />
  </React.StrictMode>,
);
