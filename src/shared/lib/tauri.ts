import { convertFileSrc } from "@tauri-apps/api/core";

export function assetUrl(path: string | null): string | undefined {
  if (!path) return undefined;
  // Normalize Windows backslashes to forward slashes for URI compatibility
  const normalized = path.replace(/\\/g, "/");
  return convertFileSrc(normalized);
}
