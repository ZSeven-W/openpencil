/**
 * For a given run, list every text-bg-contrast hit with the failing
 * text color, resolved bg color, and ratio. Helps eyeball whether the
 * detector is catching real contrast violations or over-firing because
 * the apply-to-fresh-doc harness has no page-level fill.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { detectTextBgContrast, parseModelOutput, type Issue } from '@zseven-w/pen-ai-skills';
import { applyToFreshDoc } from './apply';

interface JsonlRow {
  promptId: string;
  category: string;
  difficulty: string;
  variant: string;
  rawOutput: string;
}

async function main(): Promise<void> {
  const runId = process.argv[2];
  const limit = Number(process.argv[3] ?? 50);
  if (!runId) {
    console.error('usage: bun run scripts/ab-corpus/inspect-contrast-hits.ts <run-id> [limit]');
    process.exit(1);
  }
  const path = join(import.meta.dir, 'runs', runId, 'scores.jsonl');
  const lines = readFileSync(path, 'utf-8').split('\n').filter(Boolean);
  const rows: JsonlRow[] = lines.map((l) => JSON.parse(l));

  let shown = 0;
  for (const r of rows) {
    if (shown >= limit) break;
    const parsed = parseModelOutput(r.rawOutput);
    if (parsed.kind === 'garbage') continue;
    const result = await applyToFreshDoc(parsed);
    if (!result.ok || !result.doc) continue;
    const root = result.doc.children?.[0] ?? null;
    if (!root) continue;
    const issues: Issue[] = detectTextBgContrast(root, result.doc);
    if (issues.length === 0) continue;
    console.log(
      `\n=== ${r.promptId} [${r.category}/${r.difficulty}/${r.variant}] (${issues.length} hits) ===`,
    );
    for (const issue of issues) {
      // reason looks like: "text/bg contrast 3.05:1 below WCAG AA 4.5:1 (text=#... on bg=#...)"
      console.log(`  ${issue.nodeId.padEnd(50)}  ${issue.reason}`);
    }
    shown++;
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
