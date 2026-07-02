import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddRatingStarsV0 } from '../tools/add-rating-stars-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-rating-stars-v0');
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

describe('add_rating_stars_v0', () => {
  it('registered + required filled', () => {
    expect(DESIGN_TOOL_NAMES.has('add_rating_stars_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_rating_stars_v0');
    expect(def?.inputSchema.required).toEqual(['filled']);
  });

  it('4 of 5: 4 filled + 1 empty star icons', async () => {
    const fp = await fresh('a.op');
    await handleAddRatingStarsV0({ filePath: fp, filled: 4 });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('rating-stars');
    const kids = row.children as Record<string, unknown>[];
    expect(kids.length).toBe(5);
    expect(kids.slice(0, 4).every((k) => k.role === 'star-filled')).toBe(true);
    expect(kids[4].role).toBe('star-empty');
    expect(kids[0].iconFontName).toBe('star');
    expect(kids[0].iconFontFamily).toBe('lucide');
    expect(kids[0].width).toBe(16);
  });

  it('respects custom total + size', async () => {
    const fp = await fresh('a.op');
    await handleAddRatingStarsV0({ filePath: fp, filled: 2, total: 3, size: 24 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].width).toBe(24);
  });

  it('clamps filled > total down to total', async () => {
    const fp = await fresh('a.op');
    await handleAddRatingStarsV0({ filePath: fp, filled: 99, total: 5 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.every((k) => k.role === 'star-filled')).toBe(true);
  });

  it('clamps filled < 0 up to 0', async () => {
    const fp = await fresh('a.op');
    await handleAddRatingStarsV0({ filePath: fp, filled: -3 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.every((k) => k.role === 'star-empty')).toBe(true);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddRatingStarsV0({ filePath: fp, filled: 3, parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
