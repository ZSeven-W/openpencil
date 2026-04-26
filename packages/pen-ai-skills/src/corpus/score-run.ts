import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import { detectAllIssues } from '../diagnostics/detectors';
import type { ApplyFn, CorpusPrompt, ParsedOutput, ScoreRow } from './types';

/**
 * Walk a PenNode tree and tally `role` occurrences. Roles are optional
 * on PenNode so we skip nodes without one. Used for expected-shape
 * checks (must_contain_roles / min_roles).
 */
export function countRoles(doc: PenDocument): Record<string, number> {
  const counts: Record<string, number> = {};
  const walk = (node: PenNode): void => {
    const role = (node as { role?: string }).role;
    if (typeof role === 'string' && role.length > 0) {
      counts[role] = (counts[role] ?? 0) + 1;
    }
    const kids = (node as { children?: unknown }).children;
    if (Array.isArray(kids)) {
      for (const k of kids) {
        if (k && typeof k === 'object') walk(k as PenNode);
      }
    }
  };
  const topLevel: PenNode[] =
    (doc.children as PenNode[] | undefined) ??
    (doc.pages?.[0]?.children as PenNode[] | undefined) ??
    [];
  for (const n of topLevel) walk(n);
  return counts;
}

/**
 * Evaluate a single model output against a corpus prompt. Runs the
 * caller-supplied `apply` (which in turn calls into pen-mcp), then
 * layers legality + expected-shape checks on top.
 *
 * M1 legality = apply returned ok AND zero `severity: 'error'` issues
 *               from detectAllIssues. (All current detectors emit
 *               'warning', so M1 today is essentially "apply ok".
 *               When detectors add error-severity classes M1 tightens
 *               automatically with no API change.)
 *
 * M3 success  = M1 passes AND every `expected.must_contain_roles` role
 *               appears at least once AND every `expected.min_roles`
 *               entry meets its threshold.
 */
export async function scoreRun(args: {
  prompt: CorpusPrompt;
  parsed: ParsedOutput;
  apply: ApplyFn;
  model: string;
  variant: 'B' | 'T';
}): Promise<ScoreRow> {
  const { prompt, parsed, apply, model, variant } = args;
  const toolName = parsed.kind === 'tool_call' ? parsed.name : '';
  const base = {
    promptId: prompt.id,
    category: prompt.category,
    difficulty: prompt.difficulty,
    model,
    variant,
    outputKind: parsed.kind,
    toolName,
    expectedToolIfAny: prompt.expected_tool_if_any ?? '',
    routing: classifyRouting(prompt, variant, parsed.kind, toolName),
    rawOutput: parsed.raw,
  };

  if (parsed.kind === 'garbage') {
    return {
      ...base,
      m1_legal: false,
      m3_success: false,
      m3_failure_reason: `output unparseable: ${parsed.reason}`,
      issues: [],
      applyError: '',
    };
  }

  const applied = await apply(parsed);
  if (!applied.ok || !applied.doc) {
    return {
      ...base,
      m1_legal: false,
      m3_success: false,
      m3_failure_reason: 'apply failed before shape checks',
      issues: [],
      applyError: applied.error,
    };
  }

  const errorIssues: ReturnType<typeof detectAllIssues> = [];
  const allIssues: ReturnType<typeof detectAllIssues> = [];
  const topLevel = (applied.doc.children ?? applied.doc.pages?.[0]?.children ?? []) as PenNode[];
  for (const root of topLevel) {
    const found = detectAllIssues(root, applied.doc);
    for (const f of found) {
      allIssues.push(f);
      if (f.severity === 'error') errorIssues.push(f);
    }
  }

  const m1 = errorIssues.length === 0;
  const shape = checkExpectedShape(prompt, applied.doc);

  return {
    ...base,
    m1_legal: m1,
    m3_success: m1 && shape.ok,
    m3_failure_reason: m1 ? shape.reason : 'M1 failed — skipped shape checks',
    issues: allIssues,
    applyError: '',
  };
}

/**
 * Classify a run's routing verdict. Only meaningful on
 * `difficulty: 'obvious'` treatment runs — baseline and optional both
 * map to 'n/a' because the concept "right tool vs wrong tool" isn't
 * defined there. Used by aggregate to split routing into 4 categories
 * (right-tool / wrong-tool / fallback / garbage) that sum to 100% of
 * the obvious-treatment subset.
 *
 * Garbage is counted as its own category rather than 'n/a' because
 * otherwise a model that emits 10 unparseable outputs + 1 right + 1
 * wrong would score 50% right-tool — hiding the fact that only 2/12
 * runs were actually usable. With garbage in the denominator, the
 * right-tool rate stays interpretable on its own.
 */
function classifyRouting(
  prompt: CorpusPrompt,
  variant: 'B' | 'T',
  kind: ParsedOutput['kind'],
  toolName: string,
): ScoreRow['routing'] {
  if (variant !== 'T') return 'n/a';
  if (prompt.difficulty !== 'obvious') return 'n/a';
  if (kind === 'garbage') return 'garbage';
  if (kind === 'batch_design') return 'fallback';
  // kind === 'tool_call'
  return toolName === prompt.expected_tool_if_any ? 'right-tool' : 'wrong-tool';
}

function checkExpectedShape(
  prompt: CorpusPrompt,
  doc: PenDocument,
): { ok: boolean; reason: string } {
  const counts = countRoles(doc);
  const missing: string[] = [];
  for (const role of prompt.expected.must_contain_roles ?? []) {
    if ((counts[role] ?? 0) === 0) missing.push(role);
  }
  if (missing.length > 0) {
    return { ok: false, reason: `missing required role(s): ${missing.join(', ')}` };
  }
  const underMin: string[] = [];
  for (const [role, min] of Object.entries(prompt.expected.min_roles ?? {})) {
    if ((counts[role] ?? 0) < min) {
      underMin.push(`${role}<${min} (got ${counts[role] ?? 0})`);
    }
  }
  if (underMin.length > 0) {
    return { ok: false, reason: `role counts below minimum: ${underMin.join(', ')}` };
  }
  return { ok: true, reason: '' };
}
