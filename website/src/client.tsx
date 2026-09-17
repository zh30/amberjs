import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { AppRoutes } from "./app";
import "./global.css";

const el = document.getElementById("root");
if (!el) {
  throw new Error("missing #root");
}

const app = (
  <StrictMode>
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  </StrictMode>
);

if (el.hasChildNodes()) {
  hydrateRoot(el, app);
} else {
  createRoot(el).render(app);
}
