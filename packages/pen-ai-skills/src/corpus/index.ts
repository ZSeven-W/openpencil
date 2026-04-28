// Browser-safe corpus surface. Everything re-exported here must avoid
// top-level `node:fs` / other Node-only imports, because the main
// `@zseven-w/pen-ai-skills` barrel (src/index.ts) forwards from here
// and is consumed by the Vite browser bundle for `design-parser.ts`
// (N-tool detection inside the embedded orchestrator).
//
// Node-only helpers — currently `loadCorpus` (reads YAML files via
// `node:fs`) — are intentionally NOT re-exported. Node callers
// (scripts/ab-corpus/*) import them directly from the file:
//   import { loadCorpus } from '@zseven-w/pen-ai-skills/src/corpus/corpus-loader';
// or via a relative path from outside node_modules. See
// packages/pen-ai-skills/src/corpus/corpus-loader.ts JSDoc.
export { parseModelOutput } from './output-parser';
export { scoreRun, countRoles } from './score-run';
export { aggregate } from './aggregate';
export { mockLlmRaw, mockLlmParsed } from './mock-llm';
export type { MockLlmArgs, MockLlmOverride } from './mock-llm';
export type {
  ApplyFn,
  ApplyResult,
  CategorySummary,
  CorpusExpected,
  CorpusPrompt,
  ModelSummary,
  ParsedOutput,
  Report,
  ScoreRow,
  TokenUsage,
  ToolUsage,
} from './types';
