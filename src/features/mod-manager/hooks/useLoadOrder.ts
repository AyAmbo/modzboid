import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModManagerStore } from "../store";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import { toast } from "../../../shared/components/ui/toaster";
import type { LoadOrderIssue } from "../../../shared/types/validation";

export function useLoadOrder() {
  const setIssues = useModManagerStore((s) => s.setIssues);
  const issues = useModManagerStore((s) => s.issues);
  const allMods = useModManagerStore((s) => s.allMods);
  const activeProfile = useActiveProfile();
  const reorderMods = useProfileStore((s) => s.reorderMods);

  const validate = useCallback(async () => {
    if (!activeProfile) return;
    try {
      const result = await invoke<LoadOrderIssue[]>(
        "validate_load_order_cmd",
        {
          loadOrder: activeProfile.loadOrder,
        }
      );
      setIssues(result);
    } catch (err) {
      console.error("Failed to validate load order:", err);
      toast({ title: "Error", description: "Failed to validate load order", variant: "destructive" });
    }
  }, [activeProfile, setIssues]);

  const autoSort = useCallback(async () => {
    if (!activeProfile) return;
    try {
      const sorted = await invoke<string[]>("sort_load_order_cmd", {
        modIds: activeProfile.loadOrder,
      });
      reorderMods(sorted);
    } catch (err) {
      console.error("Failed to auto-sort:", err);
      toast({ title: "Error", description: "Failed to auto-sort load order", variant: "destructive" });
    }
  }, [activeProfile, reorderMods]);

  const sortByName = useCallback(() => {
    if (!activeProfile) return;
    const modMap = new Map(allMods.map((m) => [m.id, m]));
    const sorted = [...activeProfile.loadOrder].sort((a, b) => {
      const modA = modMap.get(a);
      const modB = modMap.get(b);
      return (modA?.name ?? a).localeCompare(modB?.name ?? b);
    });
    reorderMods(sorted);
  }, [activeProfile, allMods, reorderMods]);

  const sortByAuthor = useCallback(() => {
    if (!activeProfile) return;
    const modMap = new Map(allMods.map((m) => [m.id, m]));
    const sorted = [...activeProfile.loadOrder].sort((a, b) => {
      const modA = modMap.get(a);
      const modB = modMap.get(b);
      const authorA = modA?.authors[0] ?? "";
      const authorB = modB?.authors[0] ?? "";
      return authorA.localeCompare(authorB);
    });
    reorderMods(sorted);
  }, [activeProfile, allMods, reorderMods]);

  return { validate, autoSort, sortByName, sortByAuthor, issues };
}
