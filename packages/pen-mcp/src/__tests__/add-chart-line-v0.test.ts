import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddChartLineV0 } from '../tools/add-chart-line-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-chart-line-v0');
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

describe('add_chart_line_v0', () => {
  it('registered + required values', () => {
    expect(DESIGN_TOOL_NAMES.has('add_chart_line_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_chart_line_v0');
    expect(def?.inputSchema.required).toEqual(['values']);
  });

  it('default: path + N dots, total width = N × 32, height 160', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [1, 3, 2, 5, 4, 6] });
    const chart = getRoot(await readDoc(fp));
    expect(chart.role).toBe('chart-line');
    expect(chart.width).toBe(32 * 6);
    expect(chart.height).toBe(160);
    const kids = chart.children as Record<string, unknown>[];
    // 1 path + 6 dots
    expect(kids.length).toBe(7);
    expect(kids[0].type).toBe('path');
    expect(kids[0].role).toBe('chart-line-path');
    // Path d starts with M
    expect(kids[0].d).toMatch(/^M /);
    // Dots
    for (let i = 1; i < 7; i++) {
      expect(kids[i].type).toBe('ellipse');
      expect(kids[i].role).toBe('chart-line-dot');
      expect(kids[i].width).toBe(8);
      expect(kids[i].height).toBe(8);
    }
  });

  it('dots=false skips the dot ellipses', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [1, 2, 3], dots: false });
    const chart = getRoot(await readDoc(fp));
    const kids = chart.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('path');
  });

  it('point_spacing controls per-point width (clamped to >=8)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [1, 2, 3], point_spacing: 48 });
    let c = getRoot(await readDoc(fp));
    expect(c.width).toBe(48 * 3);

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddChartLineV0({ filePath: fp, values: [1, 2, 3], point_spacing: 2 });
    c = getRoot(await readDoc(fp));
    expect(c.width).toBe(8 * 3); // clamped
  });

  it('custom stroke_color applied to path + dots', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [1, 2], stroke_color: '#EF4444' });
    const chart = getRoot(await readDoc(fp));
    const kids = chart.children as Record<string, unknown>[];
    const pathStroke = (kids[0] as { stroke?: { fill?: Array<{ color: string }> } }).stroke;
    expect(pathStroke?.fill?.[0].color).toBe('#EF4444');
    const dotFill = (kids[1] as { fill?: Array<{ color: string }> }).fill;
    expect(dotFill?.[0].color).toBe('#EF4444');
  });

  it('all-zero values still emits path + dots (no divide-by-zero)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [0, 0, 0] });
    const chart = getRoot(await readDoc(fp));
    const kids = chart.children as Record<string, unknown>[];
    // 1 path + 3 dots
    expect(kids.length).toBe(4);
  });

  it('empty values throws', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddChartLineV0({ filePath: fp, values: [] })).rejects.toThrow(
      /values must contain/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddChartLineV0({ filePath: fp, values: [1, 2, 3] });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // 1 wrapper + 1 path + 3 dots = 5
    expect(ids.length).toBe(5);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
