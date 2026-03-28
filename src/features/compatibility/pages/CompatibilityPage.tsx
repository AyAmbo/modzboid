import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../../../shared/components/ui/button";
import { Input } from "../../../shared/components/ui/input";
import { toast } from "../../../shared/components/ui/toaster";
import { cn } from "../../../shared/lib/utils";
import { useCompatStore } from "../compatStore";
import { useActiveProfile } from "../../profiles/store";

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

interface MigrationIssue {
  file: string;
  line: number;
  column: number;
  oldApi: string;
  replacement: string | null;
  autoFixable: boolean;
  severity: string;
  category: string;
  message: string;
}

interface CompatReport {
  modId: string;
  modName: string;
  filesScanned: number;
  filesWithIssues: number;
  totalIssues: number;
  autoFixable: number;
  needsReview: number;
  issues: MigrationIssue[];
  detectedVersion: string;
  rulesUsed: string;
  activeFolder: string | null;
  scriptIssues: number;
  scriptReport: {
    modId: string;
    modName: string;
    filesScanned: number;
    totalIssues: number;
    issues: {
      file: string;
      line: number;
      blockType: string;
      property: string;
      value: string;
      message: string;
      suggestion: string;
      severity: string;
    }[];
  } | null;
  missingRefs: {
    modId: string;
    modName: string;
    file: string;
    line: number;
    context: string;
    blockName: string;
    property: string;
    referencedItem: string;
    severity: string;
  }[];
  missingRefCount: number;
}

interface MigrationVersionsData {
  versions: { from: string; to: string; ruleCount: number }[];
  latestVersion: string;
}

interface ModpackFixReport {
  outputPath: string;
  modId: string;
  modsPatched: number;
  modsSkipped: number;
  totalFixes: number;
  totalTodos: number;
  totalTranslations: number;
  manualReviewIssues: number;
}

const STORAGE_KEY = "modzboid-compat-results";

function loadPersistedResults(): CompatReport[] | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch { return null; }
}

function persistResults(reports: CompatReport[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(reports));
  } catch { /* ignore quota errors */ }
}

/* ------------------------------------------------------------------ */
/* Main Page                                                           */
/* ------------------------------------------------------------------ */

export default function CompatibilityPage() {
  const [reports, setReports] = useState<CompatReport[] | null>(loadPersistedResults);
  const [expandedMod, setExpandedMod] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [noExtension, setNoExtension] = useState(false);
  const [latestVersion, setLatestVersion] = useState("");
  const [filter, setFilter] = useState<"all" | "issues" | "ok">("all");
  const [searchText, setSearchText] = useState("");
  const [sortBy, setSortBy] = useState<"mod" | "version" | "files" | "lua" | "scripts" | "refs" | "status">("status");
  const [sortAsc, setSortAsc] = useState(false);
  const [fullScan, setFullScan] = useState(true);
  const [lastScanTime, setLastScanTime] = useState<string | null>(
    localStorage.getItem("modzboid-compat-time")
  );
  const [generatingPack, setGeneratingPack] = useState(false);
  const [packNameDialog, setPackNameDialog] = useState(false);
  const [packName, setPackName] = useState("B42_Modpack_Fixes");
  const activeProfile = useActiveProfile();

  useEffect(() => {
    invoke<MigrationVersionsData>("list_migration_versions_cmd")
      .then((data) => {
        if (data.versions.length === 0) setNoExtension(true);
        else setLatestVersion(data.latestVersion);
      })
      .catch(() => setNoExtension(true));
  }, []);

  const handleScan = useCallback(async () => {
    setScanning(true);
    setExpandedMod(null);
    try {
      const result = await invoke<CompatReport[]>("scan_all_mods_compat_cmd", { forceFull: fullScan });
      setReports(result);
      persistResults(result);
      useCompatStore.getState().setResults(result);
      const now = new Date().toLocaleString();
      setLastScanTime(now);
      localStorage.setItem("modzboid-compat-time", now);
    } catch (err) {
      console.error("Scan failed:", err);
    } finally {
      setScanning(false);
    }
  }, []);

  const handleGenerateModpackFixes = useCallback(async () => {
    if (!activeProfile?.loadOrder?.length) {
      toast({ title: "No mods", description: "Active profile has no mods in load order", variant: "destructive" });
      return;
    }
    setGeneratingPack(true);
    setPackNameDialog(false);
    try {
      const result = await invoke<ModpackFixReport>("create_modpack_fixes_cmd", {
        loadOrder: activeProfile.loadOrder,
        packName,
      });
      toast({
        title: "Modpack Fixes Created",
        description: `${result.modsPatched} mods patched, ${result.totalFixes} fixes. Saved to: ${result.outputPath}`,
      });
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      toast({ title: "Generation Failed", description: msg, variant: "destructive" });
    } finally {
      setGeneratingPack(false);
    }
  }, [activeProfile, packName]);

  // Filter + search + deduplicate by modId
  const filtered = (() => {
    if (!reports) return null;
    const seen = new Set<string>();
    return reports.filter((r) => {
      // Deduplicate: keep first occurrence of each modId
      if (seen.has(r.modId)) return false;
      seen.add(r.modId);
      const allIssues = r.totalIssues + r.scriptIssues + (r.missingRefCount || 0);
      if (filter === "issues" && allIssues === 0) return false;
      if (filter === "ok" && allIssues > 0) return false;
      if (searchText) {
        const q = searchText.toLowerCase();
        return r.modName.toLowerCase().includes(q) || r.modId.toLowerCase().includes(q);
      }
      return true;
    });
  })();

  // Sort filtered results (create new array to avoid mutating state)
  const sorted = filtered ? [...filtered].sort((a, b) => {
      let cmp = 0;
      switch (sortBy) {
        case "mod": cmp = a.modName.localeCompare(b.modName); break;
        case "version": cmp = a.detectedVersion.localeCompare(b.detectedVersion); break;
        case "files": cmp = a.filesScanned - b.filesScanned; break;
        case "lua": cmp = a.totalIssues - b.totalIssues; break;
        case "scripts": cmp = a.scriptIssues - b.scriptIssues; break;
        case "refs": cmp = (a.missingRefCount || 0) - (b.missingRefCount || 0); break;
        case "status": cmp = (a.totalIssues + a.scriptIssues + (a.missingRefCount || 0)) - (b.totalIssues + b.scriptIssues + (b.missingRefCount || 0)); break;
      }
      return sortAsc ? cmp : -cmp;
  }) : null;

  const handleSort = (col: typeof sortBy) => {
    if (sortBy === col) setSortAsc(!sortAsc);
    else { setSortBy(col); setSortAsc(false); }
  };

  const totalMods = reports?.length ?? 0;
  const modsOk = reports?.filter((r) => r.totalIssues + r.scriptIssues + (r.missingRefCount || 0) === 0).length ?? 0;
  const modsWithIssues = reports?.filter((r) => r.totalIssues + r.scriptIssues + (r.missingRefCount || 0) > 0).length ?? 0;
  const totalIssues = reports?.reduce((s, r) => s + r.totalIssues + r.scriptIssues + (r.missingRefCount || 0), 0) ?? 0;
  const totalMissingRefs = reports?.reduce((s, r) => s + (r.missingRefCount || 0), 0) ?? 0;

  return (
    <div data-testid="page-compatibility" className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 pt-5 pb-4 border-b border-border">
        <h1 className="text-xl font-semibold">Mod Compatibility</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Scan mods for deprecated APIs and compatibility issues.
          {latestVersion && <> Target: <strong>B{latestVersion}</strong></>}
        </p>
      </div>

      {noExtension ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="max-w-md text-center space-y-4">
            <p className="text-sm text-muted-foreground">
              No migration rules extension installed.
            </p>
            <div className="border border-border rounded-lg bg-card p-4 text-left space-y-2">
              <h3 className="text-sm font-medium">How to install</h3>
              <ol className="text-xs text-muted-foreground space-y-1 list-decimal list-inside">
                <li>Go to the <span className="font-medium text-foreground">Extensions</span> tab</li>
                <li>Click <span className="font-medium text-foreground">Install Extension</span></li>
                <li>Select the <span className="font-mono bg-muted px-1 rounded">migration-extension</span> folder</li>
              </ol>
            </div>
          </div>
        </div>
      ) : (
        <>
          {/* Toolbar */}
          <div className="px-6 py-3 border-b border-border flex items-center gap-3 flex-wrap">
            <Button onClick={handleScan} disabled={scanning}>
              {scanning ? "Scanning..." : "Scan All Mods"}
            </Button>
            <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none">
              <input
                type="checkbox"
                checked={fullScan}
                onChange={(e) => setFullScan(e.target.checked)}
                className="rounded border-border"
              />
              Full scan (all rules from B41)
            </label>
            {reports && modsWithIssues > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPackNameDialog(true)}
                disabled={generatingPack || !activeProfile}
              >
                {generatingPack ? "Generating..." : "Generate Modpack Fixes"}
              </Button>
            )}
            {lastScanTime && (
              <span className="text-xs text-muted-foreground">
                Last scan: {lastScanTime}
              </span>
            )}

            {reports && (
              <>
                <div className="ml-auto" />
                <input
                  type="text"
                  placeholder="Filter mods..."
                  value={searchText}
                  onChange={(e) => setSearchText(e.target.value)}
                  className="px-2 py-1 text-xs bg-background border border-border rounded w-40 focus:outline-none focus:ring-1 focus:ring-primary"
                />
                <div className="flex gap-1 text-xs">
                  {(["all", "issues", "ok"] as const).map((f) => (
                    <button
                      key={f}
                      onClick={() => setFilter(f)}
                      className={cn(
                        "px-2 py-1 rounded border capitalize",
                        filter === f
                          ? "bg-primary text-primary-foreground border-primary"
                          : "border-border text-muted-foreground hover:bg-muted"
                      )}
                    >
                      {f === "all" ? `All (${totalMods})` : f === "issues" ? `Issues (${modsWithIssues})` : `OK (${modsOk})`}
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>

          {/* Summary bar */}
          {reports && (
            <div className="px-6 py-2 border-b border-border flex gap-6 text-xs text-muted-foreground bg-muted/30">
              <span><span className="text-foreground font-medium">{totalMods}</span> mods scanned</span>
              <span className="text-green-500"><span className="font-medium">{modsOk}</span> compatible</span>
              {modsWithIssues > 0 && (
                <span className="text-yellow-500"><span className="font-medium">{modsWithIssues}</span> with issues</span>
              )}
              {totalMissingRefs > 0 && (
                <span className="text-red-500"><span className="font-medium">{totalMissingRefs}</span> missing refs (crash risk)</span>
              )}
              {totalIssues > 0 && (
                <span className="text-red-500"><span className="font-medium">{totalIssues}</span> total issues</span>
              )}
            </div>
          )}

          {/* Results */}
          <div className="flex-1 overflow-auto">
            {!reports ? (
              <div className="flex items-center justify-center h-full text-sm text-muted-foreground">
                Click "Scan All Mods" to check compatibility.
              </div>
            ) : sorted && sorted.length > 0 ? (
              <table className="w-full text-sm">
                <thead className="sticky top-0 bg-card z-10">
                  <tr className="border-b border-border">
                    <th className="text-left px-4 py-2 font-medium text-xs cursor-pointer hover:text-foreground" onClick={() => handleSort("mod")}>Mod {sortBy === "mod" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-24 cursor-pointer hover:text-foreground" onClick={() => handleSort("version")}>Version {sortBy === "version" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-20">Rules</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-16 cursor-pointer hover:text-foreground" onClick={() => handleSort("files")}>Files {sortBy === "files" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-16 cursor-pointer hover:text-foreground" onClick={() => handleSort("lua")}>Lua {sortBy === "lua" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-16 cursor-pointer hover:text-foreground" onClick={() => handleSort("scripts")}>Scripts {sortBy === "scripts" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-16 cursor-pointer hover:text-foreground" onClick={() => handleSort("refs")}>Refs {sortBy === "refs" ? (sortAsc ? "▲" : "▼") : ""}</th>
                    <th className="text-center px-3 py-2 font-medium text-xs w-24 cursor-pointer hover:text-foreground" onClick={() => handleSort("status")}>Status {sortBy === "status" ? (sortAsc ? "▲" : "▼") : ""}</th>
                  </tr>
                </thead>
                <tbody>
                  {sorted.map((r) => (
                    <ModRow
                      key={r.modId}
                      report={r}
                      expanded={expandedMod === r.modId}
                      onToggle={() => setExpandedMod(expandedMod === r.modId ? null : r.modId)}
                    />
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
                No mods match the current filter.
              </div>
            )}
          </div>
        </>
      )}

      {/* Pack name dialog */}
      {packNameDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setPackNameDialog(false)}>
          <div className="bg-card border border-border rounded-lg shadow-lg p-6 max-w-md w-full mx-4" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-sm font-semibold mb-1">Generate Modpack Fixes</h3>
            <p className="text-xs text-muted-foreground mb-4">
              This will create a single fix mod containing all auto-fixable script patches
              for mods in your active profile ({activeProfile?.loadOrder.length ?? 0} mods).
              Non-fixable Lua API issues will be listed in the description for manual review.
            </p>
            <div className="space-y-2 mb-4">
              <label className="text-xs font-medium text-foreground">Mod name</label>
              <Input
                value={packName}
                onChange={(e) => setPackName(e.target.value)}
                placeholder="B42_Modpack_Fixes"
                autoFocus
                onKeyDown={(e) => { if (e.key === "Enter" && packName.trim()) handleGenerateModpackFixes(); }}
              />
              <p className="text-xs text-muted-foreground">
                This becomes the mod ID and folder name. The mod will be created in your local mods directory
                with all patched mods as dependencies.
              </p>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => setPackNameDialog(false)}>
                Cancel
              </Button>
              <Button size="sm" onClick={handleGenerateModpackFixes} disabled={!packName.trim()}>
                Generate
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Table Row                                                           */
/* ------------------------------------------------------------------ */

function ModRow({ report, expanded, onToggle }: {
  report: CompatReport;
  expanded: boolean;
  onToggle: () => void;
}) {
  const [fixing, setFixing] = useState(false);

  const handleAutoFix = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setFixing(true);
    try {
      const result = await invoke<{
        modId: string;
        modName: string;
        outputPath: string;
        fixesApplied: number;
        todosAdded: number;
        translationEntries: number;
      }>("auto_fix_mod_cmd", { modId: report.modId });
      toast({
        title: "Auto-Fix Complete",
        description: `${result.fixesApplied} fixes applied, ${result.todosAdded} TODOs added. Saved to: ${result.outputPath}`,
      });
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      toast({ title: "Auto-Fix Failed", description: msg, variant: "destructive" });
    } finally {
      setFixing(false);
    }
  };

  const allIssues = report.totalIssues + report.scriptIssues + (report.missingRefCount || 0);
  const badge = allIssues === 0
    ? <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-green-500/15 text-green-500">OK</span>
    : allIssues <= 5
    ? <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-500/15 text-yellow-500">{allIssues} issues</span>
    : <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-red-500/15 text-red-500">{allIssues} issues</span>;

  return (
    <>
      <tr
        className={cn(
          "border-b border-border/50 cursor-pointer hover:bg-muted/30 transition-colors",
          expanded && "bg-muted/20"
        )}
        onClick={onToggle}
      >
        <td className="px-4 py-2.5">
          <div className="font-medium">{report.modName}</div>
          <div className="text-xs text-muted-foreground">
            {report.modId}
            {report.activeFolder && (
              <span className="text-blue-400 ml-1">[{report.activeFolder}/]</span>
            )}
          </div>
        </td>
        <td className="text-center px-3 py-2 text-xs text-muted-foreground">
          {report.detectedVersion}
        </td>
        <td className="text-center px-3 py-2 text-xs text-muted-foreground">
          {report.rulesUsed}
        </td>
        <td className="text-center px-3 py-2 text-muted-foreground">{report.filesScanned}</td>
        <td className="text-center px-3 py-2">
          {report.totalIssues > 0
            ? <span className="font-semibold text-yellow-500">{report.totalIssues}</span>
            : <span className="text-muted-foreground">0</span>}
        </td>
        <td className="text-center px-3 py-2">
          {report.scriptIssues > 0
            ? <span className="font-semibold text-orange-500">{report.scriptIssues}</span>
            : <span className="text-muted-foreground">0</span>}
        </td>
        <td className="text-center px-3 py-2">
          {(report.missingRefCount || 0) > 0
            ? <span className="font-semibold text-red-500">{report.missingRefCount}</span>
            : <span className="text-muted-foreground">0</span>}
        </td>
        <td className="text-center px-3 py-2">
          {badge}
          {report.scriptIssues > 0 && (
            <div className="text-[10px] text-muted-foreground mt-0.5">
              {report.scriptIssues} script fixes
            </div>
          )}
          {(report.missingRefCount || 0) > 0 && (
            <div className={`text-[10px] mt-0.5 ${report.missingRefs?.some((r) => r.severity === "error") ? "text-red-400" : "text-yellow-400"}`}>
              {report.missingRefCount} missing ref{report.missingRefCount !== 1 ? "s" : ""}
              {report.missingRefs?.some((r) => r.severity === "error") ? " (crash)" : ""}
            </div>
          )}
        </td>
      </tr>
      {expanded && (report.issues.length > 0 || report.scriptReport || (report.missingRefs?.length || 0) > 0) && (
        <tr>
          <td colSpan={8} className="bg-muted/10 px-6 py-4 border-b border-border">
            {allIssues > 0 && (
              <div className="flex items-center gap-2 mb-3">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={fixing}
                  onClick={handleAutoFix}
                >
                  {fixing ? "Fixing..." : "Auto-Fix (create local copy)"}
                </Button>
                <span className="text-xs text-muted-foreground">
                  {report.scriptIssues > 0 && report.totalIssues > 0
                    ? `Fixes script properties (${report.scriptIssues}), adds TODOs for Lua API issues (${report.totalIssues})`
                    : report.scriptIssues > 0
                    ? `Fixes ${report.scriptIssues} script property issues`
                    : `Adds TODO comments for ${report.totalIssues} Lua API issues (manual review needed)`}
                </span>
              </div>
            )}
            {report.issues.length > 0 && (
              <>
                <div className="text-xs font-medium text-muted-foreground mb-2">Lua API Issues ({report.totalIssues})</div>
                <IssueDetails issues={report.issues} />
              </>
            )}
            {report.scriptReport && report.scriptReport.issues.length > 0 && (
              <>
                <div className="text-xs font-medium text-orange-400 mb-2 mt-3">Script Property Issues ({report.scriptIssues})</div>
                <ScriptIssueDetails issues={report.scriptReport.issues} />
              </>
            )}
            {report.missingRefs && report.missingRefs.length > 0 && (
              <>
                <div className="text-xs font-medium text-red-400 mb-2 mt-3">
                  Missing Item References ({report.missingRefCount})
                  {report.missingRefs.some((r) => r.severity === "error")
                    ? " — includes CRASH risks"
                    : " — non-fatal (broken features)"}
                </div>
                <MissingRefDetails refs={report.missingRefs} />
              </>
            )}
          </td>
        </tr>
      )}
    </>
  );
}

/* ------------------------------------------------------------------ */
/* Issue Details                                                       */
/* ------------------------------------------------------------------ */

function IssueDetails({ issues }: { issues: MigrationIssue[] }) {
  const byFile: Record<string, MigrationIssue[]> = {};
  for (const issue of issues) {
    (byFile[issue.file] ??= []).push(issue);
  }

  return (
    <div className="space-y-3 max-h-64 overflow-auto">
      {Object.entries(byFile).map(([file, fileIssues]) => (
        <div key={file}>
          <div className="text-xs font-mono text-blue-400 mb-1">{file}</div>
          <div className="space-y-1.5 pl-3 border-l-2 border-border">
            {fileIssues.map((issue, i) => (
              <div key={i} className="flex items-start gap-2 text-xs">
                <span className="text-yellow-500 font-mono shrink-0 w-8 text-right">:{issue.line}</span>
                <span className="font-mono text-red-400 bg-red-500/10 px-1 rounded">{issue.oldApi}</span>
                {issue.replacement ? (
                  <>
                    <span className="text-muted-foreground">&rarr;</span>
                    <span className="font-mono text-green-400 bg-green-500/10 px-1 rounded">{issue.replacement}</span>
                  </>
                ) : (
                  <span className="text-muted-foreground">{issue.message}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Script Issue Details                                                */
/* ------------------------------------------------------------------ */

function MissingRefDetails({ refs }: { refs: { file: string; line: number; context: string; blockName: string; property: string; referencedItem: string; severity: string }[] }) {
  const byFile: Record<string, typeof refs> = {};
  for (const ref_ of refs) {
    (byFile[ref_.file] ??= []).push(ref_);
  }

  return (
    <div className="space-y-3 max-h-64 overflow-auto">
      {Object.entries(byFile).map(([file, fileRefs]) => (
        <div key={file}>
          <div className="text-xs font-mono text-red-400 mb-1">{file}</div>
          <div className="space-y-1.5 pl-3 border-l-2 border-red-500/30">
            {fileRefs.map((ref_, i) => (
              <div key={i} className="flex items-start gap-2 text-xs">
                <span className="text-yellow-500 font-mono shrink-0 w-8 text-right">:{ref_.line}</span>
                <span className="font-mono text-red-400 bg-red-500/10 px-1 rounded">
                  {ref_.referencedItem}
                </span>
                <span className="text-muted-foreground">
                  in {ref_.blockName} ({ref_.context === "recipe" ? "recipe ingredient" : ref_.property})
                </span>
                {ref_.severity === "error" && (
                  <span className="text-red-400 font-medium">CRASH</span>
                )}
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Script Issue Details                                                */
/* ------------------------------------------------------------------ */

function ScriptIssueDetails({ issues }: { issues: { file: string; line: number; property: string; value: string; suggestion: string; blockType: string }[] }) {
  const byFile: Record<string, typeof issues> = {};
  for (const issue of issues) {
    (byFile[issue.file] ??= []).push(issue);
  }

  return (
    <div className="space-y-3 max-h-64 overflow-auto">
      {Object.entries(byFile).map(([file, fileIssues]) => (
        <div key={file}>
          <div className="text-xs font-mono text-orange-400 mb-1">{file}</div>
          <div className="space-y-1.5 pl-3 border-l-2 border-orange-500/30">
            {fileIssues.map((issue, i) => (
              <div key={i} className="flex items-start gap-2 text-xs">
                <span className="text-yellow-500 font-mono shrink-0 w-8 text-right">:{issue.line}</span>
                <span className="font-mono text-orange-400 bg-orange-500/10 px-1 rounded">
                  {issue.property} = {issue.value.slice(0, 30)}
                </span>
                <span className="text-muted-foreground">{issue.suggestion}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
