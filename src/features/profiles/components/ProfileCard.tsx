import * as React from "react";
import { formatDistanceToNow } from "date-fns";
import { cn } from "../../../shared/lib/utils";
import { Button } from "../../../shared/components/ui/button";
import type { Profile } from "../../../shared/types/profile";
import { useProfiles } from "../hooks/useProfiles";
import { useProfileStore } from "../store";
import { UnsavedChangesDialog } from "./UnsavedChangesDialog";

interface ProfileCardProps {
  profile: Profile;
  isActive: boolean;
}

export function ProfileCard({ profile, isActive }: ProfileCardProps) {
  const { deleteProfile, duplicateProfile, exportProfile, switchProfile } =
    useProfiles();
  const syncToServer = useProfileStore((s) => s.syncToServer);
  const revertToSnapshot = useProfileStore((s) => s.revertToSnapshot);
  const activeProfile = useProfileStore((s) =>
    s.profiles.find((p) => p.id === s.activeProfileId)
  );
  const dirty = useProfileStore((s) => {
    const p = s.profiles.find((pr) => pr.id === s.activeProfileId);
    if (!p) return false;
    if (p.loadOrder.length !== s.loadOrderSnapshot.length) return true;
    return p.loadOrder.some((id, i) => id !== s.loadOrderSnapshot[i]);
  });
  const changes = useProfileStore((s) => {
    const p = s.profiles.find((pr) => pr.id === s.activeProfileId);
    if (!p) return 0;
    const current = new Set(p.loadOrder);
    const snapshot = new Set(s.loadOrderSnapshot);
    return p.loadOrder.filter((id) => !snapshot.has(id)).length +
      s.loadOrderSnapshot.filter((id) => !current.has(id)).length;
  });
  const [confirmDelete, setConfirmDelete] = React.useState(false);
  const [unsavedDialogOpen, setUnsavedDialogOpen] = React.useState(false);

  const handleCardClick = () => {
    if (!isActive) {
      if (activeProfile?.type === "server" && activeProfile?.serverConfigPath && dirty) {
        setUnsavedDialogOpen(true);
      } else {
        switchProfile(profile.id);
      }
    }
  };

  const handleDuplicate = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await duplicateProfile(profile.id, `${profile.name} (Copy)`);
  };

  const handleExport = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await exportProfile(profile.id);
  };

  const handleDeleteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setConfirmDelete(true);
  };

  const handleDeleteConfirm = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await deleteProfile(profile.id);
    setConfirmDelete(false);
  };

  const handleDeleteCancel = (e: React.MouseEvent) => {
    e.stopPropagation();
    setConfirmDelete(false);
  };

  const updatedAt = React.useMemo(() => {
    try {
      return formatDistanceToNow(new Date(profile.updatedAt), {
        addSuffix: true,
      });
    } catch {
      return "Unknown";
    }
  }, [profile.updatedAt]);

  const typeBadgeClass =
    profile.type === "singleplayer"
      ? "bg-blue-500/20 text-blue-400 border border-blue-500/30"
      : "bg-orange-500/20 text-orange-400 border border-orange-500/30";

  const typeLabel = profile.type === "singleplayer" ? "SP" : "Server";

  return (
    <div
      onClick={handleCardClick}
      className={cn(
        "group relative bg-card border rounded-lg p-4 flex flex-col gap-3 transition-colors",
        isActive
          ? "border-primary ring-1 ring-primary"
          : "border-border hover:border-muted-foreground cursor-pointer"
      )}
    >
      {/* Header */}
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold text-foreground truncate">
              {profile.name}
            </span>
            {profile.isDefault && (
              <span className="text-xs bg-muted text-muted-foreground px-1.5 py-0.5 rounded">
                Default
              </span>
            )}
          </div>
        </div>
        <span
          className={cn(
            "text-xs font-medium px-2 py-0.5 rounded shrink-0",
            typeBadgeClass
          )}
        >
          {typeLabel}
        </span>
      </div>

      {/* Stats */}
      <div className="flex items-center gap-4 text-sm text-muted-foreground">
        <span>{profile.loadOrder.length} mods</span>
        <span>Updated {updatedAt}</span>
      </div>

      {/* Server config path */}
      {profile.serverConfigPath && (
        <div className="text-xs text-muted-foreground/70 truncate" title={profile.serverConfigPath}>
          {profile.serverConfigPath.split(/[\\/]/).pop()}
        </div>
      )}

      {/* Active indicator */}
      {isActive && (
        <div className="text-xs text-primary font-medium">Active</div>
      )}

      {/* Actions */}
      {confirmDelete ? (
        <div className="flex items-center gap-2 pt-1 border-t border-border">
          <span className="text-xs text-muted-foreground flex-1">
            Delete this profile?
          </span>
          <Button
            size="sm"
            variant="destructive"
            onClick={handleDeleteConfirm}
          >
            Delete
          </Button>
          <Button size="sm" variant="outline" onClick={handleDeleteCancel}>
            Cancel
          </Button>
        </div>
      ) : (
        <div className="flex items-center gap-1 pt-1 border-t border-border opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
          <Button
            size="sm"
            variant="ghost"
            onClick={handleDuplicate}
            title="Duplicate"
          >
            Duplicate
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleExport}
            title="Export JSON"
          >
            Export
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleDeleteClick}
            className="text-destructive hover:text-destructive ml-auto"
            title="Delete"
          >
            Delete
          </Button>
        </div>
      )}

      {/* Unsaved changes dialog when switching away from dirty server profile */}
      <UnsavedChangesDialog
        open={unsavedDialogOpen}
        changeCount={changes}
        profileName={activeProfile?.name || ""}
        onSaveAndSwitch={async () => {
          await syncToServer();
          setUnsavedDialogOpen(false);
          switchProfile(profile.id);
        }}
        onDiscard={() => {
          revertToSnapshot();
          setUnsavedDialogOpen(false);
          switchProfile(profile.id);
        }}
        onCancel={() => setUnsavedDialogOpen(false)}
      />
    </div>
  );
}
