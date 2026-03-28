import { useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModManagerStore } from "../store";

interface WorkshopItemInfo {
  workshopId: string;
  size: number;
  timeUpdated: number;
}

export function useWorkshopUpdates() {
  const allMods = useModManagerStore((s) => s.allMods);
  const [updatedIds, setUpdatedIds] = useState<Set<string>>(new Set());

  const checkUpdates = useCallback(async () => {
    try {
      const items = await invoke<WorkshopItemInfo[]>("get_workshop_items_cmd");
      if (items.length === 0) return;

      // Build a map of workshop ID → update timestamp
      const wsMap = new Map(items.map((i) => [i.workshopId, i.timeUpdated]));

      // Find mods where Steam's timestamp is newer than our last_modified
      const updated = new Set<string>();
      for (const mod of allMods) {
        if (mod.workshopId && wsMap.has(mod.workshopId)) {
          const steamTime = wsMap.get(mod.workshopId)!;
          // Convert mod's lastModified ISO string to unix timestamp
          const modTime = Math.floor(new Date(mod.lastModified).getTime() / 1000);
          if (steamTime > modTime + 60) {
            // 60s tolerance for filesystem vs Steam timestamp differences
            updated.add(mod.id);
          }
        }
      }
      setUpdatedIds(updated);
    } catch (err) {
      console.error("Failed to check workshop updates:", err);
    }
  }, [allMods]);

  useEffect(() => {
    if (allMods.length > 0) {
      checkUpdates();
    }
  }, [allMods, checkUpdates]);

  return { updatedIds, checkUpdates };
}
