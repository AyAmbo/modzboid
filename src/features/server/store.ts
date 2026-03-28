import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "../../shared/components/ui/toaster";
import type {
  ServerConfig,
  ServerConfigInfo,
  SandboxVarsConfig,
  ServerSettingUpdate,
  SandboxSettingUpdate,
} from "./types";

interface ServerStore {
  configs: ServerConfigInfo[];
  activeConfigPath: string | null;
  serverConfig: ServerConfig | null;
  sandboxVars: SandboxVarsConfig | null;
  dirtySettings: Map<string, string>;
  dirtySandbox: Map<string, string>;
  isLoading: boolean;

  loadConfigs: () => Promise<void>;
  selectConfig: (path: string) => Promise<void>;
  updateSetting: (key: string, value: string) => void;
  updateSandboxSetting: (category: string | null, key: string, value: string) => void;
  saveServerConfig: () => Promise<void>;
  saveSandboxVars: () => Promise<void>;
  reloadConfig: () => Promise<void>;
  reloadSandbox: () => Promise<void>;
  undoConfigChanges: () => void;
  undoSandboxChanges: () => void;
}

export const useServerStore = create<ServerStore>((set, get) => ({
  configs: [],
  activeConfigPath: localStorage.getItem("modzboid-active-server"),
  serverConfig: null,
  sandboxVars: null,
  dirtySettings: new Map(),
  dirtySandbox: new Map(),
  isLoading: false,

  loadConfigs: async () => {
    try {
      const configs = await invoke<ServerConfigInfo[]>("list_server_configs_cmd");
      set({ configs });

      // Auto-select previously saved config if it matches one of the loaded configs
      const savedPath = localStorage.getItem("modzboid-active-server");
      if (savedPath && configs.some((c) => c.path === savedPath)) {
        const state = get();
        // Only auto-select if not already loaded
        if (!state.serverConfig) {
          await state.selectConfig(savedPath);
        }
      }
    } catch {
      // No configs found or directory not configured — treat as empty, not an error.
      // The ServerSelector already shows a "Configure your Zomboid user directory in Settings" message.
      set({ configs: [] });
    }
  },

  selectConfig: async (path) => {
    set({ isLoading: true, dirtySettings: new Map(), dirtySandbox: new Map() });
    localStorage.setItem("modzboid-active-server", path);
    try {
      const serverConfig = await invoke<ServerConfig>("load_server_config_cmd", { path });
      set({ activeConfigPath: path, serverConfig });

      // Try to load corresponding SandboxVars
      const sandboxPath = path.replace(/\.ini$/, "_SandboxVars.lua");
      try {
        const sandboxVars = await invoke<SandboxVarsConfig>("load_sandbox_vars_cmd", {
          path: sandboxPath,
        });
        set({ sandboxVars });
      } catch {
        set({ sandboxVars: null });
      }
    } catch (err) {
      console.error("Failed to load server config:", err);
      toast({ title: "Error", description: "Failed to load server config", variant: "destructive" });
    } finally {
      set({ isLoading: false });
    }
  },

  updateSetting: (key, value) => {
    const dirty = new Map(get().dirtySettings);
    dirty.set(key, value);
    set({ dirtySettings: dirty });
  },

  updateSandboxSetting: (category, key, value) => {
    const dirty = new Map(get().dirtySandbox);
    const mapKey = category ? `${category}.${key}` : key;
    dirty.set(mapKey, value);
    set({ dirtySandbox: dirty });
  },

  saveServerConfig: async () => {
    const { activeConfigPath, dirtySettings } = get();
    if (!activeConfigPath || dirtySettings.size === 0) return;

    const settings: ServerSettingUpdate[] = Array.from(dirtySettings.entries()).map(
      ([key, value]) => ({ key, value })
    );

    try {
      await invoke("save_server_config_cmd", { path: activeConfigPath, settings });
      // Reload to get fresh state
      const serverConfig = await invoke<ServerConfig>("load_server_config_cmd", {
        path: activeConfigPath,
      });
      set({ serverConfig, dirtySettings: new Map() });
      toast({ title: "Config Saved", description: "Server configuration saved" });
    } catch (err) {
      console.error("Failed to save server config:", err);
      toast({ title: "Error", description: "Failed to save server config", variant: "destructive" });
    }
  },

  saveSandboxVars: async () => {
    const { activeConfigPath, dirtySandbox } = get();
    if (!activeConfigPath || dirtySandbox.size === 0) return;

    const sandboxPath = activeConfigPath.replace(/\.ini$/, "_SandboxVars.lua");
    const updates: SandboxSettingUpdate[] = Array.from(dirtySandbox.entries()).map(
      ([mapKey, value]) => {
        const dotIdx = mapKey.indexOf(".");
        if (dotIdx === -1) {
          return { category: null, key: mapKey, value };
        }
        return { category: mapKey.substring(0, dotIdx), key: mapKey.substring(dotIdx + 1), value };
      }
    );

    try {
      await invoke("save_sandbox_vars_cmd", { path: sandboxPath, updates });
      const sandboxVars = await invoke<SandboxVarsConfig>("load_sandbox_vars_cmd", {
        path: sandboxPath,
      });
      set({ sandboxVars, dirtySandbox: new Map() });
      toast({ title: "Config Saved", description: "Sandbox variables saved" });
    } catch (err) {
      console.error("Failed to save sandbox vars:", err);
      toast({ title: "Error", description: "Failed to save sandbox variables", variant: "destructive" });
    }
  },

  reloadConfig: async () => {
    const { activeConfigPath } = get();
    if (!activeConfigPath) return;
    try {
      const serverConfig = await invoke<ServerConfig>("load_server_config_cmd", {
        path: activeConfigPath,
      });
      set({ serverConfig, dirtySettings: new Map() });
      toast({ title: "Reloaded", description: "Server config reloaded from disk" });
    } catch (err) {
      console.error("Failed to reload server config:", err);
      toast({ title: "Error", description: "Failed to reload server config", variant: "destructive" });
    }
  },

  reloadSandbox: async () => {
    const { activeConfigPath } = get();
    if (!activeConfigPath) return;
    const sandboxPath = activeConfigPath.replace(/\.ini$/, "_SandboxVars.lua");
    try {
      const sandboxVars = await invoke<SandboxVarsConfig>("load_sandbox_vars_cmd", {
        path: sandboxPath,
      });
      set({ sandboxVars, dirtySandbox: new Map() });
      toast({ title: "Reloaded", description: "Sandbox variables reloaded from disk" });
    } catch {
      set({ sandboxVars: null, dirtySandbox: new Map() });
      toast({ title: "Reloaded", description: "No sandbox variables file found" });
    }
  },

  undoConfigChanges: () => {
    set({ dirtySettings: new Map() });
  },

  undoSandboxChanges: () => {
    set({ dirtySandbox: new Map() });
  },
}));
