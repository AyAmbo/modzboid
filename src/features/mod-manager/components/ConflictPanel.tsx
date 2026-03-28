import { cn } from "../../../shared/lib/utils";
import type { ModConflict } from "../../../shared/types/conflicts";

interface ConflictPanelProps {
  conflicts: ModConflict[];
}

const severityIcon: Record<string, string> = {
  error: "\u26D4",
  warning: "\u26A0",
  info: "\u2139",
};

const typeLabel: Record<string, string> = {
  fileOverride: "File Override",
  scriptIdClash: "Script ID Clash",
  versionMismatch: "Version Mismatch",
  knownIncompat: "Known Incompatibility",
  functionOverride: "Function Override",
  eventCollision: "Event Hook Collision",
};

export function ConflictPanel({ conflicts }: ConflictPanelProps) {
  if (conflicts.length === 0) return null;

  return (
    <div className="mt-2">
      <span className="text-xs font-medium text-muted-foreground">
        Conflicts ({conflicts.length})
      </span>
      <div className="mt-1 space-y-1.5">
        {conflicts.map((c, i) => (
          <div
            key={i}
            className={cn(
              "text-xs px-2 py-1.5 rounded border",
              c.severity === "error"
                ? "border-destructive/30 bg-destructive/5"
                : c.severity === "warning"
                  ? "border-warning/30 bg-warning/5"
                  : "border-border bg-muted/50"
            )}
          >
            <div className="flex items-start gap-1.5">
              <span className="shrink-0">{severityIcon[c.severity] ?? ""}</span>
              <div className="min-w-0">
                <div className="font-medium">
                  {typeLabel[c.conflictType] ?? c.conflictType}
                  {c.filePath && <span className="font-mono ml-1 text-muted-foreground">{c.filePath}</span>}
                  {c.scriptId && <span className="font-mono ml-1 text-muted-foreground">{c.scriptId}</span>}
                </div>
                <div className="text-muted-foreground mt-0.5">{c.message}</div>
                {c.suggestion && <div className="text-muted-foreground/70 mt-0.5 italic">{c.suggestion}</div>}
                {c.isIntentional && (
                  <span className="inline-block mt-0.5 text-xs bg-muted px-1 rounded text-muted-foreground">Intentional</span>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
