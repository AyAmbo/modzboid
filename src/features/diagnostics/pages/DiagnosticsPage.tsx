import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../../../shared/components/ui/button";
import { toast } from "../../../shared/components/ui/toaster";
import { useProfileStore } from "../../profiles/store";
import { cn } from "../../../shared/lib/utils";
import type {
  PreflightResult,
  PreflightCheck,
  CrashReport,
  BisectSession,
} from "../types";

export default function DiagnosticsPage() {
  // Section collapse state
  const [preflightOpen, setPreflightOpen] = useState(true);
  const [crashOpen, setCrashOpen] = useState(true);
  const [bisectOpen, setBisectOpen] = useState(true);

  return (
    <div data-testid="page-diagnostics" className="p-6 max-w-3xl flex flex-col gap-6 h-full overflow-auto">
      {/* Header */}
      <div>
        <h1 className="text-xl font-semibold">Diagnostics</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Troubleshoot mod issues with pre-flight checks, crash analysis, and
          bisect debugging.
        </p>
      </div>

      {/* Section 1: Pre-flight Check */}
      <div data-testid="section-preflight">
        <CollapsibleSection
          title="Pre-flight Check"
          open={preflightOpen}
          onToggle={() => setPreflightOpen((o) => !o)}
        >
          <PreflightSection />
        </CollapsibleSection>
      </div>

      {/* Section 2: Crash Analysis */}
      <div data-testid="section-crash">
        <CollapsibleSection
          title="Crash Analysis"
          open={crashOpen}
          onToggle={() => setCrashOpen((o) => !o)}
        >
          <CrashAnalysisSection />
        </CollapsibleSection>
      </div>

      {/* Section 3: Bisect Debugger */}
      <div data-testid="section-bisect">
        <CollapsibleSection
          title="Bisect Debugger"
          open={bisectOpen}
          onToggle={() => setBisectOpen((o) => !o)}
        >
          <BisectSection />
        </CollapsibleSection>
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Collapsible Section Wrapper                                        */
/* ------------------------------------------------------------------ */

function CollapsibleSection({
  title,
  open,
  onToggle,
  children,
}: {
  title: string;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="border border-border rounded-lg bg-card">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 text-sm font-medium hover:bg-muted rounded-t-lg"
      >
        <span>{title}</span>
        <span className="text-muted-foreground">{open ? "\u25B2" : "\u25BC"}</span>
      </button>
      {open && <div className="px-4 pb-4">{children}</div>}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Section 1: Pre-flight Check                                        */
/* ------------------------------------------------------------------ */

function PreflightSection() {
  const activeProfileId = useProfileStore((s) => s.activeProfileId);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<PreflightResult | null>(null);

  const runCheck = async () => {
    if (!activeProfileId) {
      toast({
        title: "No Active Profile",
        description: "Select a profile before running pre-flight checks.",
        variant: "destructive",
      });
      return;
    }
    setLoading(true);
    try {
      const res = await invoke<PreflightResult>("preflight_check_cmd", {
        profileId: activeProfileId,
      });
      setResult(res);
    } catch (err) {
      console.error("Pre-flight check failed:", err);
      toast({
        title: "Error",
        description: "Pre-flight check failed",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const issueCount = result
    ? result.checks.filter((c) => c.status !== "pass").length
    : 0;

  return (
    <div className="space-y-3">
      <Button data-testid="btn-preflight" onClick={runCheck} disabled={loading}>
        {loading ? "Running..." : "Run Pre-flight Check"}
      </Button>

      {result && (
        <div data-testid="preflight-results" className="space-y-2">
          <div
            className={cn(
              "text-sm font-medium",
              result.passed ? "text-green-500" : "text-red-500"
            )}
          >
            {result.passed
              ? "All checks passed"
              : `${issueCount} issue${issueCount !== 1 ? "s" : ""} found`}
          </div>

          <div className="space-y-1">
            {result.checks.map((check) => (
              <CheckItem key={check.name} check={check} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function CheckItem({ check }: { check: PreflightCheck }) {
  const [expanded, setExpanded] = useState(false);

  const statusIcon =
    check.status === "pass"
      ? "\u2705"
      : check.status === "warn"
        ? "\u26A0\uFE0F"
        : "\u274C";

  return (
    <div className="border border-border rounded p-2">
      <button
        onClick={() => setExpanded((o) => !o)}
        className="w-full flex items-center gap-2 text-sm text-left"
      >
        <span>{statusIcon}</span>
        <span className="font-medium">{check.name}</span>
        <span className="text-muted-foreground ml-auto">{check.message}</span>
      </button>
      {expanded && check.details.length > 0 && (
        <ul className="mt-2 ml-7 space-y-0.5 text-xs text-muted-foreground">
          {check.details.map((d) => (
            <li key={d}>{d}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Section 2: Crash Analysis                                          */
/* ------------------------------------------------------------------ */

function CrashAnalysisSection() {
  const [loading, setLoading] = useState(false);
  const [report, setReport] = useState<CrashReport | null>(null);
  const [noCrash, setNoCrash] = useState(false);

  const analyze = async () => {
    setLoading(true);
    setNoCrash(false);
    try {
      const res = await invoke<CrashReport>("analyze_crash_log_cmd");
      if (!res) {
        setNoCrash(true);
        setReport(null);
      } else {
        setReport(res);
      }
    } catch (err) {
      console.error("Crash analysis failed:", err);
      // No crash log found — show "no data" message instead of just a toast
      setNoCrash(true);
      setReport(null);
    } finally {
      setLoading(false);
    }
  };

  const confidenceBadgeClass = (confidence: string) => {
    switch (confidence) {
      case "high":
        return "bg-red-500/20 text-red-400 border-red-500/30";
      case "medium":
        return "bg-yellow-500/20 text-yellow-400 border-yellow-500/30";
      default:
        return "bg-gray-500/20 text-gray-400 border-gray-500/30";
    }
  };

  return (
    <div className="space-y-3">
      <Button data-testid="btn-analyze-crash" onClick={analyze} disabled={loading}>
        {loading ? "Analyzing..." : "Analyze Latest Crash"}
      </Button>

      {noCrash && (
        <p className="text-sm text-muted-foreground">
          No crash data found in logs.
        </p>
      )}

      {report && (
        <div data-testid="crash-report" className="space-y-3">
          {/* Error type badge */}
          <div className="inline-block px-2 py-0.5 rounded text-xs font-medium bg-red-500/20 text-red-400 border border-red-500/30">
            {report.errorType}
          </div>

          {/* Log excerpt */}
          <pre className="text-xs bg-muted p-3 rounded overflow-auto max-h-60 font-mono whitespace-pre-wrap">
            {report.logExcerpt}
          </pre>

          {/* Suspect mods table */}
          {report.suspectMods.length > 0 && (
            <div>
              <h4 className="text-sm font-medium mb-1">Suspect Mods</h4>
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="bg-muted text-left">
                      <th className="px-3 py-1.5 font-medium">Mod ID</th>
                      <th className="px-3 py-1.5 font-medium">Confidence</th>
                      <th className="px-3 py-1.5 font-medium">Reason</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.suspectMods.map((mod) => (
                      <tr key={mod.modId} className="border-t border-border">
                        <td className="px-3 py-1.5 font-mono text-xs">
                          {mod.modId}
                        </td>
                        <td className="px-3 py-1.5">
                          <span
                            className={cn(
                              "inline-block px-1.5 py-0.5 rounded text-xs border",
                              confidenceBadgeClass(mod.confidence)
                            )}
                          >
                            {mod.confidence}
                          </span>
                        </td>
                        <td className="px-3 py-1.5 text-muted-foreground">
                          {mod.reason}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Full log path */}
          <p className="text-xs text-muted-foreground">
            Full log: <span className="font-mono">{report.fullLogPath}</span>
          </p>
        </div>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Section 3: Bisect Debugger                                         */
/* ------------------------------------------------------------------ */

function BisectSection() {
  const activeProfileId = useProfileStore((s) => s.activeProfileId);
  const [loading, setLoading] = useState(false);
  const [session, setSession] = useState<BisectSession | null>(null);

  const startBisect = async () => {
    if (!activeProfileId) {
      toast({
        title: "No Active Profile",
        description: "Select a profile before starting bisect.",
        variant: "destructive",
      });
      return;
    }
    setLoading(true);
    try {
      const res = await invoke<BisectSession>("bisect_start_cmd", {
        profileId: activeProfileId,
      });
      setSession(res);
    } catch (err) {
      console.error("Bisect start failed:", err);
      toast({
        title: "Error",
        description: "Failed to start bisect session",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const reportResult = async (crashed: boolean) => {
    if (!session) return;
    setLoading(true);
    try {
      const res = await invoke<BisectSession>("bisect_report_cmd", {
        session,
        crashed,
      });
      setSession(res);
    } catch (err) {
      console.error("Bisect report failed:", err);
      toast({
        title: "Error",
        description: "Failed to report bisect result",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const reset = () => {
    setSession(null);
  };

  // No active session — show start button
  if (!session) {
    return (
      <div className="space-y-3">
        <p className="text-sm text-muted-foreground">
          Binary-search through your mods to find which one is causing a crash.
        </p>
        <Button data-testid="btn-bisect-start" onClick={startBisect} disabled={loading}>
          {loading ? "Starting..." : "Start Bisect"}
        </Button>
      </div>
    );
  }

  // Session complete — found culprit
  if (session.status === "found") {
    return (
      <div className="space-y-3">
        <div data-testid="bisect-result" className="p-3 bg-green-500/10 border border-green-500/30 rounded text-sm text-green-400">
          Found the culprit:{" "}
          <span className="font-mono font-bold">{session.culprit}</span>
        </div>
        <Button variant="outline" onClick={reset}>
          Start Over
        </Button>
      </div>
    );
  }

  // Session complete — not found
  if (session.status === "not_found") {
    return (
      <div className="space-y-3">
        <div className="p-3 bg-yellow-500/10 border border-yellow-500/30 rounded text-sm text-yellow-400">
          Could not isolate the issue. The problem may involve multiple mods
          interacting.
        </div>
        <Button variant="outline" onClick={reset}>
          Start Over
        </Button>
      </div>
    );
  }

  // Active session — testing
  const progress =
    session.maxSteps > 0
      ? Math.round((session.step / session.maxSteps) * 100)
      : 0;

  return (
    <div className="space-y-3">
      {/* Step counter */}
      <div data-testid="bisect-progress" className="text-sm font-medium">
        Step {session.step}/{session.maxSteps}
      </div>

      {/* Progress bar */}
      <div className="h-2 bg-muted rounded-full overflow-hidden">
        <div
          className="h-full bg-primary transition-all"
          style={{ width: `${progress}%` }}
        />
      </div>

      {/* Test mods list */}
      <div>
        <p className="text-sm text-muted-foreground mb-1">
          Test with these {session.testMods.length} mods:
        </p>
        <div className="max-h-40 overflow-auto border border-border rounded p-2 space-y-0.5">
          {session.testMods.map((modId) => (
            <div key={modId} className="text-xs font-mono">
              {modId}
            </div>
          ))}
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-2">
        <Button
          data-testid="btn-bisect-crashed"
          variant="destructive"
          onClick={() => reportResult(true)}
          disabled={loading}
        >
          {loading ? "Reporting..." : "It Crashed"}
        </Button>
        <Button
          data-testid="btn-bisect-worked"
          className="bg-green-600 text-white hover:opacity-90"
          onClick={() => reportResult(false)}
          disabled={loading}
        >
          {loading ? "Reporting..." : "It Worked"}
        </Button>
        <Button data-testid="btn-bisect-cancel" variant="ghost" onClick={reset} className="ml-auto">
          Cancel
        </Button>
      </div>
    </div>
  );
}
