import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";
import { enable, isEnabled } from '@tauri-apps/plugin-autostart';
import "./index.css"

export const router = createRouter({ routeTree } as any);

// Automatically enable autostart on app launch if not already enabled
async function setupAutostart() {
  try {
    const autostartEnabled = await isEnabled();
    if (!autostartEnabled) {
      await enable();
    }
  } catch (error) {
    console.error("Failed to enable autostart:", error);
  }
}
setupAutostart();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
