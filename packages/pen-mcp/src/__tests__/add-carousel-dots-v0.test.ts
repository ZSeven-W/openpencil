import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCarouselDotsV0 } from '../tools/add-carousel-dots-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-carousel-dots-v0');
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

describe('add_carousel_dots_v0', () => {
  it('registered + required total', () => {
    expect(DESIGN_TOOL_NAMES.has('add_carousel_dots_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_carousel_dots_v0');
    expect(def?.inputSchema.required).toEqual(['total']);
  });

  it('5 dots, current=2: 4 circles + 1 pill at index 2', async () => {
    const fp = await fresh('a.op');
    await handleAddCarouselDotsV0({ filePath: fp, total: 5, current: 2 });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('carousel-dots');
    const kids = row.children as Record<string, unknown>[];
    expect(kids.length).toBe(5);
    expect(kids[2].role).toBe('dot-active');
    expect(kids[2].width).toBe(16);
    expect(kids[2].height).toBe(6);
    expect(kids[2].cornerRadius).toBe(3);
    expect(kids[0].role).toBe('dot');
    expect(kids[0].width).toBe(6);
  });

  it('current defaults to 0', async () => {
    const fp = await fresh('a.op');
    await handleAddCarouselDotsV0({ filePath: fp, total: 3 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[0].role).toBe('dot-active');
    expect(kids[1].role).toBe('dot');
  });

  it('current > total-1 clamps to last dot', async () => {
    const fp = await fresh('a.op');
    await handleAddCarouselDotsV0({ filePath: fp, total: 3, current: 99 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[2].role).toBe('dot-active');
  });
});
