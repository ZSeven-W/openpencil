/**
 * Dump the doc shape (root width / height / layout / direct-children
 * roles) for every applied row in a category. Used 2026-05-10 to figure
 * out why detectEdgeSectionPadding scored 0 hits on the 220-row mobile
 * subset of `2026-05-03-ab-v8-v1-default` — needed to see whether the
 * model is producing page-shaped roots at all or just fragments that
 * don't look like a mobile page.
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

async function main(): Promise<void> {
  const runId = process.argv[2];
  const wantCategory = process.argv[3] ?? 'mobile';
  if (!runId) {
    console.error('usage: bun run scripts/ab-corpus/inspect-shape.ts <run-id> [category]');
    process.exit(1);
  }
  const path = join(import.meta.dir, 'runs', runId, 'scores.jsonl');
  const rows: JsonlRow[] = readFileSync(path, 'utf-8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));

  const buckets = new Map<string, number>();
  for (const r of rows) {
    if (r.category !== wantCategory) continue;
    const parsed = parseModelOutput(r.rawOutput);
    if (parsed.kind === 'garbage') continue;
    const result = await applyToFreshDoc(parsed);
    if (!result.ok || !result.doc) continue;
    const root = result.doc.children?.[0] as
      | (Record<string, unknown> & { width?: unknown; height?: unknown; children?: unknown[] })
      | undefined;
    if (!root) continue;
    const w = root.width;
    const h = root.height;
    const childCount = Array.isArray(root.children) ? root.children.length : 0;
    const wKey =
      typeof w === 'number'
        ? w >= 320 && w <= 480
          ? 'mobile-w'
          : w > 480 && w <= 800
            ? 'tablet-w'
            : w > 800
              ? 'desktop-w'
              : 'tiny-w'
        : `non-numeric:${typeof w}`;
    const hKey =
      typeof h === 'number'
        ? h >= 568
          ? 'tall'
          : 'short'
        : `non-numeric:${typeof h}`;
    const ratioKey =
      typeof w === 'number' && typeof h === 'number' && w > 0 && h / w >= 1.5 ? 'aspect-ok' : 'aspect-low';
    const key = `${wKey} / ${hKey} / ${ratioKey} / children=${childCount}`;
    buckets.set(key, (buckets.get(key) ?? 0) + 1);
  }

  console.log(`shapes for category=${wantCategory}:`);
  const sorted = Array.from(buckets.entries()).sort((a, b) => b[1] - a[1]);
  for (const [k, v] of sorted) console.log(`  ${String(v).padStart(4)}  ${k}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
