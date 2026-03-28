export type ModSource = 'workshop' | 'local';

export type ModCategory = 'framework' | 'map' | 'content' | 'overhaul';

export interface ModInfo {
  id: string;
  rawId: string;
  workshopId: string | null;
  name: string;
  description: string;
  authors: string[];
  url: string | null;
  modVersion: string | null;
  posterPath: string | null;
  iconPath: string | null;
  versionMin: string | null;
  versionMax: string | null;
  versionFolders: string[];
  activeVersionFolder: string | null;
  requires: string[];
  pack: string | null;
  tileDef: string[];
  category: string | null;
  source: ModSource;
  sourcePath: string;
  modInfoPath: string;
  sizeBytes: number | null;
  lastModified: string;
  detectedCategory: ModCategory | null;
}
