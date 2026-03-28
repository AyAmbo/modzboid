import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModManagerStore } from "../store";
import { useActiveProfile } from "../../profiles/store";
import type { ModConflict } from "../../../shared/types/conflicts";

export function useConflicts() {
  const setConflicts = useModManagerStore((s) => s.setConflicts);
  const activeProfile = useActiveProfile();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const detectConflicts = useCallback(async () => {
    if (!activeProfile || activeProfile.loadOrder.length === 0) {
      setConflicts([]);
      return;
    }
    try {
      const conflicts = await invoke<ModConflict[]>("detect_conflicts_cmd", {
        modIds: activeProfile.loadOrder,
      });
      setConflicts(conflicts);
    } catch (err) {
      console.error("Failed to detect conflicts:", err);
    }
  }, [activeProfile, setConflicts]);

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => { detectConflicts(); }, 300);
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, [detectConflicts]);

  return { detectConflicts };
}
