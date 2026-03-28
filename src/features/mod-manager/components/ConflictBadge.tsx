import { cn } from "../../../shared/lib/utils";
import type { ModConflict } from "../../../shared/types/conflicts";

interface ConflictBadgeProps {
  conflicts: ModConflict[];
}

export function ConflictBadge({ conflicts }: ConflictBadgeProps) {
  if (conflicts.length === 0) return null;

  const hasError = conflicts.some((c) => c.severity === "error");
  const hasWarning = conflicts.some((c) => c.severity === "warning");

  if (!hasError && !hasWarning) return null;

  return (
    <span
      className={cn("text-xs shrink-0", hasError ? "text-destructive" : "text-warning")}
      title={`${conflicts.length} conflict${conflicts.length > 1 ? "s" : ""}`}
    >
      {hasError ? "\u26D4" : "\u26A0"}
    </span>
  );
}
