export interface CrashReport {
  timestamp: string | null;
  errorType: string;
  logExcerpt: string;
  suspectMods: SuspectMod[];
  fullLogPath: string;
}

export interface SuspectMod {
  modId: string;
  confidence: string; // "high" | "medium" | "low"
  reason: string;
}

export interface PreflightResult {
  passed: boolean;
  checks: PreflightCheck[];
}

export interface PreflightCheck {
  name: string;
  status: string; // "pass" | "warn" | "fail"
  message: string;
  details: string[];
}

export interface BisectSession {
  id: string;
  allMods: string[];
  suspects: string[];
  testMods: string[];
  step: number;
  maxSteps: number;
  status: string; // "testing" | "found" | "not_found"
  culprit: string | null;
}
