export interface AppConfig {
  gamePath: string | null;
  steamPath: string | null;
  workshopPath: string | null;
  localModsPath: string | null;
  zomboidUserDir: string | null;
  gameVersion: string | null;
  isFirstRun: boolean;
  theme: string;
  locale: string;
  checkUpdates: boolean;
  uiScale: number;
  fontSize: number;
}
