/**
 * Print every issue of a specific category across an entire run, with the
 * row id, node id, and reason. Generic version of inspect-contrast-hits
 * — used for spot-checking any single detector's true-positive vs
 * false-positive rate against a real corpus.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  detectAllIssues,
  parseModelOutput,
  type Issue,
  type IssueCategory,
} from '@zseven-w/pen-ai-skills';
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
  const wantCat = process.argv[3] as IssueCategory | undefined;
  if (!runId || !wantCat) {
    console.error(
      'usage: bun run scripts/ab-corpus/inspect-issue-category.ts <run-id> <category>',
    );
    process.exit(1);
  }
  const path = join(import.meta.dir, 'runs', runId, 'scores.jsonl');
  const rows: JsonlRow[] = readFileSync(path, 'utf-8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));

  for (const r of rows) {
    const parsed = parseModelOutput(r.rawOutput);
    if (parsed.kind === 'garbage') continue;
    const result = await applyToFreshDoc(parsed);
    if (!result.ok || !result.doc) continue;
    const root = result.doc.children?.[0] ?? null;
    if (!root) continue;
    const issues: Issue[] = detectAllIssues(root, result.doc).filter((i) => i.category === wantCat);
    if (issues.length === 0) continue;
    console.log(`\n=== ${r.promptId} [${r.category}/${r.difficulty}/${r.variant}] (${issues.length} hits) ===`);
    for (const issue of issues) {
      console.log(`  ${issue.nodeId.padEnd(50)}  ${issue.reason}`);
    }
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
