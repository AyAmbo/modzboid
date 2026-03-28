import { create } from "zustand";
import type { ModInfo, ModCategory } from "../../shared/types/modTypes";
import type { LoadOrderIssue } from "../../shared/types/validation";
import type { ModConflict } from "../../shared/types/conflicts";

export type SortField = "name" | "version" | "author" | "size" | "lastModified" | "source" | "workshopId" | "category" | "id";
export type SortDirection = "asc" | "desc";
export type ViewMode = "card" | "table";

export interface ColumnVisibility {
  version: boolean;
  author: boolean;
  size: boolean;
  source: boolean;
  category: boolean;
  dependencies: boolean;
  workshopId: boolean;
  lastModified: boolean;
}

const DEFAULT_COLUMNS: ColumnVisibility = {
  version: true,
  author: true,
  size: false,
  source: true,
  category: true,
  dependencies: true,
  workshopId: false,
  lastModified: false,
};

interface ModManagerStore {
  allMods: ModInfo[];
  selectedModId: string | null;
  selectedModIds: Set<string>;
  enabledSearch: string;
  availableSearch: string;
  categoryFilter: ModCategory | null;
  issues: LoadOrderIssue[];
  conflictsByMod: Map<string, ModConflict[]>;
  isLoading: boolean;
  sortField: SortField;
  sortDirection: SortDirection;
  viewMode: ViewMode;
  columns: ColumnVisibility;

  setMods: (mods: ModInfo[]) => void;
  selectMod: (id: string | null) => void;
  toggleModSelection: (id: string, multi: boolean) => void;
  clearSelection: () => void;
  selectAllInList: (ids: string[]) => void;
  setEnabledSearch: (q: string) => void;
  setAvailableSearch: (q: string) => void;
  setCategoryFilter: (cat: ModCategory | null) => void;
  setIssues: (issues: LoadOrderIssue[]) => void;
  setConflicts: (conflicts: ModConflict[]) => void;
  setIsLoading: (loading: boolean) => void;
  setSortField: (field: SortField) => void;
  setSortDirection: (dir: SortDirection) => void;
  toggleSort: (field: SortField) => void;
  setViewMode: (mode: ViewMode) => void;
  toggleColumn: (col: keyof ColumnVisibility) => void;
}

export const useModManagerStore = create<ModManagerStore>((set, get) => ({
  allMods: [],
  selectedModId: null,
  selectedModIds: new Set(),
  enabledSearch: "",
  availableSearch: "",
  categoryFilter: null,
  issues: [],
  conflictsByMod: new Map(),
  isLoading: false,
  sortField: "name",
  sortDirection: "asc",
  viewMode: (localStorage.getItem("modzboid-view-mode") as ViewMode) || "card",
  columns: (() => {
    try {
      const saved = localStorage.getItem("modzboid-columns");
      return saved ? { ...DEFAULT_COLUMNS, ...JSON.parse(saved) } : DEFAULT_COLUMNS;
    } catch {
      return DEFAULT_COLUMNS;
    }
  })(),

  setMods: (allMods) => set({ allMods }),
  selectMod: (selectedModId) => set({ selectedModId, selectedModIds: new Set() }),
  toggleModSelection: (id, multi) => {
    if (multi) {
      const prev = new Set(get().selectedModIds);
      if (prev.has(id)) {
        prev.delete(id);
      } else {
        prev.add(id);
      }
      set({ selectedModIds: prev, selectedModId: id });
    } else {
      set({ selectedModIds: new Set([id]), selectedModId: id });
    }
  },
  clearSelection: () => set({ selectedModIds: new Set(), selectedModId: null }),
  selectAllInList: (ids) => set({ selectedModIds: new Set(ids) }),
  setEnabledSearch: (enabledSearch) => set({ enabledSearch }),
  setAvailableSearch: (availableSearch) => set({ availableSearch }),
  setCategoryFilter: (categoryFilter) => set({ categoryFilter }),
  setIssues: (issues) => set({ issues }),
  setIsLoading: (isLoading) => set({ isLoading }),
  setConflicts: (conflicts) => {
    const byMod = new Map<string, ModConflict[]>();
    for (const c of conflicts) {
      for (const modId of c.modIds) {
        const existing = byMod.get(modId) || [];
        existing.push(c);
        byMod.set(modId, existing);
      }
    }
    set({ conflictsByMod: byMod });
  },
  setSortField: (sortField) => set({ sortField }),
  setSortDirection: (sortDirection) => set({ sortDirection }),
  toggleSort: (field) => {
    const { sortField, sortDirection } = get();
    if (sortField === field) {
      set({ sortDirection: sortDirection === "asc" ? "desc" : "asc" });
    } else {
      set({ sortField: field, sortDirection: "asc" });
    }
  },
  setViewMode: (viewMode) => {
    localStorage.setItem("modzboid-view-mode", viewMode);
    set({ viewMode });
  },
  toggleColumn: (col) => {
    const columns = { ...get().columns, [col]: !get().columns[col] };
    localStorage.setItem("modzboid-columns", JSON.stringify(columns));
    set({ columns });
  },
}));
