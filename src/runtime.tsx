import React from "react";
import ReactDOM from "react-dom/client";
import RuntimeView from "./views/RuntimeView";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RuntimeView />
  </React.StrictMode>,
);
