import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddEmptyChartV0 } from '../tools/add-empty-chart-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-empty-chart-v0');
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

describe('add_empty_chart_v0', () => {
  it('registered; required=[] (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_empty_chart_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_empty_chart_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('defaults: 320×200, bar-chart-2 icon, "No data yet" title', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyChartV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('empty-chart');
    expect(root.width).toBe(320);
    expect(root.height).toBe(200);
    const kids = root.children as Record<string, unknown>[];
    const icon = kids.find((k) => k.role === 'empty-chart-icon')!;
    expect(icon.iconFontName).toBe('bar-chart-2');
    const title = kids.find((k) => k.role === 'empty-chart-title')!;
    expect(title.content).toBe('No data yet');
  });

  it('dashed stroke (visual signal that the slot is empty)', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyChartV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const stroke = root.stroke as { strokeDashArray?: number[] };
    expect(stroke.strokeDashArray).toEqual([4, 4]);
  });

  it('icon override: line-chart for line-chart slot', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyChartV0({ filePath: fp, icon: 'line-chart' });
    const root = getRoot(await readDoc(fp));
    const icon = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'empty-chart-icon',
    )!;
    expect(icon.iconFontName).toBe('line-chart');
  });

  it('size clamps (width < 120 → 120, height < 100 → 100)', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyChartV0({ filePath: fp, width: 50, height: 30 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(120);
    expect(root.height).toBe(100);
  });

  it('custom title + subtitle', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyChartV0({
      filePath: fp,
      title: 'Come back later',
      subtitle: 'Data populates after 24h',
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    expect(kids.find((k) => k.role === 'empty-chart-title')!.content).toBe('Come back later');
    expect(kids.find((k) => k.role === 'empty-chart-subtitle')!.content).toBe(
      'Data populates after 24h',
    );
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddEmptyChartV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
