import type {
  CategorySummary,
  CorpusPrompt,
  ModelSummary,
  Report,
  ScoreRow,
  ToolUsage,
} from './types';

/**
 * Reduce a list of per-run ScoreRows into a single Report. Pure
 * function — no I/O, deterministic output. Harness writes the report
 * to disk in its own step.
 *
 * Grouping choices and why:
 *   - `byModel` keeps B and T side by side so the A/B delta is
 *     legible without cross-referencing rows.
 *   - `byCategory` collapses models so we can answer "does N-tool help
 *     mobile more than landing?".
 *   - `byTool` only counts treatment-arm runs where the model actually
 *     emitted a tool_call — the whole point of M5 diagnostics.
 *
 * Zero-sample cells (e.g. a model that had no baseline runs) return
 * NaN for rates rather than 0 so a missing cell never looks like a 0%
 * pass rate.
 */
export function aggregate(rows: ScoreRow[]): Report {
  const models = unique(rows.map((r) => r.model));
  const byModel: ModelSummary[] = models.map((model) => buildModelSummary(model, rows));

  const categories: CorpusPrompt['category'][] = ['mobile', 'dashboard', 'landing'];
  const byCategory: CategorySummary[] = categories.map((cat) => buildCategorySummary(cat, rows));

  const byTool: ToolUsage[] = buildToolUsage(rows);

  return {
    generatedAt: new Date().toISOString(),
    totalRuns: rows.length,
    byModel,
    byCategory,
    byTool,
  };
}

function buildModelSummary(model: string, rows: ScoreRow[]): ModelSummary {
  const forModel = rows.filter((r) => r.model === model);
  const baseline = forModel.filter((r) => r.variant === 'B');
  const treatment = forModel.filter((r) => r.variant === 'T');
  const m1_baseline = rate(baseline, (r) => r.m1_legal);
  const m1_treatment = rate(treatment, (r) => r.m1_legal);
  const m3_baseline = rate(baseline, (r) => r.m3_success);
  const m3_treatment = rate(treatment, (r) => r.m3_success);
  // Obvious-treatment routing breakdown (4 buckets, sum to 1).
  const obviousRouted = treatment.filter((r) => r.difficulty === 'obvious' && r.routing !== 'n/a');
  const m5_right_tool = rate(obviousRouted, (r) => r.routing === 'right-tool');
  const m5_wrong_tool = rate(obviousRouted, (r) => r.routing === 'wrong-tool');
  const m5_fallback = rate(obviousRouted, (r) => r.routing === 'fallback');
  const m5_garbage = rate(obviousRouted, (r) => r.routing === 'garbage');
  // Composite-treatment routing breakdown (3 buckets, sum to 1). NaN
  // when this model has no composite-treatment runs at all.
  const compositeRouted = treatment.filter(
    (r) => r.difficulty === 'composite' && r.routing !== 'n/a',
  );
  const m6_multi_tool = rate(compositeRouted, (r) => r.routing === 'multi-tool');
  const m6_fallback = rate(compositeRouted, (r) => r.routing === 'fallback');
  const m6_garbage = rate(compositeRouted, (r) => r.routing === 'garbage');
  // Token cost averages exclude rows with 0/0 usage. Reasons a row
  // shows 0/0: Codex CLI doesn't surface usage; harness errors before
  // the call returned anything; dry-run stub. Treating those as zeros
  // would falsely deflate averages.
  const avgPromptTokensBaseline = avgUsage(baseline, (r) => r.promptTokens);
  const avgPromptTokensTreatment = avgUsage(treatment, (r) => r.promptTokens);
  const avgCompletionTokensBaseline = avgUsage(baseline, (r) => r.completionTokens);
  const avgCompletionTokensTreatment = avgUsage(treatment, (r) => r.completionTokens);
  return {
    model,
    m1_baseline,
    m1_treatment,
    m1_delta_pp: delta(m1_baseline, m1_treatment),
    m3_baseline,
    m3_treatment,
    m3_delta_pp: delta(m3_baseline, m3_treatment),
    m5_right_tool,
    m5_wrong_tool,
    m5_fallback,
    m5_garbage,
    m6_multi_tool,
    m6_fallback,
    m6_garbage,
    runCountBaseline: baseline.length,
    runCountTreatment: treatment.length,
    avgPromptTokensBaseline,
    avgPromptTokensTreatment,
    avgCompletionTokensBaseline,
    avgCompletionTokensTreatment,
  };
}

function buildCategorySummary(
  category: CorpusPrompt['category'],
  rows: ScoreRow[],
): CategorySummary {
  const forCat = rows.filter((r) => r.category === category);
  const baseline = forCat.filter((r) => r.variant === 'B');
  const treatment = forCat.filter((r) => r.variant === 'T');
  const m1_baseline = rate(baseline, (r) => r.m1_legal);
  const m1_treatment = rate(treatment, (r) => r.m1_legal);
  return {
    category,
    m1_baseline,
    m1_treatment,
    m1_delta_pp: delta(m1_baseline, m1_treatment),
  };
}

function buildToolUsage(rows: ScoreRow[]): ToolUsage[] {
  const invocations = new Map<string, { total: number; successful: number }>();
  for (const r of rows) {
    if (r.variant !== 'T') continue;
    if (r.outputKind !== 'tool_call') continue;
    if (!r.toolName) continue;
    const cur = invocations.get(r.toolName) ?? { total: 0, successful: 0 };
    cur.total += 1;
    if (r.m1_legal) cur.successful += 1;
    invocations.set(r.toolName, cur);
  }
  return [...invocations.entries()]
    .map(([tool, c]) => ({
      tool,
      invocations: c.total,
      successfulInvocations: c.successful,
    }))
    .sort((a, b) => b.invocations - a.invocations);
}

function rate<T>(items: T[], pred: (x: T) => boolean): number {
  if (items.length === 0) return Number.NaN;
  return items.filter(pred).length / items.length;
}

/**
 * Mean of a numeric field across rows, but only counting rows where
 * the field is > 0. The "skip zero" rule is what distinguishes
 * "unknown / not measured" from "actually used zero tokens" — a
 * harness error or codex CLI run reports 0 tokens used, which would
 * otherwise dilute the average. If no row has usage data, return NaN
 * so the report can render '—' instead of a misleading 0.
 */
function avgUsage<T>(items: T[], get: (x: T) => number): number {
  let sum = 0;
  let n = 0;
  for (const it of items) {
    const v = get(it);
    if (v > 0) {
      sum += v;
      n += 1;
    }
  }
  if (n === 0) return Number.NaN;
  return sum / n;
}

function delta(baseline: number, treatment: number): number {
  if (Number.isNaN(baseline) || Number.isNaN(treatment)) return Number.NaN;
  return (treatment - baseline) * 100;
}

function unique<T>(arr: T[]): T[] {
  const out: T[] = [];
  const seen = new Set<T>();
  for (const x of arr) {
    if (!seen.has(x)) {
      seen.add(x);
      out.push(x);
    }
  }
  return out;
}
