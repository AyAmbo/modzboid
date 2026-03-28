import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../../../shared/components/ui/button";
import { cn } from "../../../shared/lib/utils";

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
}

interface MigrationVersionsData {
  versions: { from: string; to: string; ruleCount: number }[];
  latestVersion: string;
}

/* ------------------------------------------------------------------ */
/* Component                                                           */
/* ------------------------------------------------------------------ */

export function ModCompatibilitySection() {
  const [reports, setReports] = useState<CompatReport[] | null>(null);
  const [expandedMod, setExpandedMod] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [noExtension, setNoExtension] = useState(false);
  const [latestVersion, setLatestVersion] = useState("");

  // Check if extension is installed
  useEffect(() => {
    invoke<MigrationVersionsData>("list_migration_versions_cmd")
      .then((data) => {
        if (data.versions.length === 0) {
          setNoExtension(true);
        } else {
          setLatestVersion(data.latestVersion);
        }
      })
      .catch(() => setNoExtension(true));
  }, []);

  const handleScan = async () => {
    setScanning(true);
    setReports(null);
    setExpandedMod(null);
    try {
      const result = await invoke<CompatReport[]>("scan_all_mods_compat_cmd");
      setReports(result);
    } catch (err) {
      console.error("Scan failed:", err);
    } finally {
      setScanning(false);
    }
  };

  if (noExtension) {
    return (
      <div className="text-sm text-muted-foreground">
        Install the <span className="font-mono text-foreground">pz-migration-rules</span> extension
        from the Extensions page to use the compatibility scanner.
      </div>
    );
  }

  const totalMods = reports?.length ?? 0;
  const modsOk = reports?.filter((r) => r.totalIssues === 0).length ?? 0;
  const modsWithIssues = reports?.filter((r) => r.totalIssues > 0).length ?? 0;
  const totalIssues = reports?.reduce((s, r) => s + r.totalIssues, 0) ?? 0;

  return (
    <div className="space-y-4">
      {/* Scan button */}
      <div className="flex items-center gap-3 flex-wrap">
        <Button onClick={handleScan} disabled={scanning} size="sm">
          {scanning ? "Scanning..." : "Scan All Mods"}
        </Button>
        {latestVersion && (
          <span className="text-xs text-muted-foreground">
            Checks each mod against its matching rules (auto-detected from mod version).
            Target: B{latestVersion}
          </span>
        )}
      </div>

      {/* Summary stats */}
      {reports && (
        <div className="flex gap-4 text-xs">
          <span><strong className="text-foreground">{totalMods}</strong> mods scanned</span>
          <span className="text-green-500"><strong>{modsOk}</strong> OK</span>
          {modsWithIssues > 0 && (
            <span className="text-yellow-500"><strong>{modsWithIssues}</strong> with issues</span>
          )}
          {totalIssues > 0 && (
            <span className="text-red-500"><strong>{totalIssues}</strong> total issues</span>
          )}
        </div>
      )}

      {/* Results table */}
      {reports && reports.length > 0 && (
        <div className="border border-border rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="bg-muted/50 border-b border-border">
                <th className="text-left px-3 py-2 font-medium text-xs">Mod</th>
                <th className="text-center px-3 py-2 font-medium text-xs w-20">Version</th>
                <th className="text-center px-3 py-2 font-medium text-xs w-16">Files</th>
                <th className="text-center px-3 py-2 font-medium text-xs w-16">Issues</th>
                <th className="text-center px-3 py-2 font-medium text-xs w-24">Status</th>
              </tr>
            </thead>
            <tbody>
              {reports.map((r) => (
                <ModRow
                  key={r.modId}
                  report={r}
                  expanded={expandedMod === r.modId}
                  onToggle={() =>
                    setExpandedMod(expandedMod === r.modId ? null : r.modId)
                  }
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {reports && reports.length === 0 && (
        <p className="text-sm text-muted-foreground">No mods found to scan.</p>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Table Row                                                           */
/* ------------------------------------------------------------------ */

function ModRow({
  report,
  expanded,
  onToggle,
}: {
  report: CompatReport;
  expanded: boolean;
  onToggle: () => void;
}) {
  const statusBadge =
    report.totalIssues === 0 ? (
      <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-green-500/15 text-green-500">
        OK
      </span>
    ) : report.totalIssues <= 5 ? (
      <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-500/15 text-yellow-500">
        {report.totalIssues} issues
      </span>
    ) : (
      <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-red-500/15 text-red-500">
        {report.totalIssues} issues
      </span>
    );

  return (
    <>
      <tr
        className={cn(
          "border-b border-border/50 cursor-pointer hover:bg-muted/30",
          expanded && "bg-muted/20"
        )}
        onClick={onToggle}
      >
        <td className="px-3 py-2">
          <span className="font-medium">{report.modName}</span>
          <span className="text-muted-foreground text-xs ml-2">{report.modId}</span>
          {report.activeFolder && (
            <span className="text-xs text-blue-400 ml-1">[{report.activeFolder}/]</span>
          )}
        </td>
        <td className="text-center px-3 py-2">
          <span className="text-xs text-muted-foreground" title={`Rules: ${report.rulesUsed}`}>
            {report.detectedVersion}
          </span>
        </td>
        <td className="text-center px-3 py-2 text-muted-foreground">
          {report.filesScanned}
        </td>
        <td className="text-center px-3 py-2">
          {report.totalIssues > 0 ? (
            <span className="font-semibold text-yellow-500">{report.totalIssues}</span>
          ) : (
            <span className="text-muted-foreground">0</span>
          )}
        </td>
        <td className="text-center px-3 py-2">{statusBadge}</td>
      </tr>
      {expanded && report.issues.length > 0 && (
        <tr>
          <td colSpan={5} className="bg-muted/10 px-4 py-3">
            <div className="text-xs text-muted-foreground mb-2">
              Rules used: <span className="text-foreground">{report.rulesUsed}</span>
            </div>
            <IssueDetails issues={report.issues} />
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
    <div className="space-y-3">
      {Object.entries(byFile).map(([file, fileIssues]) => (
        <div key={file}>
          <div className="text-xs font-mono text-blue-400 mb-1">{file}</div>
          <div className="space-y-1.5 pl-3 border-l-2 border-border">
            {fileIssues.map((issue, i) => (
              <div key={i} className="flex items-start gap-2 text-xs">
                <span className="text-yellow-500 font-mono shrink-0">:{issue.line}</span>
                <span className="font-mono text-red-400 bg-red-500/10 px-1 rounded">
                  {issue.oldApi}
                </span>
                {issue.replacement && (
                  <>
                    <span className="text-muted-foreground">&rarr;</span>
                    <span className="font-mono text-green-400 bg-green-500/10 px-1 rounded">
                      {issue.replacement}
                    </span>
                  </>
                )}
                {!issue.replacement && (
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
