// packages/pen-ai-skills/src/diagnostics/index.ts
export type { Issue, IssueSeverity, IssueCategory } from './types';
export {
  detectInvisibleContainers,
  detectEmptyPaths,
  detectTextExplicitHeights,
  detectSiblingInconsistencies,
  detectAllIssues,
} from './detectors';
