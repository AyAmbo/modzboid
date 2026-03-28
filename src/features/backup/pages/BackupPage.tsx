import { useEffect, useState } from "react";
import { formatDistanceToNow } from "date-fns";
import { Button } from "../../../shared/components/ui/button";
import { Input } from "../../../shared/components/ui/input";
import { useBackupStore } from "../store";
import { ConfirmDialog } from "../../../shared/components/ui/confirm-dialog";
import { toast } from "../../../shared/components/ui/toaster";

export default function BackupPage() {
  const { backups, isLoading, loadBackups, createBackup, restoreBackup, deleteBackup } =
    useBackupStore();
  const [creating, setCreating] = useState(false);
  const [backupName, setBackupName] = useState("");
  const [confirmRestore, setConfirmRestore] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  useEffect(() => {
    loadBackups();
  }, [loadBackups]);

  const handleCreate = async () => {
    if (!backupName.trim()) return;
    setCreating(true);
    try {
      await createBackup(backupName.trim());
      setBackupName("");
      toast({ title: "Backup Created", description: `Backup "${backupName.trim()}" created successfully` });
    } catch (err) {
      console.error("Failed to create backup:", err);
      toast({ title: "Error", description: "Failed to create backup", variant: "destructive" });
    } finally {
      setCreating(false);
    }
  };

  const handleRestore = async (path: string) => {
    try {
      await restoreBackup(path);
      setConfirmRestore(null);
      toast({ title: "Backup Restored", description: "Backup restored successfully" });
    } catch (err) {
      console.error("Failed to restore backup:", err);
      toast({ title: "Error", description: "Failed to restore backup", variant: "destructive" });
    }
  };

  const handleDelete = async (path: string) => {
    try {
      await deleteBackup(path);
      setConfirmDelete(null);
      toast({ title: "Backup Deleted" });
    } catch (err) {
      console.error("Failed to delete backup:", err);
      toast({ title: "Error", description: "Failed to delete backup", variant: "destructive" });
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div data-testid="page-backups" className="p-6 max-w-3xl flex flex-col gap-6 h-full">
      {/* Header */}
      <div>
        <h1 className="text-xl font-semibold">Backups</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Back up your profiles and server configurations.
        </p>
      </div>

      {/* Create backup */}
      <div className="flex items-center gap-3">
        <Input
          data-testid="backup-name-input"
          placeholder="Backup name..."
          value={backupName}
          onChange={(e) => setBackupName(e.target.value)}
          className="max-w-xs"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCreate();
          }}
        />
        <Button data-testid="btn-create-backup" onClick={handleCreate} disabled={creating || !backupName.trim()}>
          {creating ? "Creating..." : "Create Backup"}
        </Button>
      </div>

      {/* Backup list */}
      {isLoading ? (
        <div className="text-sm text-muted-foreground">Loading...</div>
      ) : backups.length === 0 ? (
        <div data-testid="backups-empty" className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-2">
            <p className="text-muted-foreground text-sm">No backups yet.</p>
            <p className="text-muted-foreground/60 text-xs">
              Create a backup to save your profiles and server configs.
            </p>
          </div>
        </div>
      ) : (
        <div data-testid="backup-list" className="space-y-2">
          {backups.map((backup) => {
            let timeAgo = "Unknown";
            try {
              timeAgo = formatDistanceToNow(new Date(backup.createdAt), { addSuffix: true });
            } catch {
              /* ignore parse errors */
            }

            return (
              <div
                key={backup.path}
                className="flex items-center gap-4 p-3 bg-card border border-border rounded-lg"
              >
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-sm">{backup.name}</div>
                  <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                    <span>{timeAgo}</span>
                    <span>{formatSize(backup.sizeBytes)}</span>
                    <span>{backup.profileCount} profile{backup.profileCount !== 1 ? "s" : ""}</span>
                    {backup.hasServerConfigs && <span>+ server configs</span>}
                  </div>
                </div>
                <div className="flex items-center gap-1.5">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setConfirmRestore(backup.path)}
                  >
                    Restore
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="text-destructive hover:text-destructive"
                    onClick={() => setConfirmDelete(backup.path)}
                  >
                    Delete
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Confirm restore dialog */}
      <ConfirmDialog
        open={confirmRestore !== null}
        onOpenChange={() => setConfirmRestore(null)}
        title="Restore Backup"
        description="This will overwrite your current profiles and server configs. Are you sure?"
        actions={[
          { label: "Cancel", variant: "outline", onClick: () => setConfirmRestore(null) },
          {
            label: "Restore",
            variant: "destructive",
            onClick: () => confirmRestore && handleRestore(confirmRestore),
          },
        ]}
      />

      {/* Confirm delete dialog */}
      <ConfirmDialog
        open={confirmDelete !== null}
        onOpenChange={() => setConfirmDelete(null)}
        title="Delete Backup"
        description="This backup will be permanently deleted."
        actions={[
          { label: "Cancel", variant: "outline", onClick: () => setConfirmDelete(null) },
          {
            label: "Delete",
            variant: "destructive",
            onClick: () => confirmDelete && handleDelete(confirmDelete),
          },
        ]}
      />
    </div>
  );
}
