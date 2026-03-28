import { useState, useMemo, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "../../../shared/lib/utils";
import { assetUrl } from "../../../shared/lib/tauri";
import { Button } from "../../../shared/components/ui/button";
import { useModManagerStore } from "../store";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import type { ModInfo } from "../../../shared/types/modTypes";
import type { Replacement } from "../../extensions/types";
import { ConflictPanel } from "./ConflictPanel";
import { InspectorPanel } from "./InspectorPanel";
import { useCompatStore } from "../../compatibility/compatStore";
import { useSteamMeta, formatSteamDate, formatFileSize, formatCount } from "../hooks/useSteamMeta";

const MIN_HEIGHT = 80;
const MAX_HEIGHT = 500;
const DEFAULT_HEIGHT = 192;

export function ModDetailPanel() {
  const [collapsed, setCollapsed] = useState(false);
  const [tab, setTab] = useState<"details" | "inspector">("details");
  const [replacements, setReplacements] = useState<Replacement[]>([]);
  const [panelHeight, setPanelHeight] = useState(() => {
    const saved = localStorage.getItem("modzboid-panel-height");
    return saved ? Number(saved) : DEFAULT_HEIGHT;
  });
  const isDragging = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);
  const allMods = useModManagerStore((s) => s.allMods);
  const selectedModId = useModManagerStore((s) => s.selectedModId);
  const activeProfile = useActiveProfile();

  const handleDragStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;
    startY.current = e.clientY;
    startHeight.current = panelHeight;

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDragging.current) return;
      // Dragging up increases height (panel grows upward from bottom)
      const delta = startY.current - ev.clientY;
      const newHeight = Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, startHeight.current + delta));
      setPanelHeight(newHeight);
      localStorage.setItem("modzboid-panel-height", String(newHeight));
    };

    const handleMouseUp = () => {
      isDragging.current = false;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [panelHeight]);

  useEffect(() => {
    invoke<Replacement[]>("get_replacements_cmd")
      .then(setReplacements)
      .catch(() => {
        /* replacements not available */
      });
  }, []);

  const selectedMod = useMemo(
    () => allMods.find((m) => m.id === selectedModId) ?? null,
    [allMods, selectedModId]
  );

  const enabledSet = useMemo(
    () => new Set(activeProfile?.loadOrder ?? []),
    [activeProfile]
  );

  return (
    <div data-testid="mod-detail-panel" className="border-t border-border bg-card shrink-0"
      style={{ height: collapsed ? undefined : `${panelHeight}px` }}
    >
      {/* Drag handle — large interactive area */}
      <div
        className="cursor-row-resize select-none group hover:bg-muted/50 transition-colors"
        onMouseDown={handleDragStart}
      >
        <div className="flex justify-center py-1.5">
          <div className="w-12 h-1.5 rounded-full bg-muted-foreground/30 group-hover:bg-muted-foreground/60 transition-colors" />
        </div>
      </div>

      {/* Toggle bar */}
      <button
        className="w-full flex items-center justify-between px-3 pb-1 text-xs text-muted-foreground hover:bg-muted/50 transition-colors"
        onClick={() => setCollapsed(!collapsed)}
      >
        <span>Mod Details</span>
        <span>{collapsed ? "\u25B2" : "\u25BC"}</span>
      </button>

      {!collapsed && (
        <>
          {/* Tab bar */}
          <div className="flex border-b border-border px-3">
            <button
              className={cn(
                "px-3 py-1.5 text-xs font-medium border-b-2 transition-colors",
                tab === "details"
                  ? "border-primary text-foreground"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              )}
              onClick={() => setTab("details")}
            >
              Details
            </button>
            <button
              className={cn(
                "px-3 py-1.5 text-xs font-medium border-b-2 transition-colors",
                tab === "inspector"
                  ? "border-primary text-foreground"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              )}
              onClick={() => setTab("inspector")}
            >
              Inspector
            </button>
          </div>

          <div className="overflow-auto flex-1 min-h-0">
            {tab === "details" ? (
              <div className="px-4 py-3">
                {!selectedMod ? (
                  <p className="text-sm text-muted-foreground">
                    Select a mod to see details
                  </p>
                ) : (
                  <ModDetails
                    mod={selectedMod}
                    allMods={allMods}
                    enabledSet={enabledSet}
                    replacements={replacements}
                  />
                )}
              </div>
            ) : (
              <InspectorPanel />
            )}
          </div>
        </>
      )}
    </div>
  );
}

function ModDetails({
  mod,
  allMods,
  enabledSet,
  replacements,
}: {
  mod: ModInfo;
  allMods: ModInfo[];
  enabledSet: Set<string>;
  replacements: Replacement[];
}) {
  const posterUrl = assetUrl(mod.posterPath);
  const modMap = useMemo(() => new Map(allMods.map((m) => [m.id, m])), [allMods]);
  const conflictsByMod = useModManagerStore((s) => s.conflictsByMod);
  const compatIssues = useCompatStore((s) => s.issuesByMod.get(mod.id) ?? -1);
  const { meta: steamMeta, loading: steamLoading } = useSteamMeta(mod.workshopId);

  const replacement = useMemo(
    () => replacements.find((r) => r.outdatedModId === mod.id) ?? null,
    [replacements, mod.id]
  );

  return (
    <div className="flex flex-col gap-2">
      {/* Replacement banner */}
      {replacement && (
        <div className="bg-yellow-500/10 border border-yellow-500/30 rounded p-3">
          <div className="text-sm font-medium text-yellow-500">Recommended Replacement</div>
          <div className="text-xs text-muted-foreground mt-1">
            Consider using <strong>{replacement.replacementModName || replacement.replacementModId}</strong> instead.
          </div>
          <div className="text-xs text-muted-foreground mt-0.5">{replacement.reason}</div>
        </div>
      )}

      <div className="flex gap-4">
      {/* Poster */}
      <div className="shrink-0 w-24 h-24 rounded-md overflow-hidden bg-muted flex items-center justify-center">
        {posterUrl ? (
          <img
            src={posterUrl}
            alt={mod.name}
            className="w-full h-full object-cover"
          />
        ) : (
          <span className="text-2xl font-bold text-muted-foreground">
            {mod.name.charAt(0).toUpperCase()}
          </span>
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <h4 className="text-sm font-semibold">{mod.name}</h4>
        <div className="flex items-center gap-2 mt-0.5">
          <span className="text-xs text-muted-foreground">
            {mod.authors.join(", ") || "Unknown Author"}
          </span>
          {mod.modVersion && (
            <VersionBadge label={`v${mod.modVersion}`} />
          )}
          {mod.source === "workshop" && (
            <VersionBadge label="Workshop" />
          )}
          {mod.source === "local" && (
            <VersionBadge label="Local" />
          )}
        </div>

        {mod.description && (
          <p className="text-xs text-muted-foreground mt-2 line-clamp-3">
            {mod.description}
          </p>
        )}

        {/* Dependencies */}
        {mod.requires.length > 0 && (
          <div className="mt-2">
            <span className="text-xs font-medium text-muted-foreground">
              Dependencies:
            </span>
            <div className="flex flex-wrap gap-1 mt-1">
              {mod.requires.map((depId) => {
                const depMod = modMap.get(depId);
                const isInstalled = depMod !== undefined;
                const isEnabled = enabledSet.has(depId);
                return (
                  <span
                    key={depId}
                    className={cn(
                      "inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded",
                      isEnabled
                        ? "bg-success/10 text-success"
                        : isInstalled
                          ? "bg-warning/10 text-warning"
                          : "bg-destructive/10 text-destructive"
                    )}
                  >
                    {isEnabled ? "\u2713" : isInstalled ? "\u25CB" : "\u2717"}{" "}
                    {depMod?.name ?? depId}
                  </span>
                );
              })}
            </div>
          </div>
        )}

        <ConflictPanel conflicts={conflictsByMod.get(mod.id) ?? []} />

        {/* Compatibility status */}
        {compatIssues >= 0 && (
          <div className={cn(
            "mt-2 px-3 py-2 rounded-md text-xs border",
            compatIssues === 0
              ? "bg-green-500/10 border-green-500/30 text-green-400"
              : "bg-yellow-500/10 border-yellow-500/30 text-yellow-400"
          )}>
            {compatIssues === 0
              ? "B42 Compatible — no issues found"
              : `${compatIssues} compatibility issue${compatIssues !== 1 ? "s" : ""} — check Compatibility tab for details`}
          </div>
        )}

        {/* Version folder selector */}
        {mod.versionFolders.length > 0 && (
          <VersionFolderSelector mod={mod} />
        )}

        {/* Steam Workshop stats */}
        {mod.workshopId && steamMeta && (
          <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span title="Subscribers">{formatCount(steamMeta.subscriptions)} subs</span>
            <span title="Favorites">{formatCount(steamMeta.favorited)} favs</span>
            <span title="File size">{formatFileSize(steamMeta.fileSize)}</span>
            <span title="Last updated">Updated {formatSteamDate(steamMeta.timeUpdated)}</span>
            {steamMeta.tags.length > 0 && (
              <span className="flex gap-1">
                {steamMeta.tags.slice(0, 4).map((t) => (
                  <span key={t} className="bg-muted px-1.5 py-0.5 rounded text-[10px]">{t}</span>
                ))}
              </span>
            )}
          </div>
        )}
        {mod.workshopId && steamLoading && (
          <div className="mt-2 text-xs text-muted-foreground/50">Loading Steam data...</div>
        )}

        {/* Mod ID and path info */}
        <div className="mt-2 flex items-center gap-3">
          <span className="text-xs text-muted-foreground/60">
            ID: {mod.id}
          </span>
          {mod.workshopId && (
            <Button
              variant="link"
              size="sm"
              className="text-xs h-auto p-0"
              onClick={() => {
                if (mod.url) {
                  window.open(mod.url, "_blank");
                }
              }}
            >
              Workshop Page
            </Button>
          )}
        </div>
      </div>
      </div>
    </div>
  );
}

function VersionFolderSelector({ mod }: { mod: ModInfo }) {
  const activeProfile = useActiveProfile();
  const setVersionOverride = useProfileStore((s) => s.setVersionOverride);
  const setMods = useModManagerStore((s) => s.setMods);
  const allMods = useModManagerStore((s) => s.allMods);

  const override = activeProfile?.versionOverrides?.[mod.id] ?? null;
  const isOverridden = override !== null;

  const handleChange = useCallback(async (folder: string) => {
    const isAuto = folder === "__auto__";
    setVersionOverride(mod.id, isAuto ? null : folder);

    try {
      const updated = await invoke<ModInfo>("rescan_mod_version", {
        modId: mod.id,
        versionFolder: isAuto ? null : folder,
      });
      setMods(allMods.map((m) => (m.id === updated.id ? updated : m)));
    } catch (err) {
      console.warn("Failed to rescan mod version:", err);
    }
  }, [mod.id, allMods, setVersionOverride, setMods]);

  return (
    <div className="mt-2">
      <span className="text-xs font-medium text-muted-foreground">
        Version folder:
      </span>
      <div className="flex items-center gap-2 mt-1">
        <select
          className="text-xs bg-muted border border-border rounded px-2 py-1"
          value={override ?? "__auto__"}
          onChange={(e) => handleChange(e.target.value)}
        >
          <option value="__auto__">
            Auto ({mod.activeVersionFolder ?? "root"})
          </option>
          {mod.versionFolders.map((vf) => (
            <option key={vf} value={vf}>
              {vf}{override === vf ? " (pinned)" : ""}
            </option>
          ))}
        </select>
        {isOverridden && (
          <button
            className="text-xs text-warning hover:text-foreground transition-colors"
            onClick={() => handleChange("__auto__")}
            title="Reset to auto-detected version"
          >
            Reset
          </button>
        )}
      </div>
    </div>
  );
}

function VersionBadge({ label }: { label: string }) {
  return (
    <span className="inline-flex items-center px-1.5 py-0.5 text-xs rounded bg-muted text-muted-foreground">
      {label}
    </span>
  );
}
