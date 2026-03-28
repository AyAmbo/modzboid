export interface ExtensionInfo {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  extensionType: string; // "rule-pack" | "theme"
  enabled: boolean;
  path: string;
}

export interface Replacement {
  outdatedModId: string;
  outdatedModName: string | null;
  replacementModId: string;
  replacementModName: string | null;
  replacementWorkshopId: string | null;
  reason: string;
}
