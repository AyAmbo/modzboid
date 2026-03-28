import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useState, useEffect, useMemo } from "react";
import { useAppStore } from "../../../shared/stores/appStore";
import { useProfileStore, useActiveProfile } from "../../profiles/store";
import { Input } from "../../../shared/components/ui/input";
import type { AppConfig } from "../../../shared/types/app";

function PathRow({
  label,
  value,
  globalValue,
  onBrowse,
  onChange,
  onClear,
  verified,
}: {
  label: string;
  value: string | null;
  globalValue?: string | null;
  onBrowse: () => void;
  onChange: (path: string) => void;
  onClear?: () => void;
  verified?: boolean;
}) {
  const isOverridden = value !== null && value !== globalValue;
  const displayValue = value ?? globalValue ?? "";
  const [localValue, setLocalValue] = useState(displayValue);

  useEffect(() => {
    setLocalValue(value ?? globalValue ?? "");
  }, [value, globalValue]);

  const handleCommit = () => {
    const trimmed = localValue.trim();
    if (trimmed !== (value ?? globalValue ?? "")) {
      onChange(trimmed);
    }
  };

  return (
    <div className="flex items-center gap-3 py-2">
      <label className="w-36 text-sm text-muted-foreground shrink-0">
        {label}
        {isOverridden && (
          <span className="ml-1 text-xs text-warning" title="Overridden for this profile">*</span>
        )}
      </label>
      <Input
        className="flex-1 font-mono min-w-0"
        value={localValue}
        placeholder={globalValue || "Not set"}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleCommit}
        onKeyDown={(e) => {
          if (e.key === "Enter") handleCommit();
        }}
      />
      {verified !== undefined && (
        <span className={["text-sm shrink-0", verified ? "text-green-500" : "text-red-500"].join(" ")}>
          {verified ? "✓" : "✗"}
        </span>
      )}
      <button
        onClick={onBrowse}
        className="px-3 py-1.5 text-sm border border-border rounded hover:bg-muted transition-colors shrink-0"
      >
        Browse...
      </button>
      {isOverridden && onClear && (
        <button
          onClick={onClear}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors shrink-0"
          title="Reset to global default"
        >
          Reset
        </button>
      )}
    </div>
  );
}

export function PathSettings() {
  const { config, saveConfig } = useAppStore();
  const activeProfile = useActiveProfile();
  const profiles = useProfileStore((s) => s.profiles);
  // Resolved paths: profile override ?? global config
  const gamePath = activeProfile?.gamePath ?? config?.gamePath ?? null;
  const steamPath = activeProfile?.steamPath ?? config?.steamPath ?? null;
  const workshopPath = activeProfile?.workshopPath ?? config?.workshopPath ?? null;
  const zomboidUserDir = activeProfile?.zomboidUserDir ?? config?.zomboidUserDir ?? null;
  const localModsPath = config?.localModsPath ?? null;
  const gameVersion = activeProfile?.gameVersion ?? config?.gameVersion ?? null;

  const [localGamePath, setLocalGamePath] = useState(gamePath);
  const [localSteamPath, setLocalSteamPath] = useState(steamPath);
  const [localWorkshopPath, setLocalWorkshopPath] = useState(workshopPath);
  const [localZomboidDir, setLocalZomboidDir] = useState(zomboidUserDir);
  const [localLocalModsPath, setLocalLocalModsPath] = useState(localModsPath);
  const [localGameVersion, setLocalGameVersion] = useState(gameVersion);
  const [detectedBuild, setDetectedBuild] = useState<string | null>(null);
  const [gameVerified, setGameVerified] = useState<boolean | undefined>(undefined);
  const [steamVerified, setSteamVerified] = useState<boolean | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  // Sync local state when profile/config changes
  useEffect(() => {
    setLocalGamePath(activeProfile?.gamePath ?? config?.gamePath ?? null);
    setLocalSteamPath(activeProfile?.steamPath ?? config?.steamPath ?? null);
    setLocalWorkshopPath(activeProfile?.workshopPath ?? config?.workshopPath ?? null);
    setLocalZomboidDir(activeProfile?.zomboidUserDir ?? config?.zomboidUserDir ?? null);
    setLocalLocalModsPath(config?.localModsPath ?? null);
    setLocalGameVersion(activeProfile?.gameVersion ?? config?.gameVersion ?? null);
  }, [activeProfile, config]);

  // Detect build number when game path changes
  useEffect(() => {
    const path = localGamePath;
    if (path) {
      invoke<string | null>("detect_game_version_cmd", { path })
        .then((v) => setDetectedBuild(v ?? null))
        .catch(() => setDetectedBuild(null));
    } else {
      setDetectedBuild(null);
    }
  }, [localGamePath]);

  // Other profiles for copy-from dropdown
  const otherProfiles = useMemo(() =>
    profiles.filter((p) => p.id !== activeProfile?.id && (p.gamePath || p.workshopPath || p.zomboidUserDir)),
    [profiles, activeProfile?.id]
  );

  const copyPathsFrom = (profileId: string) => {
    const source = profiles.find((p) => p.id === profileId);
    if (!source) return;
    if (source.gamePath) setLocalGamePath(source.gamePath);
    if (source.steamPath) setLocalSteamPath(source.steamPath);
    if (source.workshopPath) setLocalWorkshopPath(source.workshopPath);
    if (source.zomboidUserDir) setLocalZomboidDir(source.zomboidUserDir);
    if (source.gameVersion) setLocalGameVersion(source.gameVersion);
  };

  const browseGame = async () => {
    const selected = await open({ directory: true, title: "Select Project Zomboid installation" });
    if (selected) {
      const path = selected as string;
      const valid = await invoke<boolean>("verify_game_path_cmd", { path });
      setLocalGamePath(path);
      setGameVerified(valid);
    }
  };

  const browseSteam = async () => {
    const selected = await open({ directory: true, title: "Select Steam installation folder" });
    if (selected) {
      const path = selected as string;
      try {
        const valid = await invoke<boolean>("verify_steam_path_cmd", { path });
        setLocalSteamPath(path);
        setSteamVerified(valid);
      } catch {
        setLocalSteamPath(path);
        setSteamVerified(false);
      }
    }
  };

  const browseWorkshop = async () => {
    const selected = await open({
      directory: true,
      title: "Select Workshop Content folder (steamapps/workshop/content/108600)",
    });
    if (selected) setLocalWorkshopPath(selected as string);
  };

  const browseZomboidDir = async () => {
    const selected = await open({ directory: true, title: "Select Zomboid user data folder" });
    if (selected) setLocalZomboidDir(selected as string);
  };

  const browseLocalMods = async () => {
    const selected = await open({ directory: true, title: "Select local mods folder (for auto-fix output)" });
    if (selected) setLocalLocalModsPath(selected as string);
  };

  const handleSave = async () => {
    if (!config || !activeProfile) return;
    setSaving(true);
    try {
      // Save paths to the active profile
      const updatedProfile = {
        ...activeProfile,
        gamePath: localGamePath,
        steamPath: localSteamPath,
        workshopPath: localWorkshopPath,
        zomboidUserDir: localZomboidDir,
        gameVersion: localGameVersion,
      };
      const updatedProfiles = profiles.map((p) =>
        p.id === activeProfile.id ? updatedProfile : p
      );
      useProfileStore.setState({ profiles: updatedProfiles });
      await invoke("update_profile_cmd", { profile: updatedProfile });

      // Also update global config as fallback defaults
      const updatedConfig: AppConfig = {
        ...config,
        gamePath: localGamePath,
        steamPath: localSteamPath,
        workshopPath: localWorkshopPath,
        localModsPath: localLocalModsPath,
        zomboidUserDir: localZomboidDir,
        gameVersion: localGameVersion,
        isFirstRun: false,
      };
      await saveConfig(updatedConfig);

      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-1">
      {/* Copy paths from another profile */}
      {otherProfiles.length > 0 && (
        <div className="flex items-center gap-3 py-2 mb-2 border-b border-border pb-4">
          <label className="w-36 text-sm text-muted-foreground shrink-0">Copy paths from</label>
          <select
            className="text-sm bg-muted border border-border rounded px-3 py-1.5"
            defaultValue=""
            onChange={(e) => {
              if (e.target.value) copyPathsFrom(e.target.value);
              e.target.value = "";
            }}
          >
            <option value="">Select a profile...</option>
            {otherProfiles.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>
      )}

      <PathRow
        label="Game path"
        value={localGamePath}
        globalValue={config?.gamePath}
        onBrowse={browseGame}
        onChange={(p) => setLocalGamePath(p || null)}
        onClear={() => setLocalGamePath(config?.gamePath ?? null)}
        verified={gameVerified}
      />
      <PathRow
        label="Steam path"
        value={localSteamPath}
        globalValue={config?.steamPath}
        onBrowse={browseSteam}
        onChange={(p) => setLocalSteamPath(p || null)}
        onClear={() => setLocalSteamPath(config?.steamPath ?? null)}
        verified={steamVerified}
      />
      <PathRow
        label="Workshop path"
        value={localWorkshopPath}
        globalValue={config?.workshopPath}
        onBrowse={browseWorkshop}
        onChange={(p) => setLocalWorkshopPath(p || null)}
        onClear={() => setLocalWorkshopPath(config?.workshopPath ?? null)}
      />
      <PathRow
        label="Zomboid user dir"
        value={localZomboidDir}
        globalValue={config?.zomboidUserDir}
        onBrowse={browseZomboidDir}
        onChange={(p) => setLocalZomboidDir(p || null)}
        onClear={() => setLocalZomboidDir(config?.zomboidUserDir ?? null)}
      />
      <PathRow
        label="Local mods path"
        value={localLocalModsPath}
        globalValue={config?.localModsPath}
        onBrowse={browseLocalMods}
        onChange={(p) => setLocalLocalModsPath(p || null)}
        onClear={() => setLocalLocalModsPath(config?.localModsPath ?? null)}
      />

      {/* Game version — editable with build number display */}
      <div className="flex items-center gap-3 py-2">
        <label className="w-36 text-sm text-muted-foreground shrink-0">Game version</label>
        <Input
          className="flex-1 font-mono min-w-0 max-w-48"
          value={localGameVersion ?? ""}
          placeholder="e.g. 42.15.2"
          onChange={(e) => setLocalGameVersion(e.target.value || null)}
        />
        {detectedBuild && !localGameVersion && (
          <button
            className="text-xs text-primary hover:underline shrink-0"
            onClick={() => setLocalGameVersion("42.15.2")}
          >
            Set to 42.15.2 (current)
          </button>
        )}
        {detectedBuild && (
          <span className="text-xs text-muted-foreground shrink-0">
            SVN Build {detectedBuild}
          </span>
        )}
        <span className="text-xs text-muted-foreground shrink-0 ml-auto">
          Used for mod version matching
        </span>
      </div>

      {activeProfile && (
        <p className="text-xs text-muted-foreground py-1">
          Paths marked with <span className="text-warning">*</span> are overridden for the <strong>{activeProfile.name}</strong> profile.
        </p>
      )}

      <div className="pt-4">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-4 py-2 bg-primary text-primary-foreground text-sm rounded font-medium hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {saving ? "Saving..." : saved ? "Saved ✓" : "Save"}
        </button>
      </div>
    </div>
  );
}
