import { useEffect, useCallback, useMemo, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LoadOrderToolbar } from "../components/LoadOrderToolbar";
import { ActiveModList } from "../components/ActiveModList";
import { AvailableModList } from "../components/AvailableModList";
import { ModTable } from "../components/ModTable";
import { ModDetailPanel } from "../components/ModDetailPanel";
import { MissingConfigBanner } from "../../profiles/components/MissingConfigBanner";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import { useModManagerStore } from "../store";
import { useMods } from "../hooks/useMods";
import { useConflicts } from "../hooks/useConflicts";
import type { ModInfo } from "../../../shared/types/modTypes";
import type { DepResolution } from "../../../shared/types/deps";
import { sortMods } from "../utils/sortMods";

function VerticalDivider({ onSplitChange }: { onSplitChange: (pct: number) => void }) {
  const isDragging = useRef(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const handleDragStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;

    const container = (e.currentTarget as HTMLElement).parentElement;
    if (!container) return;
    containerRef.current = container as HTMLDivElement;

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const pct = ((ev.clientX - rect.left) / rect.width) * 100;
      onSplitChange(Math.min(80, Math.max(20, pct)));
    };

    const handleMouseUp = () => {
      isDragging.current = false;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [onSplitChange]);

  return (
    <div
      className="w-1.5 shrink-0 cursor-col-resize group flex items-center justify-center hover:bg-primary/10 transition-colors"
      onMouseDown={handleDragStart}
    >
      <div className="w-0.5 h-8 rounded-full bg-muted-foreground/30 group-hover:bg-primary/60 transition-colors" />
    </div>
  );
}

export default function ModManagerPage() {
  const loadProfiles = useProfileStore((s) => s.loadProfiles);
  const enableMod = useProfileStore((s) => s.enableMod);
  const disableMod = useProfileStore((s) => s.disableMod);
  const isLoading = useModManagerStore((s) => s.isLoading);
  const allMods = useModManagerStore((s) => s.allMods);
  const selectedModId = useModManagerStore((s) => s.selectedModId);
  const selectMod = useModManagerStore((s) => s.selectMod);
  const selectAllInList = useModManagerStore((s) => s.selectAllInList);
  const viewMode = useModManagerStore((s) => s.viewMode);
  const sortField = useModManagerStore((s) => s.sortField);
  const sortDirection = useModManagerStore((s) => s.sortDirection);
  const activeProfile = useActiveProfile();

  const [splitPercent, setSplitPercent] = useState(() => {
    const saved = localStorage.getItem("modzboid-split-pct");
    return saved ? Number(saved) : 50;
  });

  const handleSplitChange = useCallback((pct: number) => {
    setSplitPercent(pct);
    localStorage.setItem("modzboid-split-pct", String(Math.round(pct)));
  }, []);

  // Load profiles and mods on mount
  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  // useMods fetches mods on mount and sets up event listener
  useMods();
  useConflicts();

  const enabledSet = useMemo(() => {
    return new Set(activeProfile?.loadOrder ?? []);
  }, [activeProfile]);

  const enabledModIds = useMemo(() => {
    return activeProfile?.loadOrder ?? [];
  }, [activeProfile]);

  const availableModIds = useMemo(() => {
    return allMods.filter((m) => !enabledSet.has(m.id)).map((m) => m.id);
  }, [allMods, enabledSet]);

  // Resolved mod lists for table view
  const enabledMods = useMemo(() => {
    const modMap = new Map(allMods.map((m) => [m.id, m]));
    return enabledModIds
      .map((id) => modMap.get(id))
      .filter((m): m is ModInfo => m !== undefined);
  }, [allMods, enabledModIds]);

  const availableMods = useMemo(() => {
    const sorted = allMods.filter((m) => !enabledSet.has(m.id));
    return sortMods(sorted, sortField, sortDirection);
  }, [allMods, enabledSet, sortField, sortDirection]);

  // Keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Don't intercept when typing in an input/textarea
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
        if (e.key === "Escape") {
          (e.target as HTMLElement).blur();
          e.preventDefault();
        }
        return;
      }

      const isSelected = selectedModId !== null;
      const isInEnabled = isSelected && enabledSet.has(selectedModId);
      const isInAvailable = isSelected && !enabledSet.has(selectedModId) && allMods.some((m) => m.id === selectedModId);

      switch (e.key) {
        case "Delete":
        case "Backspace": {
          if (isInEnabled) {
            e.preventDefault();
            disableMod(selectedModId);
          }
          break;
        }
        case "Enter": {
          if (isInAvailable) {
            e.preventDefault();
            enableMod(selectedModId);
          }
          break;
        }
        case "a": {
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (isInEnabled) {
              selectAllInList(enabledModIds);
            } else {
              selectAllInList(availableModIds);
            }
          }
          break;
        }
        case "f": {
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            const testId = isInEnabled ? "mod-search" : "mod-search-available";
            const input = document.querySelector<HTMLInputElement>(`[data-testid="${testId}"]`);
            input?.focus();
          }
          break;
        }
        case "ArrowUp":
        case "ArrowDown": {
          e.preventDefault();
          const currentList = isInEnabled ? enabledModIds : availableModIds;
          if (currentList.length === 0) break;

          if (!isSelected) {
            selectMod(currentList[0]);
            break;
          }

          const idx = currentList.indexOf(selectedModId);
          if (idx === -1) {
            selectMod(currentList[0]);
            break;
          }

          const next = e.key === "ArrowUp"
            ? Math.max(0, idx - 1)
            : Math.min(currentList.length - 1, idx + 1);
          selectMod(currentList[next]);
          break;
        }
      }
    },
    [selectedModId, enabledSet, allMods, enabledModIds, availableModIds, disableMod, enableMod, selectMod, selectAllInList]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Check if server profile's .ini file exists
  const [configMissing, setConfigMissing] = useState(false);
  useEffect(() => {
    if (activeProfile?.type === "server" && activeProfile.serverConfigPath) {
      invoke<boolean>("validate_server_config_cmd", { filePath: activeProfile.serverConfigPath })
        .then((exists) => setConfigMissing(!exists))
        .catch(() => setConfigMissing(true));
    } else {
      setConfigMissing(false);
    }
  }, [activeProfile?.serverConfigPath, activeProfile?.type]);

  const showSpinner = isLoading && allMods.length === 0;

  const handleTableEnable = useCallback(async (mod: ModInfo) => {
    try {
      const resolution = await invoke<DepResolution>("auto_resolve_deps_cmd", {
        modId: mod.id,
        enabledModIds: activeProfile?.loadOrder ?? [],
      });
      if (resolution.toEnable.length > 0) {
        for (const depId of resolution.toEnable) enableMod(depId);
      }
    } catch (err) {
      console.warn("Dep resolution unavailable, enabling without deps:", err);
    }
    enableMod(mod.id);
  }, [enableMod, activeProfile]);

  return (
    <div data-testid="page-mods" className="flex flex-col h-full">
      <LoadOrderToolbar />
      {configMissing && activeProfile?.serverConfigPath && (
        <MissingConfigBanner configPath={activeProfile.serverConfigPath} />
      )}
      {showSpinner ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-3">
          <div className="w-8 h-8 border-2 border-muted-foreground/30 border-t-primary rounded-full animate-spin" />
          <span className="text-sm text-muted-foreground">Discovering mods...</span>
          <span className="text-xs text-muted-foreground/60">Scanning workshop and local mod directories</span>
        </div>
      ) : viewMode === "table" ? (
        <div className="flex-1 flex min-h-0 overflow-hidden">
          <div className="flex flex-col overflow-hidden" style={{ width: `${splitPercent}%` }} data-testid="active-mod-list">
            <div className="px-3 py-1.5 border-b border-border">
              <h3 className="text-sm font-semibold">Enabled ({enabledMods.length})</h3>
            </div>
            <ModTable mods={enabledMods} isActive={true} onToggle={(mod) => disableMod(mod.id)} />
          </div>
          <VerticalDivider onSplitChange={handleSplitChange} />
          <div className="flex flex-col overflow-hidden" style={{ width: `${100 - splitPercent}%` }} data-testid="available-mod-list">
            <div className="px-3 py-1.5 border-b border-border">
              <h3 className="text-sm font-semibold">Available ({availableMods.length})</h3>
            </div>
            <ModTable mods={availableMods} isActive={false} onToggle={handleTableEnable} />
          </div>
        </div>
      ) : (
        <div className="flex-1 flex min-h-0 overflow-hidden">
          <div className="flex flex-col overflow-hidden" style={{ width: `${splitPercent}%` }} data-testid="active-mod-list">
            <ActiveModList />
          </div>
          <VerticalDivider onSplitChange={handleSplitChange} />
          <div className="flex flex-col overflow-hidden" style={{ width: `${100 - splitPercent}%` }} data-testid="available-mod-list">
            <AvailableModList />
          </div>
        </div>
      )}
      <ModDetailPanel />
    </div>
  );
}
