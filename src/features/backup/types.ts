export interface BackupInfo {
  id: string;
  name: string;
  createdAt: string;
  sizeBytes: number;
  path: string;
  profileCount: number;
  hasServerConfigs: boolean;
}
