import { create } from "zustand";

interface CompatStore {
  /** Map of modId -> total issues (lua + scripts) */
  issuesByMod: Map<string, number>;
  setResults: (results: { modId: string; totalIssues: number; scriptIssues: number }[]) => void;
}

// Load persisted results on init
function loadPersistedIssues(): Map<string, number> {
  try {
    const raw = localStorage.getItem("modzboid-compat-results");
    if (!raw) return new Map();
    const reports = JSON.parse(raw) as { modId: string; totalIssues: number; scriptIssues: number }[];
    const map = new Map<string, number>();
    for (const r of reports) {
      map.set(r.modId, (r.totalIssues ?? 0) + (r.scriptIssues ?? 0));
    }
    return map;
  } catch {
    return new Map();
  }
}

export const useCompatStore = create<CompatStore>((set) => ({
  issuesByMod: loadPersistedIssues(),
  setResults: (results) => {
    const map = new Map<string, number>();
    for (const r of results) {
      map.set(r.modId, (r.totalIssues ?? 0) + (r.scriptIssues ?? 0));
    }
    set({ issuesByMod: map });
  },
}));
