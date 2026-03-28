import { memo, useMemo, useState } from "react";
import { cn } from "../../../shared/lib/utils";
import { assetUrl } from "../../../shared/lib/tauri";
import { useModManagerStore } from "../store";
import type { ModInfo } from "../../../shared/types/modTypes";
import type { LoadOrderIssue } from "../../../shared/types/validation";
import { ConflictBadge } from "./ConflictBadge";
import { useCompatStore } from "../../compatibility/compatStore";

interface ModCardProps {
  mod: ModInfo;
  isActive: boolean;
  showDragHandle?: boolean;
  dragHandleProps?: Record<string, unknown>;
  onToggle?: () => void;
  style?: React.CSSProperties;
}

const categoryIcons: Record<string, string> = {
  framework: "\u2699",
  map: "\uD83D\uDDFA",
  content: "\uD83D\uDCE6",
  overhaul: "\u2728",
};

function ModCardInner({
  mod,
  isActive,
  showDragHandle,
  onToggle,
  style,
}: ModCardProps) {
  const selectedModId = useModManagerStore((s) => s.selectedModId);
  const selectMod = useModManagerStore((s) => s.selectMod);
  const selectedModIds = useModManagerStore((s) => s.selectedModIds);
  const toggleModSelection = useModManagerStore((s) => s.toggleModSelection);
  const issues = useModManagerStore((s) => s.issues);
  const allMods = useModManagerStore((s) => s.allMods);

  const isSelected = selectedModIds.size > 0 ? selectedModIds.has(mod.id) : selectedModId === mod.id;
  const modIssues = issues.filter(
    (i: LoadOrderIssue) => i.modId === mod.id
  );
  const hasWarning = modIssues.length > 0;
  const hasError = modIssues.some((i: LoadOrderIssue) => i.severity === "error");
  const posterUrl = assetUrl(mod.posterPath);
  const [imgFailed, setImgFailed] = useState(false);
  const category = mod.detectedCategory ?? mod.category;
  const categoryIcon = category ? categoryIcons[category] : null;
  const conflictsByMod = useModManagerStore((s) => s.conflictsByMod);
  const modConflicts = conflictsByMod.get(mod.id) ?? [];
  const compatIssues = useCompatStore((s) => s.issuesByMod.get(mod.id) ?? -1);

  // Dependency status badge
  const depStatus = useMemo(() => {
    if (mod.requires.length === 0) return null;
    const modIdSet = new Set(allMods.map((m) => m.id));
    const installed = mod.requires.filter((depId) => modIdSet.has(depId));
    const missing = mod.requires.length - installed.length;
    if (missing === 0) {
      return { icon: "\u2713", color: "text-success", tooltip: `All ${mod.requires.length} deps installed: ${mod.requires.join(", ")}` };
    } else if (missing < mod.requires.length) {
      const missingIds = mod.requires.filter((depId) => !modIdSet.has(depId));
      return { icon: "!", color: "text-warning", tooltip: `Missing deps: ${missingIds.join(", ")}` };
    } else {
      return { icon: "\u2717", color: "text-destructive", tooltip: `Missing all deps: ${mod.requires.join(", ")}` };
    }
  }, [mod.requires, allMods]);

  return (
    <div
      className={cn(
        "flex items-center h-14 px-2 gap-2 select-none border-b border-border hover:bg-muted/50 transition-colors",
        showDragHandle === false && isActive ? "cursor-grab active:cursor-grabbing" : "cursor-pointer",
        isSelected && "bg-primary/10 border-l-2 border-l-primary"
      )}
      style={style}
      onClick={(e) => {
        if (e.ctrlKey || e.metaKey || e.shiftKey) {
          toggleModSelection(mod.id, true);
        } else {
          selectMod(mod.id);
        }
      }}
      onDoubleClick={onToggle}
    >
      {/* Toggle button — left side, before thumbnail */}
      {onToggle && (
        <button
          className={cn(
            "shrink-0 w-7 h-7 rounded flex items-center justify-center text-sm font-bold hover:bg-muted transition-colors",
            isActive
              ? "text-destructive hover:text-destructive hover:bg-destructive/10"
              : "text-success hover:text-success hover:bg-success/10"
          )}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            onToggle();
          }}
          title={isActive ? "Disable mod" : "Enable mod"}
        >
          {isActive ? "\u2212" : "+"}
        </button>
      )}

      {/* Thumbnail */}
      <div className="w-8 h-8 rounded shrink-0 overflow-hidden bg-muted flex items-center justify-center">
        {posterUrl && !imgFailed ? (
          <img
            src={posterUrl}
            alt={mod.name}
            className="w-full h-full object-cover"
            onError={() => setImgFailed(true)}
          />
        ) : (
          <span className="text-xs font-bold text-muted-foreground">
            {mod.name.charAt(0).toUpperCase()}
          </span>
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5">
          <span className="text-sm font-medium truncate">{mod.name}</span>
          {hasWarning && (
            <span
              className={cn(
                "text-xs shrink-0",
                hasError ? "text-destructive" : "text-warning"
              )}
              title={modIssues.map((i: LoadOrderIssue) => i.message).join("\n")}
            >
              {hasError ? "\u26D4" : "\u26A0"}
            </span>
          )}
          {categoryIcon && (
            <span className="text-xs text-muted-foreground shrink-0">
              {categoryIcon}
            </span>
          )}
          {depStatus && (
            <span
              className={cn("text-xs shrink-0 font-bold", depStatus.color)}
              title={depStatus.tooltip}
            >
              {depStatus.icon}
            </span>
          )}
          <ConflictBadge conflicts={modConflicts} />
          {compatIssues === 0 && (
            <span className="text-xs shrink-0 text-green-500" title="B42 compatible">&#10003;</span>
          )}
          {compatIssues > 0 && (
            <span className="text-xs shrink-0 text-yellow-500" title={`${compatIssues} compatibility issue${compatIssues !== 1 ? "s" : ""}`}>
              {compatIssues}&#9888;
            </span>
          )}
        </div>
        <div className="text-xs text-muted-foreground truncate">
          {mod.authors.join(", ") || "Unknown"}
          {mod.modVersion && (
            <span className="ml-1.5 text-muted-foreground/70">
              v{mod.modVersion}
            </span>
          )}
        </div>
      </div>

    </div>
  );
}

export const ModCard = memo(ModCardInner);
