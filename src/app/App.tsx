import { useEffect, useCallback } from "react";
import { useLocation } from "wouter";
import { Layout } from "./Layout";
import { AppRouter } from "./router";
import { useAppStore } from "../shared/stores/appStore";
import { Toaster } from "../shared/components/ui/toaster";
import { CommandPalette } from "../shared/components/CommandPalette";
import { UpdateChecker } from "../shared/components/UpdateChecker";
import { useTauriEvent } from "../shared/hooks/useTauriEvent";

export default function App() {
  const { config, isLoading, loadConfig, setGameRunning } = useAppStore();
  const [, setLocation] = useLocation();

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (!isLoading && config) {
      // Only redirect to first-run if no paths are configured at all
      const hasNoPaths = !config.gamePath && !config.workshopPath && !config.steamPath;
      if (config.isFirstRun && hasNoPaths) {
        setLocation("/first-run");
      }
    }
  }, [isLoading, config, setLocation]);

  // Apply theme class to <html> element
  useEffect(() => {
    const theme = config?.theme ?? "dark";
    const root = document.documentElement;
    if (theme === "system") {
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      root.classList.toggle("dark", prefersDark);
    } else {
      root.classList.toggle("dark", theme === "dark");
    }
  }, [config?.theme]);

  // Apply UI scale and font size
  // Windows (WebView2): use CSS zoom (fixes ReactFlow, context menus)
  // Linux (WebKitGTK): use transform:scale (zoom not supported on WebKitGTK)
  useEffect(() => {
    const root = document.documentElement;
    const body = document.body;
    const scale = (config?.uiScale ?? 100) / 100;
    const size = config?.fontSize ?? 14;
    const isWindows = navigator.userAgent.includes("Windows");
    root.style.fontSize = `${size}px`;

    // Clean up both approaches
    body.style.transform = "";
    body.style.transformOrigin = "";
    body.style.width = "";
    body.style.height = "";
    body.style.overflow = "";
    root.style.zoom = "";

    if (scale !== 1) {
      if (isWindows) {
        root.style.zoom = `${scale}`;
      } else {
        body.style.transform = `scale(${scale})`;
        body.style.transformOrigin = "top left";
        body.style.width = `${100 / scale}%`;
        body.style.height = `${100 / scale}vh`;
        body.style.overflow = "auto";
      }
    }
  }, [config?.uiScale, config?.fontSize]);

  const handleGameExited = useCallback(
    (_payload: { exitCode: number | null }) => {
      setGameRunning(false);
    },
    [setGameRunning]
  );

  useTauriEvent("game-exited", handleGameExited);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-foreground" style={{ backgroundColor: '#0a0a0b', color: '#fafafa' }}>
        Loading...
      </div>
    );
  }

  return (
    <>
      <Layout>
        <AppRouter />
      </Layout>
      <Toaster />
      <CommandPalette />
      <UpdateChecker />
    </>
  );
}
