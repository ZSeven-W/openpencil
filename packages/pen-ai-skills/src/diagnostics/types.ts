export type IssueSeverity = 'error' | 'warning' | 'info';

export type IssueCategory =
  | 'invisible-container'
  | 'empty-path'
  | 'text-explicit-height'
  | 'sibling-inconsistency';

export interface Issue {
  /** Node 检测到问题的 ID */
  nodeId: string;
  /** Which 检测器产生了这个问题 */
  category: IssueCategory;
  /** Severity 用于报告 — 所有当前检测器都会产生“警告” */
  severity: IssueSeverity;
  /** Property 名称或特殊标记 '__remove' */
  property: string;
  /** 节点上的 Current 值（原始） */
  currentValue: unknown;
  /** Value 检测器建议（“修复”） */
  suggestedValue: unknown;
  /** Human-可读原因，匹配原始 `fix.reason` 字符串 */
  reason: string;
}
