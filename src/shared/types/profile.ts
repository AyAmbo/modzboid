export type ProfileType = 'singleplayer' | 'server';

export interface Profile {
  id: string;
  name: string;
  type: ProfileType;
  loadOrder: string[];
  serverConfigPath: string | null;
  createdAt: string;
  updatedAt: string;
  isDefault: boolean;
  versionOverrides: Record<string, string>;
  // Per-profile path overrides (null = use global config)
  gamePath: string | null;
  steamPath: string | null;
  workshopPath: string | null;
  localModsPath: string | null;
  zomboidUserDir: string | null;
  gameVersion: string | null;
}
