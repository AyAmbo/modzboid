export interface ModListExport {
  format: string;
  version: number;
  name: string;
  exportedAt: string;
  gameVersion: string | null;
  modCount: number;
  mods: ModListEntry[];
}

export interface ModListEntry {
  id: string;
  name: string;
  workshopId: string | null;
  authors: string[];
  url: string | null;
}

export interface ImportPreview {
  total: number;
  found: string[];
  missing: MissingMod[];
  detectedFormat: string;
}

export interface MissingMod {
  id: string;
  name: string | null;
  workshopId: string | null;
}
