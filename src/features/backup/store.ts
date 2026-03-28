import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "../../shared/components/ui/toaster";
import type { BackupInfo } from "./types";

interface BackupStore {
  backups: BackupInfo[];
  isLoading: boolean;
  loadBackups: () => Promise<void>;
  createBackup: (name: string) => Promise<void>;
  restoreBackup: (path: string) => Promise<void>;
  deleteBackup: (path: string) => Promise<void>;
}

export const useBackupStore = create<BackupStore>((set, get) => ({
  backups: [],
  isLoading: false,

  loadBackups: async () => {
    set({ isLoading: true });
    try {
      const backups = await invoke<BackupInfo[]>("list_backups_cmd");
      set({ backups });
    } catch (err) {
      console.error("Failed to load backups:", err);
      toast({ title: "Error", description: "Failed to load backups", variant: "destructive" });
    } finally {
      set({ isLoading: false });
    }
  },

  createBackup: async (name) => {
    await invoke("create_backup_cmd", { name });
    await get().loadBackups();
  },

  restoreBackup: async (path) => {
    await invoke("restore_backup_cmd", { backupPath: path });
    toast({ title: "Backup Restored", description: "Profiles and configs restored. Reloading..." });
    window.location.reload();
  },

  deleteBackup: async (path) => {
    await invoke("delete_backup_cmd", { backupPath: path });
    await get().loadBackups();
  },
}));
