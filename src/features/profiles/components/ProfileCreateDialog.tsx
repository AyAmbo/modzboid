import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../../../shared/components/ui/dialog";
import { Button } from "../../../shared/components/ui/button";
import { Input } from "../../../shared/components/ui/input";
import { cn } from "../../../shared/lib/utils";
import type { ProfileType } from "../../../shared/types/profile";
import { useProfiles } from "../hooks/useProfiles";
import { useProfileStore } from "../store";

interface ServerConfigInfo {
  name: string;
  path: string;
}

interface ProfileCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ProfileCreateDialog({
  open,
  onOpenChange,
}: ProfileCreateDialogProps) {
  const [name, setName] = React.useState("");
  const [type, setType] = React.useState<ProfileType>("singleplayer");
  const [serverConfigs, setServerConfigs] = React.useState<ServerConfigInfo[]>([]);
  const [selectedServerConfig, setSelectedServerConfig] = React.useState("");
  const [isCreating, setIsCreating] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const { createProfile } = useProfiles();
  const profiles = useProfileStore((s) => s.profiles);

  // Load server configs when dialog opens and type is server
  React.useEffect(() => {
    if (open && type === "server") {
      invoke<ServerConfigInfo[]>("list_server_configs_cmd")
        .then(setServerConfigs)
        .catch(() => setServerConfigs([]));
    }
  }, [open, type]);

  // Filter out already-managed configs (1-to-1 mapping enforcement)
  const linkedPaths = new Set(
    profiles.filter((p) => p.serverConfigPath).map((p) => p.serverConfigPath)
  );
  const availableConfigs = serverConfigs.filter((cfg) => !linkedPaths.has(cfg.path));

  const handleCreate = async () => {
    if (!name.trim()) {
      setError("Profile name is required.");
      return;
    }

    // 1-to-1 .ini mapping check
    if (selectedServerConfig) {
      const existing = profiles.find((p) => p.serverConfigPath === selectedServerConfig);
      if (existing) {
        setError(`This config is already managed by "${existing.name}".`);
        return;
      }
    }

    setIsCreating(true);
    setError(null);

    try {
      await createProfile(name.trim(), type, selectedServerConfig || undefined);
      setName("");
      setType("singleplayer");
      setSelectedServerConfig("");
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create profile.");
    } finally {
      setIsCreating(false);
    }
  };

  const handleCancel = () => {
    setName("");
    setType("singleplayer");
    setError(null);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="dialog-create-profile">
        <DialogHeader>
          <DialogTitle>Create Profile</DialogTitle>
          <DialogDescription>
            Set up a new mod profile for your game.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">
              Profile Name
            </label>
            <Input
              placeholder="My Profile"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
              }}
              autoFocus
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">Type</label>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setType("singleplayer")}
                className={cn(
                  "flex-1 py-2 px-3 rounded-md text-sm font-medium border transition-colors",
                  type === "singleplayer"
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-transparent border-border text-muted-foreground hover:bg-muted"
                )}
              >
                Singleplayer
              </button>
              <button
                type="button"
                onClick={() => setType("server")}
                className={cn(
                  "flex-1 py-2 px-3 rounded-md text-sm font-medium border transition-colors",
                  type === "server"
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-transparent border-border text-muted-foreground hover:bg-muted"
                )}
              >
                Server
              </button>
            </div>
          </div>

          {type === "server" && availableConfigs.length > 0 && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-foreground">
                Link to Server Config
              </label>
              <select
                value={selectedServerConfig}
                onChange={(e) => setSelectedServerConfig(e.target.value)}
                className="w-full px-3 py-2 text-sm rounded-md border border-border bg-muted text-foreground"
              >
                <option value="">None (standalone)</option>
                {availableConfigs.map((cfg) => (
                  <option key={cfg.path} value={cfg.path}>
                    {cfg.name}
                  </option>
                ))}
              </select>
              <p className="text-xs text-muted-foreground">
                Linked profiles can load/save mods from the server.ini file.
              </p>
            </div>
          )}

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleCancel} disabled={isCreating}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={isCreating || !name.trim()}>
            {isCreating ? "Creating..." : "Create"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
