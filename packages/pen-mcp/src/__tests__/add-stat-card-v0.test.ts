import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddStatCardV0 } from '../tools/add-stat-card-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-stat-card-v0');
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
function findByRole(n: Record<string, unknown>, role: string): Record<string, unknown> | undefined {
  if (n.role === role) return n;
  const kids = (n.children ?? []) as Record<string, unknown>[];
  for (const c of kids) {
    const hit = findByRole(c, role);
    if (hit) return hit;
  }
  return undefined;
}
function fillColor(n: Record<string, unknown>): string | undefined {
  const fills = n.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['s.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_stat_card_v0', () => {
  it('registered; required=[label, value]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_stat_card_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_stat_card_v0');
    expect(def?.inputSchema.required).toEqual(['label', 'value']);
  });

  it('minimal: label + value → header with label, big value, no delta, no icon', async () => {
    const fp = await fresh('s.op');
    await handleAddStatCardV0({ filePath: fp, label: 'Revenue', value: '$12k' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('stat-card');
    const label = findByRole(root, 'stat-card-label')!;
    expect(label.content).toBe('REVENUE'); // uppercased by the builder
    const value = findByRole(root, 'stat-card-value')!;
    expect(value.content).toBe('$12k');
    expect(value.fontSize).toBe(32);
    expect(findByRole(root, 'stat-card-delta')).toBeUndefined();
    expect(findByRole(root, 'stat-card-icon')).toBeUndefined();
  });

  it('with icon + delta + trend=up → emerald delta color', async () => {
    const fp = await fresh('s.op');
    await handleAddStatCardV0({
      filePath: fp,
      label: 'Orders',
      value: '1,284',
      icon: 'shopping-cart',
      delta: '+12% vs last week',
      trend: 'up',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'stat-card-icon')!.iconFontName).toBe('shopping-cart');
    const delta = findByRole(root, 'stat-card-delta')!;
    expect(delta.content).toBe('+12% vs last week');
    expect(fillColor(delta)).toBe('#10B981');
  });

  it('trend=down → red delta color', async () => {
    const fp = await fresh('s.op');
    await handleAddStatCardV0({
      filePath: fp,
      label: 'Churn',
      value: '3.2%',
      delta: '-0.4%',
      trend: 'down',
    });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(root, 'stat-card-delta')!)).toBe('#EF4444');
  });

  it('trend=flat (default) → slate delta color', async () => {
    const fp = await fresh('s.op');
    await handleAddStatCardV0({
      filePath: fp,
      label: 'Visitors',
      value: '1,024',
      delta: '±0%',
    });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(root, 'stat-card-delta')!)).toBe('#64748B');
  });

  it('width clamps (< 160 → 160)', async () => {
    const fp = await fresh('s.op');
    await handleAddStatCardV0({ filePath: fp, label: 'X', value: '1', width: 50 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(160);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('s.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddStatCardV0({ filePath: fp, label: 'X', value: '1', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
