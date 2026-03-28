import { useState, useMemo, useRef, useCallback } from "react";
import {
  DndContext,
  DragOverlay,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type Modifier,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import Fuse from "fuse.js";
import { Input } from "../../../shared/components/ui/input";
import { Button } from "../../../shared/components/ui/button";
import { ModCard } from "./ModCard";
import { ModContextMenu } from "./ModContextMenu";
import { DisableWarningDialog } from "./DisableWarningDialog";
import { useModManagerStore } from "../store";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import { useModDragDrop } from "../hooks/useModDragDrop";
import type { ModInfo } from "../../../shared/types/modTypes";

const MOD_HEIGHT = 56;

function SortableModCard({
  mod,
  onDisable,
}: {
  mod: ModInfo;
  onDisable: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: mod.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <ModContextMenu mod={mod} isActive={true} onToggle={onDisable}>
        <ModCard
          mod={mod}
          isActive={true}
          showDragHandle={false}
          onToggle={onDisable}
        />
      </ModContextMenu>
    </div>
  );
}

export function ActiveModList() {
  const allMods = useModManagerStore((s) => s.allMods);
  const enabledSearch = useModManagerStore((s) => s.enabledSearch);
  const setEnabledSearch = useModManagerStore((s) => s.setEnabledSearch);
  const selectedModIds = useModManagerStore((s) => s.selectedModIds);
  const clearSelection = useModManagerStore((s) => s.clearSelection);
  const activeProfile = useActiveProfile();
  const disableMod = useProfileStore((s) => s.disableMod);
  const { handleDragEnd } = useModDragDrop();

  const [activeId, setActiveId] = useState<string | null>(null);
  const [disableDialogOpen, setDisableDialogOpen] = useState(false);
  const [pendingDisableId, setPendingDisableId] = useState<string | null>(null);
  const [pendingDisableName, setPendingDisableName] = useState("");
  const [reverseDeps, setReverseDeps] = useState<string[]>([]);

  const parentRef = useRef<HTMLDivElement>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 5 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  // CSS zoom on Windows causes coordinate mismatch in @dnd-kit.
  // This modifier corrects the DragOverlay position by dividing by zoom factor.
  const zoomModifier: Modifier = useCallback(({ transform }) => {
    const zoom = parseFloat(getComputedStyle(document.documentElement).zoom) || 1;
    return {
      ...transform,
      x: transform.x / zoom,
      y: transform.y / zoom,
    };
  }, []);

  // Resolve mod objects in load order — create placeholders for missing mods
  const enabledMods = useMemo(() => {
    if (!activeProfile) return [];
    const modMap = new Map(allMods.map((m) => [m.id, m]));
    return activeProfile.loadOrder.map((id): ModInfo => {
      const found = modMap.get(id);
      if (found) return found;
      // Placeholder for mods in load order but not on disk
      return {
        id, rawId: id, workshopId: null, name: id, description: "Not installed — mod files not found on disk",
        authors: ["Unknown"], url: null, modVersion: null, posterPath: null, iconPath: null,
        versionMin: null, versionMax: null, versionFolders: [], activeVersionFolder: null,
        requires: [], pack: null, tileDef: [], category: null, source: "local",
        sourcePath: "", modInfoPath: "", sizeBytes: null, lastModified: "", detectedCategory: null,
      } as ModInfo;
    });
  }, [activeProfile, allMods]);

  const missingModIds = useMemo(() => {
    const modIdSet = new Set(allMods.map((m) => m.id));
    return new Set(enabledMods.filter((m) => !modIdSet.has(m.id)).map((m) => m.id));
  }, [enabledMods, allMods]);

  // Fuzzy search filtering
  const filteredMods = useMemo(() => {
    if (!enabledSearch.trim()) return enabledMods;
    const fuse = new Fuse(enabledMods, {
      keys: ["name", "authors", "description"],
      threshold: 0.3,
    });
    return fuse.search(enabledSearch).map((r) => r.item);
  }, [enabledMods, enabledSearch]);

  const rowVirtualizer = useVirtualizer({
    count: filteredMods.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => MOD_HEIGHT,
    overscan: 5,
  });

  const handleDisable = useCallback(
    async (modId: string, modName: string) => {
      try {
        const deps = await invoke<string[]>("reverse_deps_cmd", {
          modId,
          enabledModIds: activeProfile?.loadOrder ?? [],
        });
        if (deps.length === 0) {
          disableMod(modId);
        } else {
          setPendingDisableId(modId);
          setPendingDisableName(modName);
          setReverseDeps(deps);
          setDisableDialogOpen(true);
        }
      } catch {
        disableMod(modId);
      }
    },
    [disableMod, activeProfile]
  );

  const handleDisableAll = useCallback(() => {
    if (!pendingDisableId) return;
    for (const depId of reverseDeps) {
      disableMod(depId);
    }
    disableMod(pendingDisableId);
    setPendingDisableId(null);
  }, [pendingDisableId, reverseDeps, disableMod]);

  const handleDisableOnly = useCallback(() => {
    if (!pendingDisableId) return;
    disableMod(pendingDisableId);
    setPendingDisableId(null);
  }, [pendingDisableId, disableMod]);

  // Count how many selected mods are in this (active) list
  const selectedActiveCount = useMemo(() => {
    if (selectedModIds.size === 0) return 0;
    const enabledSet = new Set(enabledMods.map((m) => m.id));
    let count = 0;
    for (const id of selectedModIds) {
      if (enabledSet.has(id)) count++;
    }
    return count;
  }, [selectedModIds, enabledMods]);

  const handleBulkDisable = useCallback(() => {
    const enabledSet = new Set(enabledMods.map((m) => m.id));
    for (const id of selectedModIds) {
      if (enabledSet.has(id)) {
        disableMod(id);
      }
    }
    clearSelection();
  }, [selectedModIds, enabledMods, disableMod, clearSelection]);

  const handleDragStart = useCallback((event: { active: { id: string | number } }) => {
    setActiveId(String(event.active.id));
  }, []);

  const handleDragEndWrapped = useCallback((event: Parameters<typeof handleDragEnd>[0]) => {
    setActiveId(null);
    handleDragEnd(event);
  }, [handleDragEnd]);

  const activeMod = activeId ? filteredMods.find((m) => m.id === activeId) ?? null : null;

  const modIds = useMemo(
    () => filteredMods.map((m) => m.id),
    [filteredMods]
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-3 py-2 border-b border-border">
        <div className="flex items-center justify-between mb-1.5">
          <h3 className="text-sm font-semibold">
            Enabled ({enabledMods.length})
            {missingModIds.size > 0 && (
              <span className="ml-1.5 text-xs font-normal text-warning">
                ({missingModIds.size} not installed)
              </span>
            )}
          </h3>
          {selectedActiveCount > 1 ? (
            <Button variant="outline" size="sm" className="h-6 text-xs text-destructive" onClick={handleBulkDisable}>
              Disable {selectedActiveCount} selected
            </Button>
          ) : selectedActiveCount === 0 && enabledMods.length > 0 && (
            <span className="text-xs text-muted-foreground/50">Ctrl+click to multi-select</span>
          )}
        </div>
        <Input
          data-testid="mod-search"
          placeholder="Search enabled mods..."
          value={enabledSearch}
          onChange={(e) => setEnabledSearch(e.target.value)}
          className="h-7 text-xs"
        />
      </div>

      {/* Sortable list */}
      <div ref={parentRef} className="flex-1 overflow-auto">
        {filteredMods.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
            {enabledMods.length === 0
              ? "No mods enabled. Use the + button to enable mods."
              : "No matches found."}
          </div>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEndWrapped}
          >
            <SortableContext
              items={modIds}
              strategy={verticalListSortingStrategy}
            >
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  width: "100%",
                  position: "relative",
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualItem) => {
                  const mod = filteredMods[virtualItem.index];
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
                      className={missingModIds.has(mod.id) ? "opacity-60 bg-destructive/5" : ""}
                    >
                      <SortableModCard
                        mod={mod}
                        onDisable={() => handleDisable(mod.id, mod.name)}
                      />
                    </div>
                  );
                })}
              </div>
            </SortableContext>
            <DragOverlay modifiers={[zoomModifier]}>
              {activeMod && (
                <ModCard
                  mod={activeMod}
                  isActive={true}
                  showDragHandle={false}
                />
              )}
            </DragOverlay>
          </DndContext>
        )}
      </div>
      <DisableWarningDialog
        open={disableDialogOpen}
        onOpenChange={setDisableDialogOpen}
        modName={pendingDisableName}
        dependentIds={reverseDeps}
        onDisableAll={handleDisableAll}
        onDisableOnly={handleDisableOnly}
      />
    </div>
  );
}
