import { useMemo, useState } from "react";
import { cn } from "../../../shared/lib/utils";
import { assetUrl } from "../../../shared/lib/tauri";
import { useModManagerStore, type SortField, type ColumnVisibility } from "../store";
import { ModContextMenu } from "./ModContextMenu";
import type { ModInfo } from "../../../shared/types/modTypes";

interface ModTableProps {
  mods: ModInfo[];
  isActive: boolean;
  onToggle?: (mod: ModInfo) => void;
}

function formatBytes(bytes: number | null | undefined): string {
  if (!bytes) return "-";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string | null | undefined): string {
  if (!iso) return "-";
  try {
    return new Date(iso).toLocaleDateString();
  } catch {
    return "-";
  }
}

type Column = {
  key: keyof ColumnVisibility | "name";
  label: string;
  sortField?: SortField;
  width: string;
  render: (mod: ModInfo) => React.ReactNode;
};

const ALL_COLUMNS: Column[] = [
  {
    key: "name",
    label: "Name",
    sortField: "name",
    width: "flex-1 min-w-[150px]",
    render: (mod) => (
      <div className="flex items-center gap-2 min-w-0">
        <ModThumb mod={mod} />
        <span className="truncate font-medium">{mod.name}</span>
      </div>
    ),
  },
  {
    key: "version",
    label: "Version",
    sortField: "version",
    width: "w-20",
    render: (mod) => <span className="text-muted-foreground">{mod.modVersion ?? "-"}</span>,
  },
  {
    key: "author",
    label: "Author",
    sortField: "author",
    width: "w-32",
    render: (mod) => <span className="truncate text-muted-foreground">{mod.authors[0] ?? "-"}</span>,
  },
  {
    key: "size",
    label: "Size",
    sortField: "size",
    width: "w-20",
    render: (mod) => <span className="text-muted-foreground font-mono text-xs">{formatBytes(mod.sizeBytes)}</span>,
  },
  {
    key: "source",
    label: "Source",
    sortField: "source",
    width: "w-20",
    render: (mod) => (
      <span className={cn("text-xs px-1.5 py-0.5 rounded", mod.source === "workshop" ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground")}>
        {mod.source === "workshop" ? "WS" : "Local"}
      </span>
    ),
  },
  {
    key: "category",
    label: "Category",
    sortField: "category",
    width: "w-24",
    render: (mod) => {
      const cat = mod.detectedCategory ?? mod.category;
      return <span className="text-xs text-muted-foreground">{cat ?? "-"}</span>;
    },
  },
  {
    key: "dependencies",
    label: "Deps",
    width: "w-12",
    render: (mod) => <span className="text-xs text-muted-foreground">{mod.requires.length || "-"}</span>,
  },
  {
    key: "workshopId",
    label: "Workshop ID",
    sortField: "workshopId",
    width: "w-28",
    render: (mod) => <span className="text-xs text-muted-foreground font-mono">{mod.workshopId ?? "-"}</span>,
  },
  {
    key: "lastModified",
    label: "Modified",
    sortField: "lastModified",
    width: "w-24",
    render: (mod) => <span className="text-xs text-muted-foreground">{formatDate(mod.lastModified)}</span>,
  },
];

function ModThumb({ mod }: { mod: ModInfo }) {
  const url = assetUrl(mod.posterPath);
  const [failed, setFailed] = useState(false);
  return (
    <div className="w-6 h-6 rounded shrink-0 overflow-hidden bg-muted flex items-center justify-center">
      {url && !failed ? (
        <img src={url} alt="" className="w-full h-full object-cover" onError={() => setFailed(true)} />
      ) : (
        <span className="text-[10px] font-bold text-muted-foreground">{mod.name.charAt(0).toUpperCase()}</span>
      )}
    </div>
  );
}

export function ModTable({ mods, isActive, onToggle }: ModTableProps) {
  const selectedModId = useModManagerStore((s) => s.selectedModId);
  const selectMod = useModManagerStore((s) => s.selectMod);
  const columns = useModManagerStore((s) => s.columns);
  const toggleSort = useModManagerStore((s) => s.toggleSort);
  const sortField = useModManagerStore((s) => s.sortField);
  const sortDirection = useModManagerStore((s) => s.sortDirection);

  const visibleColumns = useMemo(() => {
    return ALL_COLUMNS.filter((col) => {
      if (col.key === "name") return true;
      return columns[col.key as keyof ColumnVisibility];
    });
  }, [columns]);

  return (
    <div className="flex flex-col h-full text-xs">
      {/* Header row */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-border bg-muted/50 sticky top-0 z-10">
        {onToggle && <div className="w-7 shrink-0" />}
        {visibleColumns.map((col) => (
          <button
            key={col.key}
            onClick={() => col.sortField && toggleSort(col.sortField)}
            className={cn(
              "text-left text-muted-foreground hover:text-foreground transition-colors truncate",
              col.width,
              col.sortField && "cursor-pointer"
            )}
          >
            {col.label}
            {col.sortField === sortField && (
              <span className="ml-0.5">{sortDirection === "asc" ? "\u2191" : "\u2193"}</span>
            )}
          </button>
        ))}
      </div>

      {/* Rows */}
      <div className="flex-1 overflow-auto">
        {mods.map((mod) => (
          <ModContextMenu key={mod.id} mod={mod} isActive={isActive} onToggle={onToggle ? () => onToggle(mod) : undefined}>
            <div
              className={cn(
                "flex items-center gap-1 px-2 py-1 border-b border-border hover:bg-muted/50 cursor-pointer transition-colors",
                selectedModId === mod.id && "bg-primary/10 border-l-2 border-l-primary"
              )}
              onClick={() => selectMod(mod.id)}
              onDoubleClick={() => onToggle?.(mod)}
            >
              {onToggle && (
                <button
                  className={cn(
                    "w-7 shrink-0 text-center text-sm",
                    isActive ? "text-destructive hover:text-destructive" : "text-success hover:text-success"
                  )}
                  onClick={(e) => { e.stopPropagation(); onToggle(mod); }}
                >
                  {isActive ? "\u2212" : "+"}
                </button>
              )}
              {visibleColumns.map((col) => (
                <div key={col.key} className={cn("truncate", col.width)}>
                  {col.render(mod)}
                </div>
              ))}
            </div>
          </ModContextMenu>
        ))}
      </div>
    </div>
  );
}
