import { createRootRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

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

  return <Outlet />;
}
