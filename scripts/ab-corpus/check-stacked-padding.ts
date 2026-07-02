/**
 * Empirical check: how common is "root has horizontal padding AND a
 * direct child also has horizontal padding" in real LLM output?
 *
 * Drives the cost-benefit on adding a `detect-stacked-horizontal-
 * padding` detector. The 2026-05-10 user-reported "Bistro" Mobile
 * design hit this pattern — root [0,16,0,16] + section [0,24] = 40px
 * effective gutter, which read as "too much padding". Need to know
 * if it's a one-off or a recurring AI-output shape before adding a
 * detector.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseModelOutput } from '@zseven-w/pen-ai-skills';
import { applyToFreshDoc } from './apply';

interface JsonlRow {
  promptId: string;
  category: string;
  difficulty: string;
  variant: string;
  rawOutput: string;
}

function getHorizontalPadding(p: unknown): { left: number; right: number } {
  if (typeof p === 'number') return { left: p, right: p };
  if (Array.isArray(p)) {
    if (p.length === 4) return { left: Number(p[3] ?? 0), right: Number(p[1] ?? 0) };
    if (p.length === 2) return { left: Number(p[1] ?? 0), right: Number(p[1] ?? 0) };
    if (p.length === 1) return { left: Number(p[0] ?? 0), right: Number(p[0] ?? 0) };
  }
  return { left: 0, right: 0 };
}

const FULL_BLEED_ROLES = new Set([
  'hero',
  'banner',
  'cover',
  'header',
  'top-nav',
  'bottom-nav',
  'status-bar',
  'tab-bar',
  'tabbar',
  'navbar',
]);

async function main(): Promise<void> {
  const runId = process.argv[2];
  if (!runId) {
    console.error('usage: bun run scripts/ab-corpus/check-stacked-padding.ts <run-id>');
    process.exit(1);
  }
  const path = join(import.meta.dir, 'runs', runId, 'scores.jsonl');
  const rows: JsonlRow[] = readFileSync(path, 'utf-8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));

  let applied = 0;
  let stacked = 0;
  const examples: string[] = [];

  for (const r of rows) {
    const parsed = parseModelOutput(r.rawOutput);
    if (parsed.kind === 'garbage') continue;
    const result = await applyToFreshDoc(parsed);
    if (!result.ok || !result.doc) continue;
    applied++;
    const root = result.doc.children?.[0] as
      | (Record<string, unknown> & { padding?: unknown; children?: unknown[] })
      | undefined;
    if (!root) continue;
    const rootP = getHorizontalPadding(root.padding);
    if (rootP.left === 0 && rootP.right === 0) continue;
    if (!Array.isArray(root.children)) continue;
    const offending: string[] = [];
    for (const child of root.children as Array<
      Record<string, unknown> & { id?: string; role?: string; padding?: unknown }
    >) {
      if (child?.type !== 'frame') continue;
      const role = String(child.role ?? '').toLowerCase();
      if (FULL_BLEED_ROLES.has(role)) continue;
      const p = getHorizontalPadding(child.padding);
      if (p.left > 0 || p.right > 0) offending.push(String(child.id ?? '?'));
    }
    if (offending.length > 0) {
      stacked++;
      if (examples.length < 8) {
        examples.push(
          `  ${r.promptId}/${r.variant}  root=${rootP.left}+${rootP.right}  offenders=${offending.length} (${offending.slice(0, 3).join(',')})`,
        );
      }
    }
  }

  console.log(`applied: ${applied}`);
  console.log(
    `stacked horizontal padding (root H>0 AND ≥1 child section H>0): ${stacked} (${((stacked / applied) * 100).toFixed(1)}%)`,
  );
  if (examples.length > 0) {
    console.log('\nexamples:');
    for (const e of examples) console.log(e);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
