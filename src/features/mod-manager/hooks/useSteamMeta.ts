import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface WorkshopItemMeta {
  publishedFileId: string;
  title: string;
  description: string;
  previewUrl: string | null;
  tags: string[];
  timeCreated: number;
  timeUpdated: number;
  fileSize: number;
  subscriptions: number;
  favorited: number;
  views: number;
  creatorId: string;
  dependencies: string[];
  found: boolean;
}

// In-memory cache to avoid re-fetching during the session
const metaCache = new Map<string, WorkshopItemMeta>();
const pendingFetches = new Set<string>();

/**
 * Fetch Steam Workshop metadata for a single mod.
 * Uses in-memory caching — data persists for the session.
 */
export function useSteamMeta(workshopId: string | null) {
  const [meta, setMeta] = useState<WorkshopItemMeta | null>(
    workshopId ? metaCache.get(workshopId) ?? null : null
  );
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!workshopId) {
      setMeta(null);
      return;
    }

    // Check cache
    const cached = metaCache.get(workshopId);
    if (cached) {
      setMeta(cached);
      return;
    }

    // Don't double-fetch
    if (pendingFetches.has(workshopId)) return;

    setLoading(true);
    pendingFetches.add(workshopId);

    invoke<WorkshopItemMeta | null>("fetch_single_workshop_meta_cmd", {
      workshopId,
    })
      .then((result) => {
        if (result) {
          metaCache.set(workshopId, result);
          setMeta(result);
        }
      })
      .catch((err) => {
        console.error("Failed to fetch Workshop meta:", err);
      })
      .finally(() => {
        setLoading(false);
        pendingFetches.delete(workshopId);
      });
  }, [workshopId]);

  return { meta, loading };
}

/**
 * Batch-fetch Steam Workshop metadata for multiple mods.
 * Returns a map of workshopId -> metadata.
 */
export function useBatchSteamMeta(workshopIds: string[]) {
  const [metaMap, setMetaMap] = useState<Map<string, WorkshopItemMeta>>(
    new Map()
  );
  const [loading, setLoading] = useState(false);
  const fetchedRef = useRef<Set<string>>(new Set());

  const fetchMeta = useCallback(async () => {
    // Filter to only IDs we haven't fetched yet
    const needed = workshopIds.filter(
      (id) => id && !metaCache.has(id) && !fetchedRef.current.has(id)
    );

    if (needed.length === 0) {
      // All cached
      const cached = new Map<string, WorkshopItemMeta>();
      for (const id of workshopIds) {
        const m = metaCache.get(id);
        if (m) cached.set(id, m);
      }
      setMetaMap(cached);
      return;
    }

    setLoading(true);
    try {
      const results = await invoke<WorkshopItemMeta[]>(
        "fetch_workshop_meta_cmd",
        { workshopIds: needed }
      );

      for (const item of results) {
        if (item.found) {
          metaCache.set(item.publishedFileId, item);
        }
        fetchedRef.current.add(item.publishedFileId);
      }

      // Build full map from cache
      const fullMap = new Map<string, WorkshopItemMeta>();
      for (const id of workshopIds) {
        const m = metaCache.get(id);
        if (m) fullMap.set(id, m);
      }
      setMetaMap(fullMap);
    } catch (err) {
      console.error("Failed to batch fetch Workshop meta:", err);
    } finally {
      setLoading(false);
    }
  }, [workshopIds.join(",")]);

  useEffect(() => {
    if (workshopIds.length > 0) {
      fetchMeta();
    }
  }, [fetchMeta]);

  return { metaMap, loading, refetch: fetchMeta };
}

/** Format a Unix timestamp as a human-readable date. */
export function formatSteamDate(timestamp: number): string {
  if (!timestamp) return "Unknown";
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/** Format file size in bytes to human-readable. */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

/** Format subscriber count with K/M suffix. */
export function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}
