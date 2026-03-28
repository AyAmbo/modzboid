import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ServerConfigInfo {
  name: string;
  path: string;
}
import { Button } from "../../../shared/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "../../../shared/components/ui/dropdown-menu";
import { Separator } from "../../../shared/components/ui/separator";
import { ExportDialog } from "../../sharing/components/ExportDialog";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import { useAppStore } from "../../../shared/stores/appStore";
import { useModManagerStore, type ColumnVisibility } from "../store";
import { useLoadOrder } from "../hooks/useLoadOrder";
import { useMods } from "../hooks/useMods";
import { toast } from "../../../shared/components/ui/toaster";

const COLUMN_LABELS: Record<keyof ColumnVisibility, string> = {
  version: "Version",
  author: "Author",
  size: "Mod Size",
  source: "Source",
  category: "Category",
  dependencies: "Dependencies",
  workshopId: "Workshop ID",
  lastModified: "Last Modified",
};

export function LoadOrderToolbar() {
  const { validate, autoSort, sortByName, sortByAuthor, issues } =
    useLoadOrder();
  const { refreshMods } = useMods();
  const revertToSnapshot = useProfileStore((s) => s.revertToSnapshot);
  const activeProfileId = useProfileStore((s) => s.activeProfileId);
  const syncFromServer = useProfileStore((s) => s.syncFromServer);
  const syncToServer = useProfileStore((s) => s.syncToServer);
  const addModsFromServerIni = useProfileStore((s) => s.addModsFromServerIni);
  const replaceModsFromServerIni = useProfileStore((s) => s.replaceModsFromServerIni);
  const addModsFromProfile = useProfileStore((s) => s.addModsFromProfile);
  const replaceModsFromProfile = useProfileStore((s) => s.replaceModsFromProfile);
  const profiles = useProfileStore((s) => s.profiles);
  const activeProfile = useActiveProfile();
  const isServerProfile = activeProfile?.type === "server" && !!activeProfile?.serverConfigPath;
  // Compute dirty state reactively in selector so Zustand triggers re-renders
  const dirty = useProfileStore((s) => {
    const profile = s.profiles.find((p) => p.id === s.activeProfileId);
    if (!profile) return false;
    if (profile.loadOrder.length !== s.loadOrderSnapshot.length) return true;
    return profile.loadOrder.some((id, i) => id !== s.loadOrderSnapshot[i]);
  });
  const changes = useProfileStore((s) => {
    const profile = s.profiles.find((p) => p.id === s.activeProfileId);
    if (!profile) return 0;
    const current = new Set(profile.loadOrder);
    const snapshot = new Set(s.loadOrderSnapshot);
    const added = profile.loadOrder.filter((id) => !snapshot.has(id)).length;
    const removed = s.loadOrderSnapshot.filter((id) => !current.has(id)).length;
    const reordered = added === 0 && removed === 0 && profile.loadOrder.some((id, i) => id !== s.loadOrderSnapshot[i]) ? 1 : 0;
    return added + removed + reordered;
  });
  const gameRunning = useAppStore((s) => s.gameRunning);
  const setGameRunning = useAppStore((s) => s.setGameRunning);
  const conflictsByMod = useModManagerStore((s) => s.conflictsByMod);
  const viewMode = useModManagerStore((s) => s.viewMode);
  const setViewMode = useModManagerStore((s) => s.setViewMode);
  const columns = useModManagerStore((s) => s.columns);
  const toggleColumn = useModManagerStore((s) => s.toggleColumn);
  const [launching, setLaunching] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [serverConfigs, setServerConfigs] = useState<ServerConfigInfo[]>([]);
  const [confirmReplace, setConfirmReplace] = useState<{ type: "ini" | "profile"; value: string } | null>(null);

  // Load server configs for the add/replace dropdown
  useEffect(() => {
    invoke<ServerConfigInfo[]>("list_server_configs_cmd")
      .then(setServerConfigs)
      .catch(() => setServerConfigs([]));
  }, []);

  const allConflicts = new Set<string>();
  conflictsByMod.forEach((conflicts) => {
    conflicts.forEach((c) => {
      if (c.severity !== "info") {
        allConflicts.add(c.message);
      }
    });
  });
  const conflictCount = allConflicts.size;

  const issueCount = issues.length;
  const errorCount = issues.filter((i) => i.severity === "error").length;

  const handleLaunch = useCallback(async () => {
    if (!activeProfileId) return;
    setLaunching(true);
    setGameRunning(true);
    try {
      await invoke("launch_game_cmd", { profileId: activeProfileId });
    } catch (err) {
      console.error("Failed to launch game:", err);
      toast({ title: "Error", description: "Failed to launch game", variant: "destructive" });
    } finally {
      setLaunching(false);
    }
  }, [activeProfileId, setGameRunning]);

  const otherProfiles = profiles.filter((p) => p.id !== activeProfileId);

  return (
    <div data-testid="load-order-toolbar" className="flex items-center gap-2 px-3 py-2 border-b border-border bg-card">
      {/* Sort dropdown */}
      <DropdownMenu>
        <DropdownMenuTrigger>
          <Button data-testid="btn-sort" variant="outline" size="sm">
            Sort
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onClick={autoSort}>Auto Sort</DropdownMenuItem>
          <DropdownMenuItem onClick={sortByName}>Sort by Name</DropdownMenuItem>
          <DropdownMenuItem onClick={sortByAuthor}>
            Sort by Author
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Validate */}
      <Button data-testid="btn-validate" variant="outline" size="sm" onClick={validate}>
        Validate
        {issueCount > 0 && (
          <span
            className={`ml-1.5 inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1 rounded-full text-xs font-medium ${
              errorCount > 0
                ? "bg-destructive text-white"
                : "bg-warning text-black"
            }`}
          >
            {issueCount}
          </span>
        )}
      </Button>

      {conflictCount > 0 && (
        <span className="text-xs text-warning">
          {conflictCount} conflict{conflictCount > 1 ? "s" : ""}
        </span>
      )}

      {/* Refresh */}
      <Button variant="outline" size="sm" onClick={refreshMods}>
        Refresh
      </Button>

      {/* Revert */}
      <Button variant="outline" size="sm" onClick={revertToSnapshot} disabled={!dirty}>
        Revert
      </Button>

      {/* Export */}
      <Button
        data-testid="btn-export"
        variant="outline"
        size="sm"
        onClick={() => setExportOpen(true)}
        disabled={!activeProfileId}
      >
        Export
      </Button>

      <Separator orientation="vertical" className="h-5" />

      {/* View mode toggle */}
      <div className="flex items-center rounded border border-border">
        <button
          className={`px-2 py-1 text-xs transition-colors ${viewMode === "card" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground"}`}
          onClick={() => setViewMode("card")}
          title="Card view"
        >
          {"\u2630"}
        </button>
        <button
          className={`px-2 py-1 text-xs transition-colors ${viewMode === "table" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground"}`}
          onClick={() => setViewMode("table")}
          title="Table view"
        >
          {"\u2637"}
        </button>
      </div>

      {/* Toggle Columns */}
      {viewMode === "table" && (
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button variant="outline" size="sm">
              Columns
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {(Object.keys(COLUMN_LABELS) as (keyof ColumnVisibility)[]).map((col) => (
              <DropdownMenuItem key={col} onClick={() => toggleColumn(col)}>
                <span className="w-4 inline-block">{columns[col] ? "\u2713" : ""}</span>
                {COLUMN_LABELS[col]}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}

      {/* Server sync section */}
      {isServerProfile && (
        <>
          <Separator orientation="vertical" className="h-5" />
          <span className="text-xs text-muted-foreground truncate max-w-[120px]" title={activeProfile.serverConfigPath || ""}>
            {activeProfile.serverConfigPath?.split(/[\\/]/).pop()}
          </span>
          {dirty ? (
            <span className="text-xs text-warning font-medium">
              {changes} unsaved change{changes !== 1 ? "s" : ""}
            </span>
          ) : (
            <span className="text-xs text-green-500">Synced</span>
          )}
          <Button variant="outline" size="sm" onClick={syncFromServer} title="Reload mods from server.ini">
            Reload
          </Button>
          <Button
            variant={dirty ? "default" : "outline"}
            size="sm"
            onClick={syncToServer}
            title="Save mods back to server.ini"
            className={dirty ? "font-semibold" : ""}
          >
            Save to Server
          </Button>
        </>
      )}

      {/* Add mods from... */}
      {(serverConfigs.length > 0 || otherProfiles.length > 0) && (
        <>
          <Separator orientation="vertical" className="h-5" />
          <DropdownMenu>
            <DropdownMenuTrigger>
              <Button variant="outline" size="sm" title="Add missing mods from another source (merge)">
                Add mods from...
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {otherProfiles.length > 0 && (
                <>
                  <div className="px-2 py-1 text-xs text-muted-foreground font-medium">From profile</div>
                  {otherProfiles.map((p) => (
                    <DropdownMenuItem key={p.id} onClick={() => addModsFromProfile(p.id)}>
                      {p.name} ({p.loadOrder.length} mods)
                    </DropdownMenuItem>
                  ))}
                </>
              )}
              {serverConfigs.length > 0 && otherProfiles.length > 0 && <DropdownMenuSeparator />}
              {serverConfigs.length > 0 && (
                <>
                  <div className="px-2 py-1 text-xs text-muted-foreground font-medium">From server.ini</div>
                  {serverConfigs.map((cfg) => (
                    <DropdownMenuItem key={cfg.path} onClick={() => addModsFromServerIni(cfg.path)}>
                      {cfg.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Replace mod list from... */}
          <DropdownMenu>
            <DropdownMenuTrigger>
              <Button variant="outline" size="sm" title="Replace entire mod list from another source (destructive)">
                Replace from...
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {otherProfiles.length > 0 && (
                <>
                  <div className="px-2 py-1 text-xs text-muted-foreground font-medium">From profile</div>
                  {otherProfiles.map((p) => (
                    <DropdownMenuItem key={p.id} onClick={() => setConfirmReplace({ type: "profile", value: p.id })}>
                      {p.name} ({p.loadOrder.length} mods)
                    </DropdownMenuItem>
                  ))}
                </>
              )}
              {serverConfigs.length > 0 && otherProfiles.length > 0 && <DropdownMenuSeparator />}
              {serverConfigs.length > 0 && (
                <>
                  <div className="px-2 py-1 text-xs text-muted-foreground font-medium">From server.ini</div>
                  {serverConfigs.map((cfg) => (
                    <DropdownMenuItem key={cfg.path} onClick={() => setConfirmReplace({ type: "ini", value: cfg.path })}>
                      {cfg.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </>
      )}

      <div className="flex-1" />

      {/* Launch */}
      <Button
        data-testid="btn-launch"
        variant="default"
        size="sm"
        onClick={handleLaunch}
        disabled={launching || gameRunning}
      >
        {launching || gameRunning ? "Running..." : isServerProfile ? "Open Server Folder" : "Launch Game"}
      </Button>

      <ExportDialog
        open={exportOpen}
        onOpenChange={setExportOpen}
        profileId={activeProfileId}
      />

      {/* Replace confirmation dialog */}
      {confirmReplace && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setConfirmReplace(null)}>
          <div className="bg-card border border-border rounded-lg shadow-lg p-6 max-w-sm w-full mx-4" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-sm font-semibold mb-2">Replace mod list?</h3>
            <p className="text-xs text-muted-foreground mb-4">
              This will replace all {activeProfile?.loadOrder.length ?? 0} mods in your current profile.
              This action cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => setConfirmReplace(null)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={() => {
                  if (confirmReplace.type === "ini") {
                    replaceModsFromServerIni(confirmReplace.value);
                  } else {
                    replaceModsFromProfile(confirmReplace.value);
                  }
                  setConfirmReplace(null);
                }}
              >
                Replace
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
