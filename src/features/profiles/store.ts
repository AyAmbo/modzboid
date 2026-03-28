import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "../../shared/components/ui/toaster";
import type { Profile } from "../../shared/types/profile";

interface ServerConfigInfo {
  name: string;
  path: string;
}

interface ProfileStore {
  profiles: Profile[];
  activeProfileId: string | null;
  loadOrderSnapshot: string[];
  detectedConfigs: ServerConfigInfo[];

  loadProfiles: () => Promise<void>;
  switchProfile: (id: string) => Promise<void>;
  enableMod: (modId: string) => void;
  disableMod: (modId: string) => void;
  reorderMods: (order: string[]) => void;
  revertToSnapshot: () => void;
  persistProfile: () => Promise<void>;
  syncFromServer: () => Promise<void>;
  syncToServer: () => Promise<void>;
  addModsFromServerIni: (filePath: string) => Promise<void>;
  replaceModsFromServerIni: (filePath: string) => Promise<void>;
  addModsFromProfile: (sourceProfileId: string) => Promise<void>;
  replaceModsFromProfile: (sourceProfileId: string) => Promise<void>;
  manageServerConfig: (configPath: string) => Promise<void>;
  unlinkServerConfig: () => void;
  setLoadOrder: (order: string[]) => void;
  setVersionOverride: (modId: string, versionFolder: string | null) => void;
  isDirty: () => boolean;
  changeCount: () => number;
}

let persistTimeout: ReturnType<typeof setTimeout> | null = null;

export const useProfileStore = create<ProfileStore>((set, get) => ({
  profiles: [],
  activeProfileId: null,
  loadOrderSnapshot: [],
  detectedConfigs: [],

  loadProfiles: async () => {
    const profiles = await invoke<Profile[]>("list_profiles_cmd");
    const { activeProfileId } = get();

    // Preserve current selection if it still exists, otherwise fall back to default/first
    const currentStillExists = activeProfileId && profiles.some((p) => p.id === activeProfileId);
    const selectedProfile = currentStillExists
      ? profiles.find((p) => p.id === activeProfileId)!
      : profiles.find((p) => p.isDefault) || profiles[0];

    set({
      profiles,
      activeProfileId: selectedProfile?.id || null,
      loadOrderSnapshot: selectedProfile?.loadOrder || [],
    });

    // Discover unmanaged server configs (no auto-create)
    try {
      const configs = await invoke<ServerConfigInfo[]>("list_server_configs_cmd");
      const linkedPaths = new Set(
        profiles.filter((p) => p.serverConfigPath).map((p) => p.serverConfigPath)
      );
      set({ detectedConfigs: configs.filter((c) => !linkedPaths.has(c.path)) });
    } catch {
      set({ detectedConfigs: [] });
    }
  },

  switchProfile: async (id) => {
    const profile = await invoke<Profile>("get_profile_cmd", {
      profileId: id,
    });
    set({ activeProfileId: id, loadOrderSnapshot: [...profile.loadOrder] });
  },

  enableMod: (modId) => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId
        ? { ...p, loadOrder: [...p.loadOrder, modId] }
        : p
    );
    set({ profiles: updated });
    get().persistProfile();
  },

  disableMod: (modId) => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId
        ? { ...p, loadOrder: p.loadOrder.filter((id) => id !== modId) }
        : p
    );
    set({ profiles: updated });
    get().persistProfile();
  },

  reorderMods: (order) => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId ? { ...p, loadOrder: order } : p
    );
    set({ profiles: updated });
    get().persistProfile();
  },

  revertToSnapshot: () => {
    const { profiles, activeProfileId, loadOrderSnapshot } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId
        ? { ...p, loadOrder: [...loadOrderSnapshot] }
        : p
    );
    set({ profiles: updated });
    get().persistProfile();
  },

  setLoadOrder: (order) => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId ? { ...p, loadOrder: order } : p
    );
    set({ profiles: updated, loadOrderSnapshot: [...order] });
    get().persistProfile();
  },

  setVersionOverride: (modId, versionFolder) => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) => {
      if (p.id !== activeProfileId) return p;
      const overrides = { ...p.versionOverrides };
      if (versionFolder === null) {
        delete overrides[modId];
      } else {
        overrides[modId] = versionFolder;
      }
      return { ...p, versionOverrides: overrides };
    });
    set({ profiles: updated });
    get().persistProfile();
  },

  persistProfile: async () => {
    if (persistTimeout) clearTimeout(persistTimeout);
    persistTimeout = setTimeout(async () => {
      const { profiles, activeProfileId } = get();
      const profile = profiles.find((p) => p.id === activeProfileId);
      if (profile) {
        try {
          await invoke("update_profile_cmd", { profile });
        } catch (err) {
          console.error("Failed to persist profile:", err);
          toast({ title: "Error", description: "Failed to save profile changes", variant: "destructive" });
        }
      }
    }, 500);
  },

  // Load mods from server.ini into the active profile's loadOrder
  syncFromServer: async () => {
    const { profiles, activeProfileId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile?.serverConfigPath) {
      toast({ title: "Not a server profile", description: "This profile is not linked to a server config", variant: "destructive" });
      return;
    }
    try {
      const modIds = await invoke<string[]>("load_mods_from_server_ini_cmd", {
        filePath: profile.serverConfigPath,
      });
      const updated = profiles.map((p) =>
        p.id === activeProfileId ? { ...p, loadOrder: modIds } : p
      );
      set({ profiles: updated, loadOrderSnapshot: [...modIds] });
      await invoke("update_profile_cmd", { profile: { ...profile, loadOrder: modIds } });
      toast({ title: "Synced", description: `Loaded ${modIds.length} mods from server.ini` });
    } catch (err) {
      console.error("Failed to sync from server:", err);
      toast({ title: "Error", description: `Failed to load mods from server: ${err}`, variant: "destructive" });
    }
  },

  // Write the active profile's loadOrder back to server.ini
  syncToServer: async () => {
    const { profiles, activeProfileId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile?.serverConfigPath) {
      toast({ title: "Not a server profile", description: "This profile is not linked to a server config", variant: "destructive" });
      return;
    }
    try {
      await invoke("save_mods_to_server_ini_cmd", {
        filePath: profile.serverConfigPath,
        loadOrder: profile.loadOrder,
      });
      // Reset snapshot so dirty state clears
      set({ loadOrderSnapshot: [...profile.loadOrder] });
      toast({ title: "Saved", description: `Wrote ${profile.loadOrder.length} mods to server.ini` });
    } catch (err) {
      console.error("Failed to sync to server:", err);
      toast({ title: "Error", description: `Failed to save mods to server: ${err}`, variant: "destructive" });
    }
  },

  // Add mods from a server.ini into current profile (merge — only adds missing)
  addModsFromServerIni: async (filePath: string) => {
    const { profiles, activeProfileId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return;
    try {
      const modIds = await invoke<string[]>("load_mods_from_server_ini_cmd", { filePath });
      if (modIds.length === 0) {
        toast({ title: "No mods found", description: "The server.ini has no Mods= entries", variant: "destructive" });
        return;
      }
      const existing = new Set(profile.loadOrder);
      const newMods = modIds.filter((id) => !existing.has(id));
      if (newMods.length === 0) {
        toast({ title: "No new mods", description: "All mods are already in your load order" });
        return;
      }
      const merged = [...profile.loadOrder, ...newMods];
      const updated = profiles.map((p) =>
        p.id === activeProfileId ? { ...p, loadOrder: merged } : p
      );
      set({ profiles: updated });
      get().persistProfile();
      const fileName = filePath.split(/[\\/]/).pop();
      toast({ title: "Added mods", description: `Added ${newMods.length} mods from ${fileName}` });
    } catch (err) {
      console.error("Failed to add mods from server.ini:", err);
      toast({ title: "Error", description: `Failed to add mods: ${err}`, variant: "destructive" });
    }
  },

  // Replace mod list from a server.ini (destructive — replaces entire load order)
  replaceModsFromServerIni: async (filePath: string) => {
    const { profiles, activeProfileId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return;
    try {
      const modIds = await invoke<string[]>("load_mods_from_server_ini_cmd", { filePath });
      if (modIds.length === 0) {
        toast({ title: "No mods found", description: "The server.ini has no Mods= entries", variant: "destructive" });
        return;
      }
      const updated = profiles.map((p) =>
        p.id === activeProfileId ? { ...p, loadOrder: modIds } : p
      );
      set({ profiles: updated });
      get().persistProfile();
      const fileName = filePath.split(/[\\/]/).pop();
      toast({ title: "Replaced mod list", description: `Set ${modIds.length} mods from ${fileName}` });
    } catch (err) {
      console.error("Failed to replace from server.ini:", err);
      toast({ title: "Error", description: `Failed to replace: ${err}`, variant: "destructive" });
    }
  },

  // Add mods from another profile (merge — only adds missing)
  addModsFromProfile: async (sourceProfileId: string) => {
    const { profiles, activeProfileId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    const source = profiles.find((p) => p.id === sourceProfileId);
    if (!profile || !source) return;
    const existing = new Set(profile.loadOrder);
    const newMods = source.loadOrder.filter((id) => !existing.has(id));
    if (newMods.length === 0) {
      toast({ title: "No new mods", description: "All mods are already in your load order" });
      return;
    }
    const merged = [...profile.loadOrder, ...newMods];
    const updated = profiles.map((p) =>
      p.id === activeProfileId ? { ...p, loadOrder: merged } : p
    );
    set({ profiles: updated });
    get().persistProfile();
    toast({ title: "Added mods", description: `Added ${newMods.length} mods from "${source.name}"` });
  },

  // Replace mod list from another profile (destructive)
  replaceModsFromProfile: async (sourceProfileId: string) => {
    const { profiles, activeProfileId } = get();
    const source = profiles.find((p) => p.id === sourceProfileId);
    if (!source) return;
    const updated = profiles.map((p) =>
      p.id === activeProfileId ? { ...p, loadOrder: [...source.loadOrder] } : p
    );
    set({ profiles: updated });
    get().persistProfile();
    toast({ title: "Replaced mod list", description: `Set ${source.loadOrder.length} mods from "${source.name}"` });
  },

  // Create a server profile for a detected .ini config (one-click [Manage])
  manageServerConfig: async (configPath: string) => {
    const { profiles } = get();
    // 1-to-1 guard: check if already managed
    const existing = profiles.find((p) => p.serverConfigPath === configPath);
    if (existing) {
      toast({ title: "Already managed", description: `This config is managed by "${existing.name}". Switch to that profile instead.`, variant: "destructive" });
      return;
    }
    try {
      const fileName = configPath.split(/[\\/]/).pop() || "server";
      const name = fileName.replace(/\.ini$/, "");
      const newProfile = await invoke<Profile>("create_profile_cmd", {
        name,
        profileType: "server",
        serverConfigPath: configPath,
      });
      // Auto-import mods from .ini
      try {
        const modIds = await invoke<string[]>("load_mods_from_server_ini_cmd", { filePath: configPath });
        if (modIds.length > 0) {
          newProfile.loadOrder = modIds;
          await invoke("update_profile_cmd", { profile: newProfile });
        }
      } catch {
        // .ini might not have Mods= line
      }
      await get().loadProfiles();
      // Switch to the new profile
      set({ activeProfileId: newProfile.id, loadOrderSnapshot: [...newProfile.loadOrder] });
      toast({ title: "Profile created", description: `Now managing ${name}` });
    } catch (err) {
      console.error("Failed to manage server config:", err);
      toast({ title: "Error", description: `Failed to create profile: ${err}`, variant: "destructive" });
    }
  },

  // Unlink a server profile (convert to standalone, keeps mods)
  unlinkServerConfig: () => {
    const { profiles, activeProfileId } = get();
    const updated = profiles.map((p) =>
      p.id === activeProfileId ? { ...p, serverConfigPath: null, type: "singleplayer" as const } : p
    );
    set({ profiles: updated });
    get().persistProfile();
    toast({ title: "Unlinked", description: "Profile converted to standalone (mods kept)" });
  },

  // Dirty state: compare current load order against snapshot
  isDirty: () => {
    const { profiles, activeProfileId, loadOrderSnapshot } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return false;
    if (profile.loadOrder.length !== loadOrderSnapshot.length) return true;
    return profile.loadOrder.some((id, i) => id !== loadOrderSnapshot[i]);
  },

  changeCount: () => {
    const { profiles, activeProfileId, loadOrderSnapshot } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return 0;
    const current = new Set(profile.loadOrder);
    const snapshot = new Set(loadOrderSnapshot);
    const added = profile.loadOrder.filter((id) => !snapshot.has(id)).length;
    const removed = loadOrderSnapshot.filter((id) => !current.has(id)).length;
    // Also count reorder if lists differ but same elements
    const reordered = added === 0 && removed === 0 && profile.loadOrder.some((id, i) => id !== loadOrderSnapshot[i]) ? 1 : 0;
    return added + removed + reordered;
  },
}));

export const useActiveProfile = () =>
  useProfileStore(
    (state) =>
      state.profiles.find((p) => p.id === state.activeProfileId) ?? null
  );
