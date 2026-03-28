export type IssueSeverity = 'error' | 'warning' | 'info';

export interface LoadOrderIssue {
  severity: IssueSeverity;
  modId: string;
  message: string;
  suggestion: string | null;
  relatedModId: string | null;
}
