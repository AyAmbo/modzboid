import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "../../../shared/lib/utils";
import { useModManagerStore } from "../store";
import { toast } from "../../../shared/components/ui/toaster";
import { Button } from "../../../shared/components/ui/button";

interface InspectionCheck {
  name: string;
  passed: boolean;
  severity: string;
  message: string;
}

interface InspectionReport {
  modId: string;
  modName: string;
  checks: InspectionCheck[];
  score: number;
  luaFileCount: number;
  scriptFileCount: number;
  textureCount: number;
  totalFiles: number;
}

interface LuaIssue {
  file: string;
  line: number | null;
  severity: string;
  category: string;
  message: string;
  suggestion: string | null;
}

interface LuaCheckReport {
  filesChecked: number;
  filesWithIssues: number;
  issues: LuaIssue[];
  summary: { errors: number; warnings: number; info: number };
}

export function InspectorPanel() {
  const selectedModId = useModManagerStore((s) => s.selectedModId);
  const [report, setReport] = useState<InspectionReport | null>(null);
  const [luaReport, setLuaReport] = useState<LuaCheckReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [luaLoading, setLuaLoading] = useState(false);

  useEffect(() => {
    if (!selectedModId) {
      setReport(null);
      setLuaReport(null);
      return;
    }
    setLoading(true);
    setLuaReport(null);
    invoke<InspectionReport>("inspect_mod_cmd", { modId: selectedModId })
      .then(setReport)
      .catch((err) => {
        console.error("Inspection failed:", err);
        toast({ title: "Error", description: "Mod inspection failed", variant: "destructive" });
        setReport(null);
      })
      .finally(() => setLoading(false));
  }, [selectedModId]);

  const runLuaCheck = useCallback(() => {
    if (!selectedModId) return;
    setLuaLoading(true);
    invoke<LuaCheckReport>("check_mod_lua_cmd", { modId: selectedModId })
      .then(setLuaReport)
      .catch((err) => {
        console.error("Lua check failed:", err);
        toast({ title: "Error", description: "Lua check failed", variant: "destructive" });
      })
      .finally(() => setLuaLoading(false));
  }, [selectedModId]);

  if (!selectedModId) {
    return (
      <div className="p-3 text-sm text-muted-foreground">
        Select a mod to inspect.
      </div>
    );
  }

  if (loading) {
    return <div className="p-3 text-sm text-muted-foreground">Inspecting...</div>;
  }

  if (!report) return null;

  const scoreColor =
    report.score >= 80 ? "text-green-500" :
    report.score >= 50 ? "text-yellow-500" : "text-red-500";

  const severityIcon: Record<string, string> = {
    error: "\u2717",
    warning: "\u26A0",
    info: "\u2713",
  };

  const categoryLabel: Record<string, string> = {
    syntax: "Syntax",
    encoding: "Encoding",
    deprecated: "Deprecated API",
    compat: "Compatibility",
    quality: "Quality",
  };

  return (
    <div className="p-3 space-y-3">
      {/* Score */}
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Quality Score</span>
        <span className={cn("text-2xl font-bold", scoreColor)}>
          {report.score}
        </span>
      </div>

      {/* File stats */}
      <div className="grid grid-cols-4 gap-2 text-center">
        <div className="bg-muted rounded p-2">
          <div className="text-lg font-bold">{report.totalFiles}</div>
          <div className="text-xs text-muted-foreground">Files</div>
        </div>
        <div className="bg-muted rounded p-2">
          <div className="text-lg font-bold">{report.luaFileCount}</div>
          <div className="text-xs text-muted-foreground">Lua</div>
        </div>
        <div className="bg-muted rounded p-2">
          <div className="text-lg font-bold">{report.scriptFileCount}</div>
          <div className="text-xs text-muted-foreground">Scripts</div>
        </div>
        <div className="bg-muted rounded p-2">
          <div className="text-lg font-bold">{report.textureCount}</div>
          <div className="text-xs text-muted-foreground">Textures</div>
        </div>
      </div>

      {/* Checks */}
      <div className="space-y-1">
        {report.checks.map((check, i) => (
          <div
            key={i}
            className={cn(
              "flex items-start gap-2 text-xs px-2 py-1.5 rounded",
              !check.passed && check.severity === "error" && "bg-destructive/5",
              !check.passed && check.severity === "warning" && "bg-warning/5"
            )}
          >
            <span
              className={cn(
                "shrink-0 mt-0.5",
                check.passed ? "text-green-500" :
                check.severity === "error" ? "text-destructive" : "text-warning"
              )}
            >
              {check.passed
                ? severityIcon["info"]
                : severityIcon[check.severity] ?? severityIcon["warning"]}
            </span>
            <div className="min-w-0">
              <div className="font-medium">{check.name}</div>
              <div className="text-muted-foreground">{check.message}</div>
            </div>
          </div>
        ))}
      </div>

      {/* Lua Check */}
      <div className="border-t border-border pt-3">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">Lua Analysis</span>
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={runLuaCheck}
            disabled={luaLoading}
          >
            {luaLoading ? "Checking..." : luaReport ? "Re-check" : "Run Check"}
          </Button>
        </div>

        {luaReport && (
          <div className="space-y-2">
            {/* Summary */}
            <div className="flex gap-3 text-xs">
              <span className="text-muted-foreground">
                {luaReport.filesChecked} files checked
              </span>
              {luaReport.summary.errors > 0 && (
                <span className="text-destructive font-medium">
                  {luaReport.summary.errors} error{luaReport.summary.errors > 1 ? "s" : ""}
                </span>
              )}
              {luaReport.summary.warnings > 0 && (
                <span className="text-warning font-medium">
                  {luaReport.summary.warnings} warning{luaReport.summary.warnings > 1 ? "s" : ""}
                </span>
              )}
              {luaReport.summary.errors === 0 && luaReport.summary.warnings === 0 && (
                <span className="text-green-500 font-medium">All clear</span>
              )}
            </div>

            {/* Issues */}
            {luaReport.issues.length > 0 && (
              <div className="space-y-1 max-h-48 overflow-auto">
                {luaReport.issues.map((issue, i) => (
                  <div
                    key={i}
                    className={cn(
                      "text-xs px-2 py-1.5 rounded",
                      issue.severity === "error" && "bg-destructive/5",
                      issue.severity === "warning" && "bg-warning/5",
                      issue.severity === "info" && "bg-muted"
                    )}
                  >
                    <div className="flex items-center gap-2">
                      <span className={cn(
                        "shrink-0",
                        issue.severity === "error" ? "text-destructive" :
                        issue.severity === "warning" ? "text-warning" : "text-muted-foreground"
                      )}>
                        {severityIcon[issue.severity] ?? severityIcon["warning"]}
                      </span>
                      <span className="font-medium truncate">{issue.file}</span>
                      {issue.line && (
                        <span className="text-muted-foreground shrink-0">:{issue.line}</span>
                      )}
                      <span className="text-muted-foreground/60 shrink-0 ml-auto">
                        {categoryLabel[issue.category] ?? issue.category}
                      </span>
                    </div>
                    <div className="text-muted-foreground ml-5 mt-0.5">{issue.message}</div>
                    {issue.suggestion && (
                      <div className="text-primary/80 ml-5 mt-0.5">{issue.suggestion}</div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
