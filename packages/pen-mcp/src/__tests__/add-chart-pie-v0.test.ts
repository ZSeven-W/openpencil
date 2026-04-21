import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddChartPieV0 } from '../tools/add-chart-pie-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-chart-pie-v0');
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

describe('add_chart_pie_v0', () => {
  it('registered + required values', () => {
    expect(DESIGN_TOOL_NAMES.has('add_chart_pie_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_chart_pie_v0');
    expect(def?.inputSchema.required).toEqual(['values']);
  });

  it('4 equal values → 4 slices at 90° each, default palette', async () => {
    const fp = await fresh('a.op');
    await handleAddChartPieV0({ filePath: fp, values: [1, 1, 1, 1] });
    const chart = getRoot(await readDoc(fp));
    expect(chart.role).toBe('chart-pie');
    expect(chart.width).toBe(160);
    expect(chart.height).toBe(160);
    const slices = chart.children as Record<string, unknown>[];
    expect(slices.length).toBe(4);
    for (const s of slices) {
      expect(s.type).toBe('ellipse');
      expect(s.role).toBe('chart-pie-slice');
      expect(s.sweepAngle).toBeCloseTo(90, 5);
    }
    // Angles accumulate from -90 (12 o'clock)
    expect(slices[0].startAngle).toBeCloseTo(-90, 5);
    expect(slices[1].startAngle).toBeCloseTo(0, 5);
    expect(slices[2].startAngle).toBeCloseTo(90, 5);
    expect(slices[3].startAngle).toBeCloseTo(180, 5);
  });

  it('unequal values normalize to 360° total', async () => {
    const fp = await fresh('a.op');
    await handleAddChartPieV0({ filePath: fp, values: [40, 30, 20, 10] }); // sums to 100
    const slices = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const sweeps = slices.map((s) => s.sweepAngle as number);
    expect(sweeps[0]).toBeCloseTo(144, 1); // 40%
    expect(sweeps[1]).toBeCloseTo(108, 1); // 30%
    expect(sweeps[2]).toBeCloseTo(72, 1); // 20%
    expect(sweeps[3]).toBeCloseTo(36, 1); // 10%
    const total = sweeps.reduce((s, v) => s + v, 0);
    expect(total).toBeCloseTo(360, 1);
  });

  it('custom colors override palette (with palette fallback for overflow)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartPieV0({
      filePath: fp,
      values: [1, 1, 1, 1],
      colors: ['#FF0000', '#00FF00'], // only 2 of 4 provided
    });
    const slices = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const fills = slices.map(
      (s) => ((s.fill as Array<{ color: string }>) ?? [{ color: '' }])[0].color,
    );
    expect(fills[0]).toBe('#FF0000');
    expect(fills[1]).toBe('#00FF00');
    // slices 2 & 3 fall back to default palette
    expect(fills[2]).toMatch(/^#[0-9A-F]{6}$/i);
    expect(fills[3]).toMatch(/^#[0-9A-F]{6}$/i);
  });

  it('diameter respected; clamped to >= 40', async () => {
    const fp = await fresh('a.op');
    await handleAddChartPieV0({ filePath: fp, values: [1, 1], diameter: 200 });
    let c = getRoot(await readDoc(fp));
    expect(c.width).toBe(200);
    expect(c.height).toBe(200);

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddChartPieV0({ filePath: fp, values: [1, 1], diameter: 10 });
    c = getRoot(await readDoc(fp));
    expect(c.width).toBe(40); // clamped
  });

  it('inner_radius_ratio > 0 → donut (innerRadius stored as ratio 0..1, NOT pixels)', async () => {
    // Renderer interprets EllipseNode.innerRadius as a ratio multiplied
    // by rx at paint time. Storing pixels here would blow past the
    // outer radius and clip every slice.
    const fp = await fresh('a.op');
    await handleAddChartPieV0({
      filePath: fp,
      values: [1, 1, 1],
      diameter: 160,
      inner_radius_ratio: 0.5,
    });
    const slices = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    for (const s of slices) {
      expect(s.innerRadius).toBe(0.5);
    }
  });

  it('inner_radius_ratio is invariant across diameter (ratio, not pixels)', async () => {
    const fp = await fresh('a.op');
    await handleAddChartPieV0({
      filePath: fp,
      values: [1, 1],
      diameter: 80,
      inner_radius_ratio: 0.6,
    });
    const slicesSmall = getRoot(await readDoc(fp)).children as Record<string, unknown>[];

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddChartPieV0({
      filePath: fp,
      values: [1, 1],
      diameter: 300,
      inner_radius_ratio: 0.6,
    });
    const slicesBig = getRoot(await readDoc(fp)).children as Record<string, unknown>[];

    // Same ratio regardless of diameter — pixels-based storage would diverge.
    expect(slicesSmall[0].innerRadius).toBe(0.6);
    expect(slicesBig[0].innerRadius).toBe(0.6);
  });

  it('all-zero values throws (degenerate chart)', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddChartPieV0({ filePath: fp, values: [0, 0, 0] })).rejects.toThrow(
      /sum to > 0/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });

  it('empty values throws', async () => {
    const fp = await fresh('a.op');
    await expect(handleAddChartPieV0({ filePath: fp, values: [] })).rejects.toThrow(/must contain/);
  });
});
