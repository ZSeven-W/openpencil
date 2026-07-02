import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildEmptyChart, buildEmptyChartV1 } from '@zseven-w/pen-core';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddEmptyChartV1 } from '../tools/add-empty-chart-v1';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-empty-chart-v1');
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
function fillColor(n: Record<string, unknown> | undefined): string | undefined {
  const fills = n?.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}
function strokeColor(n: Record<string, unknown>): string | undefined {
  const stroke = n.stroke as { fill?: Array<{ color?: string }> } | undefined;
  return stroke?.fill?.[0]?.color;
}
function stripIds(n: unknown): unknown {
  if (Array.isArray(n)) return n.map(stripIds);
  if (n && typeof n === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(n as Record<string, unknown>)) {
      if (k === 'id') continue;
      out[k] = stripIds(v);
    }
    return out;
  }
  return n;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['e.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_empty_chart_v1', () => {
  it('registered; no required fields (all defaults)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_empty_chart_v1')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_empty_chart_v1');
    const req = def?.inputSchema.required as string[] | undefined;
    expect(req === undefined || req.length === 0).toBe(true);
  });

  it('schema exposes theme=["light","dark","system"]', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_empty_chart_v1');
    const props = def?.inputSchema.properties as Record<string, { enum?: unknown }> | undefined;
    expect(props?.theme?.enum).toEqual(['light', 'dark', 'system']);
  });

  it('default theme=light → slate-50 bg + slate-300 dashed border (v0 parity colors)', async () => {
    const fp = await fresh('e.op');
    await handleAddEmptyChartV1({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('#F8FAFC');
    expect(strokeColor(root)).toBe('#CBD5E1');
    expect(fillColor(findByRole(root, 'empty-chart-title'))).toBe('#334155');
  });

  it('theme=light is byte-parity with buildEmptyChart v0 (modulo ids)', () => {
    const args = { width: 320, height: 200, title: 'Empty', icon: 'line-chart' };
    const v0 = stripIds(buildEmptyChart(args));
    const v1 = stripIds(buildEmptyChartV1({ ...args, theme: 'light' }));
    expect(v1).toEqual(v0);
  });

  it('theme=dark → slate-800 bg, slate-200 title, slate-400 subtitle', async () => {
    const fp = await fresh('e.op');
    await handleAddEmptyChartV1({ filePath: fp, theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('#1E293B');
    expect(strokeColor(root)).toBe('#475569');
    expect(fillColor(findByRole(root, 'empty-chart-title'))).toBe('#E2E8F0');
    expect(fillColor(findByRole(root, 'empty-chart-subtitle'))).toBe('#94A3B8');
  });

  it('theme=system → $color-* refs for bg + border + title + subtitle', async () => {
    const fp = await fresh('e.op');
    await handleAddEmptyChartV1({ filePath: fp, theme: 'system' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('$color-surface-2');
    expect(strokeColor(root)).toBe('$color-border');
    expect(fillColor(findByRole(root, 'empty-chart-title'))).toBe('$color-text-primary');
    expect(fillColor(findByRole(root, 'empty-chart-subtitle'))).toBe('$color-text-muted');
  });

  it('icon override works', async () => {
    const fp = await fresh('e.op');
    await handleAddEmptyChartV1({ filePath: fp, icon: 'pie-chart', theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'empty-chart-icon')!.iconFontName).toBe('pie-chart');
  });

  it('dashed stroke preserved across themes (strokeDashArray=[4,4])', async () => {
    const fp = await fresh('e.op');
    await handleAddEmptyChartV1({ filePath: fp, theme: 'system' });
    const root = getRoot(await readDoc(fp));
    const stroke = root.stroke as { strokeDashArray?: number[] };
    expect(stroke.strokeDashArray).toEqual([4, 4]);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('e.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddEmptyChartV1({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
