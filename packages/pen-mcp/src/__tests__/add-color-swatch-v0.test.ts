import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddColorSwatchV0 } from '../tools/add-color-swatch-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-color-swatch-v0');
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

describe('add_color_swatch_v0', () => {
  it('registered + required color', () => {
    expect(DESIGN_TOOL_NAMES.has('add_color_swatch_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_color_swatch_v0');
    expect(def?.inputSchema.required).toEqual(['color']);
  });

  it('hex color + label: 2 children, square fill is hex', async () => {
    const fp = await fresh('a.op');
    await handleAddColorSwatchV0({ filePath: fp, color: '#2563EB', label: 'Primary' });
    const swatch = getRoot(await readDoc(fp));
    expect(swatch.role).toBe('color-swatch');
    expect(swatch.layout).toBe('vertical');
    const kids = swatch.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].role).toBe('color-swatch-square');
    expect(kids[0].width).toBe(64);
    expect(kids[0].cornerRadius).toBe(12);
    const fill = (kids[0].fill as Array<{ color: string }>)[0];
    expect(fill.color).toBe('#2563EB');
    expect(kids[1].role).toBe('color-swatch-label');
    expect(kids[1].content).toBe('Primary');
  });

  it('accepts $variable ref as color', async () => {
    const fp = await fresh('a.op');
    await handleAddColorSwatchV0({ filePath: fp, color: '$color-primary' });
    const square = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const fill = (square.fill as Array<{ color: string }>)[0];
    expect(fill.color).toBe('$color-primary');
  });

  it('no label: only square', async () => {
    const fp = await fresh('a.op');
    await handleAddColorSwatchV0({ filePath: fp, color: '#fff' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
  });

  it('custom size', async () => {
    const fp = await fresh('a.op');
    await handleAddColorSwatchV0({ filePath: fp, color: '#000', size: 96 });
    const square = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(square.width).toBe(96);
    expect(square.height).toBe(96);
  });
});
