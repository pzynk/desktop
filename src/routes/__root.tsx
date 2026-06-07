import { createRootRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Titlebar } from "../components/layout/titlebar";

export const Route = createRootRoute({
  component: RootComponent,
});

function RootComponent() {
  const navigate = useNavigate();

  useEffect(() => {
    const unlisten = listen("navigate-to-transfer-page", () => {
      navigate({ to: "/transfer" });
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [navigate]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh", width: "100vw", overflow: "hidden" }}>
      <Titlebar />
      <div style={{ flex: 1, overflow: "hidden", position: "relative" }}>
        <Outlet />
      </div>
    </div>
  );
}
