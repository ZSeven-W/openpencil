import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddMetricComparisonV0 } from '../tools/add-metric-comparison-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-metric-comparison-v0');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}
async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}
function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const top = doc['children'] as Record<string, unknown>[] | undefined;
  const root = (top ?? pages?.[0]?.children)?.[0];
  if (!root) throw new Error('no root');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['a.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_metric_comparison_v0', () => {
  it('registered; required label + value', () => {
    expect(DESIGN_TOOL_NAMES.has('add_metric_comparison_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_metric_comparison_v0');
    expect(def?.inputSchema.required).toEqual(['label', 'value']);
  });

  it('no change → label + value only (no arrow)', async () => {
    const fp = await fresh('a.op');
    await handleAddMetricComparisonV0({ filePath: fp, label: 'Revenue', value: '$12k' });
    const m = getRoot(await readDoc(fp));
    expect(m.role).toBe('metric-comparison');
    const kids = m.children as Record<string, unknown>[];
    expect(kids.length).toBe(2); // label + value row
    const valueRow = kids[1];
    const vrKids = valueRow.children as Record<string, unknown>[];
    expect(vrKids.length).toBe(1); // only value, no change
    expect(vrKids[0].content).toBe('$12k');
  });

  it('trend=up → emerald + trending-up icon', async () => {
    const fp = await fresh('a.op');
    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Revenue',
      value: '$12k',
      change: '8%',
      trend: 'up',
    });
    const valueRow = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const change = (valueRow.children as Record<string, unknown>[])[1];
    const changeKids = change.children as Record<string, unknown>[];
    expect(changeKids[0].iconFontName).toBe('trending-up');
    const iconFill = changeKids[0].fill as Array<{ color: string }>;
    expect(iconFill[0].color).toBe('#10B981');
  });

  it('trend=down → red + trending-down', async () => {
    const fp = await fresh('a.op');
    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Churn',
      value: '1.2%',
      change: '3%',
      trend: 'down',
    });
    const valueRow = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const change = (valueRow.children as Record<string, unknown>[])[1];
    const changeKids = change.children as Record<string, unknown>[];
    expect(changeKids[0].iconFontName).toBe('trending-down');
    const iconFill = changeKids[0].fill as Array<{ color: string }>;
    expect(iconFill[0].color).toBe('#EF4444');
  });

  it('trend=flat (default) → slate + minus', async () => {
    const fp = await fresh('a.op');
    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Neutral',
      value: '100',
      change: '0%',
    });
    const valueRow = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const change = (valueRow.children as Record<string, unknown>[])[1];
    const changeKids = change.children as Record<string, unknown>[];
    expect(changeKids[0].iconFontName).toBe('minus');
    const iconFill = changeKids[0].fill as Array<{ color: string }>;
    expect(iconFill[0].color).toBe('#64748B');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddMetricComparisonV0({
        filePath: fp,
        label: 'X',
        value: '1',
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
