import * as React from "react";
import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../../../shared/components/ui/button";
import { ProfileCard } from "../components/ProfileCard";
import { ProfileCreateDialog } from "../components/ProfileCreateDialog";
import { ExportDialog } from "../../sharing/components/ExportDialog";
import { ImportDialog } from "../../sharing/components/ImportDialog";
import { useProfiles } from "../hooks/useProfiles";
import { useProfileStore } from "../store";
import { toast } from "../../../shared/components/ui/toaster";

export default function ProfilesPage() {
  const { profiles, loadProfiles } = useProfiles();
  const activeProfileId = useProfileStore((s) => s.activeProfileId);
  const detectedConfigs = useProfileStore((s) => s.detectedConfigs);
  const manageServerConfig = useProfileStore((s) => s.manageServerConfig);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [exportOpen, setExportOpen] = React.useState(false);
  const [importListOpen, setImportListOpen] = React.useState(false);
  const importInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  const handleImport = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (event) => {
      const text = event.target?.result;
      if (typeof text !== "string") return;
      try {
        JSON.parse(text); // Validate it's valid JSON
        await invoke("import_profile_cmd", { json: text });
        await loadProfiles();
      } catch (err) {
        console.error("Failed to import profile:", err);
        toast({ title: "Error", description: "Failed to import profile", variant: "destructive" });
      }
    };
    reader.readAsText(file);

    // Reset input so same file can be re-imported
    e.target.value = "";
  };

  return (
    <div data-testid="page-profiles" className="p-6 flex flex-col gap-6 h-full overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-foreground">Profiles</h1>
        <div className="flex items-center gap-2">
          <Button
            data-testid="btn-export-list"
            variant="outline"
            size="sm"
            onClick={() => setExportOpen(true)}
            disabled={!activeProfileId}
          >
            Export List
          </Button>
          <Button
            data-testid="btn-import-list"
            variant="outline"
            size="sm"
            onClick={() => setImportListOpen(true)}
          >
            Import List
          </Button>
          <Button
            data-testid="btn-import-file"
            variant="outline"
            size="sm"
            onClick={() => importInputRef.current?.click()}
          >
            Import
          </Button>
          <input
            ref={importInputRef}
            type="file"
            accept=".json"
            className="hidden"
            onChange={handleImport}
          />
          <Button data-testid="btn-create-profile" size="sm" onClick={() => setCreateOpen(true)}>
            + Create Profile
          </Button>
        </div>
      </div>

      {/* Profile grid */}
      {profiles.length === 0 ? (
        <div data-testid="profiles-empty" className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-3">
            <p className="text-muted-foreground text-sm">No profiles yet.</p>
            <Button onClick={() => setCreateOpen(true)}>
              Create your first profile
            </Button>
          </div>
        </div>
      ) : (
        <div data-testid="profile-grid" className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {profiles.map((profile) => (
            <ProfileCard
              key={profile.id}
              profile={profile}
              isActive={profile.id === activeProfileId}
            />
          ))}
        </div>
      )}

      {/* Detected server configs (not managed) */}
      {detectedConfigs.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-sm font-medium text-muted-foreground">
            Detected server configs (not managed)
          </h2>
          <div className="space-y-2">
            {detectedConfigs.map((cfg) => (
              <div
                key={cfg.path}
                className="flex items-center justify-between gap-3 px-4 py-3 border border-dashed border-border rounded-lg bg-muted/30"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-muted-foreground shrink-0">&#128196;</span>
                  <span className="text-sm font-medium text-foreground truncate">{cfg.name}</span>
                  <span className="text-xs text-muted-foreground truncate hidden sm:inline" title={cfg.path}>
                    {cfg.path}
                  </span>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => manageServerConfig(cfg.path)}
                >
                  Manage
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}

      <ProfileCreateDialog open={createOpen} onOpenChange={setCreateOpen} />
      <ExportDialog
        open={exportOpen}
        onOpenChange={setExportOpen}
        profileId={activeProfileId}
      />
      <ImportDialog
        open={importListOpen}
        onOpenChange={setImportListOpen}
        onImported={loadProfiles}
      />
    </div>
  );
}
