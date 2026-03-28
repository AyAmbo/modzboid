import { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModManagerStore } from "../store";
import { useAppStore } from "../../../shared/stores/appStore";
import { useTauriEvent } from "../../../shared/hooks/useTauriEvent";
import { toast } from "../../../shared/components/ui/toaster";
import type { ModInfo } from "../../../shared/types/modTypes";

export function useMods() {
  const setMods = useModManagerStore((s) => s.setMods);
  const allMods = useModManagerStore((s) => s.allMods);
  const setIsLoading = useModManagerStore((s) => s.setIsLoading);
  const workshopPath = useAppStore((s) => s.config?.workshopPath);
  const localModsPath = useAppStore((s) => s.config?.localModsPath);

  const fetchMods = useCallback(async () => {
    try {
      setIsLoading(true);
      const mods = await invoke<ModInfo[]>("discover_mods");
      setMods(mods);
    } catch (err) {
      console.error("Failed to discover mods:", err);
      toast({ title: "Error", description: "Failed to discover mods", variant: "destructive" });
    } finally {
      setIsLoading(false);
    }
  }, [setMods, setIsLoading]);

  // Only run discovery when we have at least one configured path and mods haven't been loaded yet.
  useEffect(() => {
    if (allMods.length === 0 && (workshopPath || localModsPath)) {
      fetchMods();
    }
  }, [fetchMods, workshopPath, localModsPath, allMods.length]);

  const handleModsChanged = useCallback(() => {
    fetchMods();
  }, [fetchMods]);

  useTauriEvent("mods-changed", handleModsChanged);

  const refreshMods = useCallback(async () => {
    try {
      setIsLoading(true);
      const mods = await invoke<ModInfo[]>("refresh_mods");
      setMods(mods);
      toast({ title: "Refreshed", description: `Found ${mods.length} mods` });
    } catch (err) {
      console.error("Failed to refresh mods:", err);
      toast({ title: "Error", description: "Failed to refresh mods", variant: "destructive" });
    } finally {
      setIsLoading(false);
    }
  }, [setMods, setIsLoading]);

  return { allMods, refreshMods };
}
