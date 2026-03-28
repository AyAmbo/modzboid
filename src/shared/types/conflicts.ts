import type { IssueSeverity } from './validation';

export type ConflictType = 'fileOverride' | 'scriptIdClash' | 'versionMismatch' | 'knownIncompat' | 'functionOverride' | 'eventCollision';

export interface ModConflict {
  conflictType: ConflictType;
  severity: IssueSeverity;
  modIds: string[];
  filePath: string | null;
  scriptId: string | null;
  message: string;
  suggestion: string | null;
  isIntentional: boolean;
}
