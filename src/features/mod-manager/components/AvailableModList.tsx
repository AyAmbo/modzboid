import { useState, useMemo, useRef, useCallback } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import Fuse from "fuse.js";
import { ModCard } from "./ModCard";
import { ModContextMenu } from "./ModContextMenu";
import { ModFilters } from "./ModFilters";
import { DependencyDialog } from "./DependencyDialog";
import { Button } from "../../../shared/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "../../../shared/components/ui/dropdown-menu";
import { useModManagerStore, type SortField } from "../store";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import type { ModInfo } from "../../../shared/types/modTypes";
import type { DepResolution } from "../../../shared/types/deps";
import { sortMods } from "../utils/sortMods";

const MOD_HEIGHT = 56;

export function AvailableModList() {
  const allMods = useModManagerStore((s) => s.allMods);
  const availableSearch = useModManagerStore((s) => s.availableSearch);
  const categoryFilter = useModManagerStore((s) => s.categoryFilter);
  const selectedModIds = useModManagerStore((s) => s.selectedModIds);
  const clearSelection = useModManagerStore((s) => s.clearSelection);
  const sortField = useModManagerStore((s) => s.sortField);
  const sortDirection = useModManagerStore((s) => s.sortDirection);
  const toggleSort = useModManagerStore((s) => s.toggleSort);
  const activeProfile = useActiveProfile();
  const enableMod = useProfileStore((s) => s.enableMod);

  const [depDialogOpen, setDepDialogOpen] = useState(false);
  const [pendingMod, setPendingMod] = useState<ModInfo | null>(null);
  const [depResolution, setDepResolution] = useState<DepResolution>({ toEnable: [], notInstalled: [] });

  const parentRef = useRef<HTMLDivElement>(null);

  // Filter out mods already in the active profile's load order
  const availableMods = useMemo(() => {
    if (!activeProfile) return allMods;
    const enabledSet = new Set(activeProfile.loadOrder);
    return allMods.filter((m) => !enabledSet.has(m.id));
  }, [allMods, activeProfile]);

  // Apply category filter
  const categoryFilteredMods = useMemo(() => {
    if (!categoryFilter) return availableMods;
    return availableMods.filter((m) => {
      const cat = m.detectedCategory ?? m.category;
      return cat === categoryFilter;
    });
  }, [availableMods, categoryFilter]);

  // Apply fuzzy search
  const searchedMods = useMemo(() => {
    if (!availableSearch.trim()) return categoryFilteredMods;
    const fuse = new Fuse(categoryFilteredMods, {
      keys: ["name", "authors", "description"],
      threshold: 0.3,
    });
    return fuse.search(availableSearch).map((r) => r.item);
  }, [categoryFilteredMods, availableSearch]);

  // Apply sorting
  const filteredMods = useMemo(() => {
    return sortMods(searchedMods, sortField, sortDirection);
  }, [searchedMods, sortField, sortDirection]);

  // Group mods by workshopId for multi-package workshop items
  const workshopGroups = useMemo(() => {
    const groups = new Map<string, ModInfo[]>();
    for (const mod of filteredMods) {
      const key = mod.workshopId ?? mod.id;
      const group = groups.get(key) || [];
      group.push(mod);
      groups.set(key, group);
    }
    return groups;
  }, [filteredMods]);

  // Build a flat list of rows with group headers for multi-mod workshop groups
  type RowItem = { type: "header"; workshopId: string; count: number } | { type: "mod"; mod: ModInfo; grouped: boolean };
  const rows = useMemo(() => {
    const result: RowItem[] = [];
    for (const [key, mods] of workshopGroups) {
      if (mods.length > 1) {
        result.push({ type: "header", workshopId: key, count: mods.length });
        for (const mod of mods) {
          result.push({ type: "mod", mod, grouped: true });
        }
      } else {
        result.push({ type: "mod", mod: mods[0], grouped: false });
      }
    }
    return result;
  }, [workshopGroups]);

  const HEADER_HEIGHT = 28;
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => rows[index].type === "header" ? HEADER_HEIGHT : MOD_HEIGHT,
    overscan: 5,
  });

  const handleEnable = useCallback(
    async (mod: ModInfo) => {
      try {
        const resolution = await invoke<DepResolution>("auto_resolve_deps_cmd", {
          modId: mod.id,
          enabledModIds: activeProfile?.loadOrder ?? [],
        });
        if (resolution.toEnable.length === 0 && resolution.notInstalled.length === 0) {
          enableMod(mod.id);
        } else {
          setPendingMod(mod);
          setDepResolution(resolution);
          setDepDialogOpen(true);
        }
      } catch {
        enableMod(mod.id);
      }
    },
    [enableMod, activeProfile]
  );

  const handleConfirmDeps = useCallback(() => {
    if (!pendingMod) return;
    for (const depId of depResolution.toEnable) {
      enableMod(depId);
    }
    enableMod(pendingMod.id);
    setPendingMod(null);
  }, [pendingMod, depResolution, enableMod]);

  // Count how many selected mods are in this (available) list
  const selectedAvailableCount = useMemo(() => {
    if (selectedModIds.size === 0) return 0;
    const availableSet = new Set(availableMods.map((m) => m.id));
    let count = 0;
    for (const id of selectedModIds) {
      if (availableSet.has(id)) count++;
    }
    return count;
  }, [selectedModIds, availableMods]);

  const handleBulkEnable = useCallback(() => {
    const availableSet = new Set(availableMods.map((m) => m.id));
    for (const id of selectedModIds) {
      if (availableSet.has(id)) {
        enableMod(id);
      }
    }
    clearSelection();
  }, [selectedModIds, availableMods, enableMod, clearSelection]);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-3 py-2 border-b border-border">
        <div className="flex items-center justify-between mb-1.5">
          <h3 className="text-sm font-semibold">
            Available ({availableMods.length})
          </h3>
          <div className="flex items-center gap-1.5">
            {selectedAvailableCount > 1 ? (
              <Button variant="outline" size="sm" className="h-6 text-xs text-success" onClick={handleBulkEnable}>
                Enable {selectedAvailableCount} selected
              </Button>
            ) : selectedAvailableCount === 0 && availableMods.length > 0 && (
              <span className="text-xs text-muted-foreground/50">Ctrl+click to multi-select</span>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="ghost" size="sm" className="h-6 text-xs" title="Sort mods">
                  {sortDirection === "asc" ? "\u2191" : "\u2193"} {sortField}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {(["name","author","id","source","category","size","lastModified","workshopId"] as SortField[]).map((f) => (
                  <DropdownMenuItem key={f} onClick={() => toggleSort(f)}>
                    {f === sortField ? (sortDirection === "asc" ? "\u2191 " : "\u2193 ") : "  "}
                    {f === "lastModified" ? "Last Modified" : f === "workshopId" ? "Workshop ID" : f.charAt(0).toUpperCase() + f.slice(1)}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
        <ModFilters />
      </div>

      {/* Virtualized list */}
      <div ref={parentRef} className="flex-1 overflow-auto">
        {filteredMods.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
            {availableMods.length === 0
              ? "All mods are enabled!"
              : "No matches found."}
          </div>
        ) : (
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualItem) => {
              const row = rows[virtualItem.index];
              if (row.type === "header") {
                return (
                  <div
                    key={`header-${row.workshopId}`}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      height: `${virtualItem.size}px`,
                      transform: `translateY(${virtualItem.start}px)`,
                    }}
                    className="flex items-center px-3 gap-1.5 bg-muted/30 border-b border-border"
                  >
                    <span className="text-xs font-medium text-muted-foreground">
                      Workshop {row.workshopId}
                    </span>
                    <span className="text-xs text-muted-foreground/60">
                      ({row.count} mods)
                    </span>
                  </div>
                );
              }
              const mod = row.mod;
              return (
                <div
                  key={mod.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                  className={row.grouped ? "border-l-2 border-l-muted-foreground/20" : ""}
                >
                  <ModContextMenu mod={mod} isActive={false} onToggle={() => handleEnable(mod)}>
                    <ModCard
                      mod={mod}
                      isActive={false}
                      onToggle={() => handleEnable(mod)}
                    />
                  </ModContextMenu>
                </div>
              );
            })}
          </div>
        )}
      </div>
      <DependencyDialog
        open={depDialogOpen}
        onOpenChange={setDepDialogOpen}
        modName={pendingMod?.name ?? ""}
        resolution={depResolution}
        onConfirm={handleConfirmDeps}
      />
    </div>
  );
}
