import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddChartBarsV0 } from '../tools/add-chart-bars-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-chart-bars-v0');
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

describe('add_chart_bars_v0', () => {
  it('registered + required values', () => {
    expect(DESIGN_TOOL_NAMES.has('add_chart_bars_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_chart_bars_v0');
    expect(def?.inputSchema.required).toEqual(['values']);
  });

  it('heights scale to max(values)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartBarsV0({ filePath: fp, values: [1, 2, 4], chart_height: 100 });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('chart-bars');
    expect(root.height).toBe(100);
    expect(root.layout).toBe('horizontal');
    expect(root.alignItems).toBe('flex-end');
    const kids = root.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    // value=4 is the max → full 100 height
    expect(kids[2].height).toBe(100);
    // value=2 → 50, value=1 → 25
    expect(kids[1].height).toBe(50);
    expect(kids[0].height).toBe(25);
    expect(kids[0].role).toBe('chart-bar');
  });

  it('zero values get 2px floor (never collapse)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartBarsV0({ filePath: fp, values: [0, 0, 5], chart_height: 80 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[0].height).toBe(2);
    expect(kids[1].height).toBe(2);
    expect(kids[2].height).toBe(80);
  });

  it('negative + non-finite values clamp to 0 → 2px floor', async () => {
    const fp = await fresh('a.op');
    await handleAddChartBarsV0({
      filePath: fp,
      values: [-5, Number.NaN, 10],
      chart_height: 100,
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[0].height).toBe(2);
    expect(kids[1].height).toBe(2);
    expect(kids[2].height).toBe(100);
  });

  it('empty values throws', async () => {
    const fp = await fresh('a.op');
    await expect(handleAddChartBarsV0({ filePath: fp, values: [] })).rejects.toThrow(
      /at least one number/,
    );
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddChartBarsV0({ filePath: fp, values: [1, 2], parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
