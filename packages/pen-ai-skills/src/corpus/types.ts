import type { PenDocument } from '@zseven-w/pen-types';
import type { Issue } from '../diagnostics/types';

/**
 * Corpus prompt definition — matches the YAML schema under
 * `packages/pen-ai-skills/corpus/ab-v0/*.yaml` (and ab-v1 / ab-v3).
 *
 * `difficulty` values:
 *   - `obvious`   — single-tool intent. `expected_tool_if_any` is set
 *                   and routing classifies as right/wrong/fallback/garbage.
 *   - `optional`  — single-tool intent that doesn't *require* any tool;
 *                   batch_design fallback is acceptable. Routing = 'n/a'.
 *   - `composite` — multi-tool intent where no single tool fits the
 *                   whole brief. `expected_tool_if_any` is omitted; the
 *                   model is expected to either chain element tools or
 *                   fall back to batch_design. Composite-treatment runs
 *                   classify into multi-tool / fallback / garbage —
 *                   different bucket from obvious's right/wrong/etc.
 *                   See score-run.ts::classifyRouting.
 */
export interface CorpusPrompt {
  id: string;
  category: 'mobile' | 'dashboard' | 'landing';
  difficulty: 'obvious' | 'optional' | 'composite';
  prompt: string;
  expected: CorpusExpected;
  expected_tool_if_any?: string;
}

/**
 * Provider-reported token usage for a single model call. Populated by
 * the OpenAI-compatible clients from `data.usage.{prompt,completion}_tokens`
 * when present. Codex CLI doesn't surface usage, so its calls report 0/0
 * — aggregate filters those out of token-cost averages so the codex
 * column doesn't read as "free" when really it's "unmeasured".
 */
export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
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
  /** Routing verdict on treatment runs of obvious / composite prompts.
   *  Values, by `difficulty`:
   *
   *   `obvious` (4 buckets, sum to 1):
   *   - 'right-tool'   — kind=tool_call AND toolName=expectedToolIfAny
   *   - 'wrong-tool'   — kind=tool_call AND toolName≠expectedToolIfAny
   *   - 'fallback'     — kind=batch_design (didn't route to any tool)
   *   - 'garbage'      — kind=garbage (unparseable)
   *
   *   `composite` (3 buckets, sum to 1):
   *   - 'multi-tool'   — kind=tool_call (chained or single, no
   *                      expected_tool_if_any to match against)
   *   - 'fallback'     — kind=batch_design (multi-element via DSL)
   *   - 'garbage'      — kind=garbage
   *
   *   Other:
   *   - 'n/a'          — baseline run OR `difficulty: 'optional'`
   *
   *  Aggregate splits these into M5 (obvious) and M6 (composite)
   *  breakdowns. Garbage is its own bucket in both, never silently
   *  dropped, so a model that emits 10 garbage + 1 right + 1 wrong
   *  doesn't look like "50% right-tool". */
  routing: 'right-tool' | 'wrong-tool' | 'multi-tool' | 'fallback' | 'garbage' | 'n/a';
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
  /** Provider-reported prompt token count for this call. 0 when the
   *  client doesn't surface usage (Codex CLI), when the call errored
   *  before any usage came back (HARNESS_ERROR), or in stub/dry-run
   *  modes that don't simulate token counts. */
  promptTokens: number;
  /** Provider-reported completion token count. Same caveat as
   *  promptTokens — 0 means "unknown", not "actually zero". */
  completionTokens: number;
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
  /** Routing breakdown on the `difficulty: 'composite'` treatment subset.
   *  Three rates summing to 1: multi-tool (model emitted at least one
   *  tool_call — composite doesn't require a specific tool), fallback
   *  (batch_design DSL), garbage (unparseable). NaN when the model has
   *  no composite-treatment runs in this report. */
  m6_multi_tool: number;
  m6_fallback: number;
  m6_garbage: number;
  /** Run counts for each cell so we can show sample size in the report. */
  runCountBaseline: number;
  runCountTreatment: number;
  /** Mean prompt tokens per call, baseline arm. NaN if no rows in this
   *  cell reported usage (e.g. Codex CLI which doesn't surface it). */
  avgPromptTokensBaseline: number;
  avgPromptTokensTreatment: number;
  avgCompletionTokensBaseline: number;
  avgCompletionTokensTreatment: number;
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
