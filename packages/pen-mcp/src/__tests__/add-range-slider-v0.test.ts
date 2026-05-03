import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddRangeSliderV0 } from '../tools/add-range-slider-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-range-slider-v0');
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

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['r.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_range_slider_v0', () => {
  it('registered; no required fields (all inputs have defaults)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_range_slider_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_range_slider_v0');
    // `required` may be undefined or an empty array for this tool
    const req = def?.inputSchema.required as string[] | undefined;
    expect(req === undefined || req.length === 0).toBe(true);
  });

  it('defaults: value=50 / width=320 → fill + thumb + remaining all present', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('range-slider');
    expect(findByRole(root, 'range-slider-fill')).toBeDefined();
    expect(findByRole(root, 'range-slider-thumb')).toBeDefined();
    expect(findByRole(root, 'range-slider-remaining')).toBeDefined();
  });

  it('value=0 → no fill rect (only thumb + remaining)', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp, value: 0 });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'range-slider-fill')).toBeUndefined();
    expect(findByRole(root, 'range-slider-thumb')).toBeDefined();
    expect(findByRole(root, 'range-slider-remaining')).toBeDefined();
  });

  it('value=100 → no remaining rect (only fill + thumb)', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp, value: 100 });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'range-slider-fill')).toBeDefined();
    expect(findByRole(root, 'range-slider-thumb')).toBeDefined();
    expect(findByRole(root, 'range-slider-remaining')).toBeUndefined();
  });

  it('fill width matches value% math: value=25, width=320 → fill=(320-20)*0.25=75', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp, value: 25, width: 320 });
    const root = getRoot(await readDoc(fp));
    const fill = findByRole(root, 'range-slider-fill')!;
    expect(fill.width).toBe(75);
  });

  it('custom min/max (0..255) → value pegged correctly', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({
      filePath: fp,
      min: 0,
      max: 255,
      value: 128,
      width: 320,
      show_value: true,
    });
    const root = getRoot(await readDoc(fp));
    // show_value renders as "128" (rounded) with no suffix
    expect(findByRole(root, 'range-slider-value')!.content).toBe('128');
  });

  it('show_value + value_suffix renders "60%" in header', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({
      filePath: fp,
      value: 60,
      label: 'Volume',
      show_value: true,
      value_suffix: '%',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'range-slider-label')!.content).toBe('Volume');
    expect(findByRole(root, 'range-slider-value')!.content).toBe('60%');
  });

  it('no label, no show_value → no header rendered', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp, value: 50 });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'range-slider-header')).toBeUndefined();
  });

  it('value clamps outside [min,max]', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({
      filePath: fp,
      value: 999,
      min: 0,
      max: 100,
      show_value: true,
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'range-slider-value')!.content).toBe('100');
  });

  it('width clamps (< 160 → 160)', async () => {
    const fp = await fresh('r.op');
    await handleAddRangeSliderV0({ filePath: fp, width: 50 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(160);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('r.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddRangeSliderV0({ filePath: fp, value: 50, parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
