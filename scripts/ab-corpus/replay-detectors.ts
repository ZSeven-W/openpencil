/**
 * Replay an existing scores.jsonl through the CURRENT detector set.
 *
 * 2026-05-10 — used to validate the post-2026-05-08 detector additions
 * (detectEdgeSectionPadding + detectTextBgContrast) against real GPT-5.5
 * output without burning fresh API tokens. Reads each row's rawOutput,
 * re-applies it to a fresh doc, runs detectAllIssues with today's 13
 * detectors, and reports per-category counts so we can spot detectors
 * that now over-fire (false-positive prone) or stay silent.
 *
 * Usage: bun run scripts/ab-corpus/replay-detectors.ts <run-id>
 */
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { detectAllIssues, parseModelOutput, type Issue } from '@zseven-w/pen-ai-skills';
import { applyToFreshDoc } from './apply';

interface JsonlRow {
  promptId: string;
  category: string;
  difficulty: string;
  model: string;
  variant: string;
  rawOutput: string;
  issues: Issue[];
}

async function main(): Promise<void> {
  const runId = process.argv[2];
  if (!runId) {
    console.error('usage: bun run scripts/ab-corpus/replay-detectors.ts <run-id>');
    process.exit(1);
  }
  const path = join(import.meta.dir, 'runs', runId, 'scores.jsonl');
  if (!existsSync(path)) {
    console.error(`scores.jsonl not found: ${path}`);
    process.exit(1);
  }

  const lines = readFileSync(path, 'utf-8').split('\n').filter(Boolean);
  const rows: JsonlRow[] = lines.map((l) => JSON.parse(l));
  console.log(`replay: ${rows.length} rows from ${runId}`);

  let appliedOk = 0;
  let appliedErr = 0;
  const oldByCategory = new Map<string, number>();
  const newByCategory = new Map<string, number>();
  const sampleByCategory = new Map<string, JsonlRow[]>();

  for (const r of rows) {
    for (const issue of r.issues ?? []) {
      oldByCategory.set(issue.category, (oldByCategory.get(issue.category) ?? 0) + 1);
    }
  }

  for (const r of rows) {
    const parsed = parseModelOutput(r.rawOutput);
    if (parsed.kind === 'garbage') {
      appliedErr++;
      continue;
    }
    const result = await applyToFreshDoc(parsed);
    if (!result.ok || !result.doc) {
      appliedErr++;
      continue;
    }
    appliedOk++;

    const root = result.doc.children?.[0] ?? null;
    if (!root) continue;
    const issues = detectAllIssues(root, result.doc);
    for (const issue of issues) {
      newByCategory.set(issue.category, (newByCategory.get(issue.category) ?? 0) + 1);
      const samples = sampleByCategory.get(issue.category) ?? [];
      if (samples.length < 3) {
        samples.push(r);
        sampleByCategory.set(issue.category, samples);
      }
    }
  }

  console.log(`\napplied: ok=${appliedOk} err=${appliedErr}`);
  const allCategories = new Set([...oldByCategory.keys(), ...newByCategory.keys()]);
  console.log('\nper-category counts (old → new):');
  console.log('category                          old    new   delta');
  console.log('--------------------------------- ----- ----- -------');
  const sorted = Array.from(allCategories).sort();
  for (const cat of sorted) {
    const o = oldByCategory.get(cat) ?? 0;
    const n = newByCategory.get(cat) ?? 0;
    const delta = n - o;
    const sign = delta > 0 ? '+' : '';
    console.log(
      `${cat.padEnd(33)} ${String(o).padStart(5)} ${String(n).padStart(5)} ${sign}${String(delta).padStart(5)}`,
    );
  }

  const newCats = Array.from(newByCategory.keys()).filter((c) => !oldByCategory.has(c));
  if (newCats.length > 0) {
    console.log('\nfirst-time-firing detectors (worth spot-checking):');
    for (const cat of newCats) {
      console.log(`\n  ${cat} (${newByCategory.get(cat)} hits)`);
      for (const sample of sampleByCategory.get(cat) ?? []) {
        console.log(`    - ${sample.promptId} [${sample.category}/${sample.difficulty}]`);
      }
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
