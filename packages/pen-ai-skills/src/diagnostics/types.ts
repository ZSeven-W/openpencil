export type IssueSeverity = 'error' | 'warning' | 'info';

export type IssueCategory =
  | 'invisible-container'
  | 'empty-path'
  | 'text-explicit-height'
  | 'sibling-inconsistency';

export interface Issue {
  /** Node id where the issue was detected */
  nodeId: string;
  /** Which detector produced this issue */
  category: IssueCategory;
  /** Severity for reporting — all current detectors produce 'warning' */
  severity: IssueSeverity;
  /** Property name or special sentinel '__remove' */
  property: string;
  /** Current value on the node (raw) */
  currentValue: unknown;
  /** Value the detector suggests (the 'fix') */
  suggestedValue: unknown;
  /** Human-readable reason, matches the original `fix.reason` strings */
  reason: string;
}
