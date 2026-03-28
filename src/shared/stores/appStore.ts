import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig } from "../types/app";

interface AppStore {
  config: AppConfig | null;
  isLoading: boolean;
  gameRunning: boolean;
  loadConfig: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  setGameRunning: (running: boolean) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  config: null,
  isLoading: true,
  gameRunning: false,
  loadConfig: async () => {
    try {
      const config = await invoke<AppConfig>("get_config_cmd");
      set({ config, isLoading: false });
    } catch {
      set({ isLoading: false });
    }
  },
  saveConfig: async (config) => {
    await invoke("save_config_cmd", { config });
    set({ config });
  },
  setGameRunning: (gameRunning) => set({ gameRunning }),
}));
