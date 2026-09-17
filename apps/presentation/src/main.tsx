import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { PresentationApp } from "./PresentationApp.tsx";
import "./styles.css";

const container = document.getElementById("root");
if (!container) throw new Error("#root not found");

createRoot(container).render(
  <StrictMode>
    <PresentationApp />
  </StrictMode>,
);
