export type SettingType = 'bool' | 'int' | 'float' | 'string';
export type SandboxSettingType = 'int' | 'float' | 'bool' | 'string' | 'enum';

export interface ServerSetting {
  key: string;
  value: string;
  description: string | null;
  settingType: SettingType;
  min: number | null;
  max: number | null;
  defaultValue: string | null;
  category: string;
}

export interface ServerConfig {
  name: string;
  path: string;
  settings: ServerSetting[];
}

export interface ServerConfigInfo {
  name: string;
  path: string;
}

export interface ServerSettingUpdate {
  key: string;
  value: string;
}

export interface EnumOption {
  value: number;
  label: string;
}

export interface SandboxSetting {
  key: string;
  value: number | boolean | string;
  description: string | null;
  settingType: SandboxSettingType;
  min: number | null;
  max: number | null;
  defaultValue: string | null;
  enumOptions: EnumOption[];
}

export interface SandboxCategory {
  name: string;
  settings: SandboxSetting[];
}

export interface SandboxVarsConfig {
  name: string;
  path: string;
  topLevel: SandboxSetting[];
  categories: SandboxCategory[];
}

export interface SandboxSettingUpdate {
  category: string | null;
  key: string;
  value: string;
}
