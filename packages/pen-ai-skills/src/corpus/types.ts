import type { PenDocument } from '@zseven-w/pen-types';
import type { Issue } from '../diagnostics/types';

/**
 * Corpus prompt definition — matches the YAML schema under
 * `packages/pen-ai-skills/corpus/ab-v0/*.yaml`.
 */
export interface CorpusPrompt {
  id: string;
  category: 'mobile' | 'dashboard' | 'landing';
  difficulty: 'obvious' | 'optional';
  prompt: string;
  expected: CorpusExpected;
  expected_tool_if_any?: string;
}

export interface CorpusExpected {
  /** Roles whose presence somewhere in the rendered tree is required. */
  must_contain_roles?: string[];
  /** role → minimum instance count. Failing any key → M3 = 0. */
  min_roles?: Record<string, number>;
}

/**
 * Parsed model output. Weak models can emit either an element-tool call
 * (our treatment-arm simulation) or the legacy `batch_design` DSL; we
 * detect which and surface a tagged union so the harness dispatches
 * correctly. `garbage` captures parse failures — both tags missing or
 * malformed payload.
 */
export type ParsedOutput =
  | { kind: 'tool_call'; name: string; arguments: Record<string, unknown>; raw: string }
  | { kind: 'batch_design'; dsl: string; raw: string }
  | { kind: 'garbage'; reason: string; raw: string };

/**
 * Abstracted apply-to-document interface. The scorer consumes this; the
 * harness implements it on top of `pen-mcp`'s `handleBatchDesign` +
 * element-tool handlers. Kept as an interface so the scorer stays
 * pen-mcp-independent (avoids the pen-ai-skills ↔ pen-mcp cycle).
 */
export interface ApplyResult {
  /** True iff apply ran without throwing and produced a non-null root. */
  ok: boolean;
  /** Error message when ok=false, empty string when ok=true. */
  error: string;
  /** The resulting document state, or null when apply failed. */
  doc: PenDocument | null;
}

export type ApplyFn = (parsed: ParsedOutput) => Promise<ApplyResult>;

/**
 * Single-run score row. One row per (model, variant, prompt) triple.
 */
export interface ScoreRow {
  promptId: string;
  category: CorpusPrompt['category'];
  difficulty: CorpusPrompt['difficulty'];
  model: string;
  variant: 'B' | 'T';
  outputKind: ParsedOutput['kind'];
  /** Tool name when outputKind='tool_call', else empty. */
  toolName: string;
  /** Copied from the prompt so the aggregator can score routing without
   *  re-joining against the corpus. Empty for `difficulty: 'optional'`
   *  (M5 routing is undefined there). */
  expectedToolIfAny: string;
  /** Routing verdict on `difficulty: 'obvious'` treatment runs only.
   *  Values:
   *   - 'right-tool'   — outputKind=tool_call AND toolName=expectedToolIfAny
   *   - 'wrong-tool'   — outputKind=tool_call AND toolName≠expectedToolIfAny
   *   - 'fallback'     — outputKind=batch_design (didn't route to any tool)
   *   - 'garbage'      — outputKind=garbage (unparseable; nothing to route)
   *   - 'n/a'          — baseline run OR `difficulty: 'optional'`
   *  Aggregate's denominator = obvious-treatment runs (routing !== 'n/a'),
   *  so the four rates sum to 1. Including garbage in the denominator
   *  prevents "right-tool rate looks great while most outputs are
   *  broken" (e.g. 10 garbage + 1 right + 1 wrong would otherwise report
   *  50% right-tool even though only 2/12 runs were usable). */
  routing: 'right-tool' | 'wrong-tool' | 'fallback' | 'garbage' | 'n/a';
  /** M1: parse + apply + no detector errors. */
  m1_legal: boolean;
  /** M3: M1 AND expected shape checks pass. */
  m3_success: boolean;
  /** Non-empty when the expected shape check failed — for diagnosis. */
  m3_failure_reason: string;
  /** Detector issues found on the applied tree (empty when apply failed). */
  issues: Issue[];
  /** Apply-phase error (empty on success). */
  applyError: string;
  /** Raw model output — kept for debugging, not for aggregation. */
  rawOutput: string;
}

/**
 * Aggregated report across a set of ScoreRows. Emitted as report.md +
 * report.json by the harness.
 */
export interface Report {
  /** Timestamp when the report was generated (ISO 8601). */
  generatedAt: string;
  /** How many rows this report summarizes. */
  totalRuns: number;
  /** Per-model summary rows. */
  byModel: ModelSummary[];
  /** Per-category summary rows (both variants collapsed). */
  byCategory: CategorySummary[];
  /** Per-tool usage: which element tools got invoked in treatment runs. */
  byTool: ToolUsage[];
}

export interface ModelSummary {
  model: string;
  /** M1 rate in baseline variant (0..1). */
  m1_baseline: number;
  /** M1 rate in treatment variant (0..1). */
  m1_treatment: number;
  /** Treatment - baseline, expressed in percentage points. */
  m1_delta_pp: number;
  m3_baseline: number;
  m3_treatment: number;
  m3_delta_pp: number;
  /** Routing breakdown on the `difficulty: 'obvious'` treatment subset —
   *  four rates that sum to 1 (modulo NaN when the subset is empty).
   *  Including garbage in the denominator (rather than silently
   *  dropping it) prevents right-tool rates from looking rosy when
   *  most outputs failed to parse. Splits:
   *   - right-tool: routed to expected_tool_if_any
   *   - wrong-tool: routed to an element tool BUT not the expected one
   *   - fallback:   emitted batch_design instead of any tool
   *   - garbage:    output unparseable — counted here so the denominator
   *                 is "all obvious treatment runs" and the rate is
   *                 interpretable without cross-referencing M1. */
  m5_right_tool: number;
  m5_wrong_tool: number;
  m5_fallback: number;
  m5_garbage: number;
  /** Run counts for each cell so we can show sample size in the report. */
  runCountBaseline: number;
  runCountTreatment: number;
}

export interface CategorySummary {
  category: CorpusPrompt['category'];
  m1_baseline: number;
  m1_treatment: number;
  m1_delta_pp: number;
}

export interface ToolUsage {
  tool: string;
  /** How many times this tool was invoked across all treatment runs. */
  invocations: number;
  /** How many of those invocations passed M1. */
  successfulInvocations: number;
}
