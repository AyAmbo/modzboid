import { invoke } from "@tauri-apps/api/core";
import { useProfileStore } from "../store";
import type { ProfileType } from "../../../shared/types/profile";

export function useProfiles() {
  const { profiles, loadProfiles, switchProfile } = useProfileStore();

  const createProfile = async (name: string, type: ProfileType, serverConfigPath?: string) => {
    await invoke("create_profile_cmd", {
      name,
      profileType: type,
      serverConfigPath: serverConfigPath || null,
    });
    await loadProfiles();
  };

  const deleteProfile = async (id: string) => {
    await invoke("delete_profile_cmd", { profileId: id });
    await loadProfiles();
  };

  const duplicateProfile = async (id: string, newName: string) => {
    await invoke("duplicate_profile_cmd", { profileId: id, newName });
    await loadProfiles();
  };

  const exportProfile = async (id: string) => {
    const json = await invoke<string>("export_profile_cmd", { profileId: id });
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `profile-${id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return {
    profiles,
    createProfile,
    deleteProfile,
    duplicateProfile,
    exportProfile,
    switchProfile,
    loadProfiles,
  };
}
