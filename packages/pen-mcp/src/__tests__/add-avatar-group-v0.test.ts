/**
 * Unit tests for add_avatar_group_v0 — stacked avatar tile group.
 * Distinct surface from `add_avatar_v0` (single tile).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddAvatarGroupV0 } from '../tools/add-avatar-group-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-avatar-group-v0-tests');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP_DIR, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
  return fp;
}

async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}

function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const pageChildren = pages?.[0]?.children;
  const topChildren = doc['children'] as Record<string, unknown>[] | undefined;
  const root = pageChildren?.[0] ?? topChildren?.[0];
  if (!root) throw new Error('expected root');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['group.op', 'overflow.op', 'clamp.op', 'parent.op', 'no-init.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_avatar_group_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_avatar_group_v0');
    expect(DESIGN_TOOL_NAMES.has('add_avatar_group_v0')).toBe(true);
  });
});

describe('add_avatar_group_v0 — structure', () => {
  it('emits avatar-group frame, horizontal layout, 4px gap, fit_content sizing', async () => {
    const fp = await fresh('group.op');
    await handleAddAvatarGroupV0({
      filePath: fp,
      items: [{ initial: 'A' }, { initial: 'B' }, { initial: 'C' }],
    });
    const group = getRoot(await readDoc(fp));
    expect(group.role).toBe('avatar-group');
    expect(group.layout).toBe('horizontal');
    expect(group.gap).toBe(4);
    expect(group.width).toBe('fit_content');
    expect(group.height).toBe('fit_content');
    const tiles = group.children as Record<string, unknown>[];
    expect(tiles.length).toBe(3);
    for (const tile of tiles) {
      expect(tile.role).toBe('avatar-group-item');
      expect(tile.width).toBe(32);
      expect(tile.height).toBe(32);
      expect(tile.cornerRadius).toBe(16);
      const stroke = tile.stroke as { thickness: number; fill: Array<{ color: string }> };
      expect(stroke.thickness).toBe(2);
      expect(stroke.fill[0].color).toBe('#FFFFFF');
    }
  });

  it('appends a "+N" overflow tile when items > max_visible', async () => {
    const fp = await fresh('overflow.op');
    await handleAddAvatarGroupV0({
      filePath: fp,
      max_visible: 3,
      items: [
        { initial: 'A' },
        { initial: 'B' },
        { initial: 'C' },
        { initial: 'D' },
        { initial: 'E' },
      ],
    });
    const group = getRoot(await readDoc(fp));
    const tiles = group.children as Record<string, unknown>[];
    expect(tiles.length).toBe(4); // 3 visible + 1 overflow
    const overflow = tiles[3];
    expect(overflow.role).toBe('avatar-group-overflow');
    const fill = overflow.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#F1F5F9');
    const count = (overflow.children as Record<string, unknown>[])[0];
    expect(count.role).toBe('avatar-group-overflow-count');
    expect(count.content).toBe('+2');
  });

  it('omits the overflow tile when items <= max_visible', async () => {
    const fp = await fresh('group.op');
    await handleAddAvatarGroupV0({
      filePath: fp,
      max_visible: 4,
      items: [{ initial: 'A' }, { initial: 'B' }],
    });
    const group = getRoot(await readDoc(fp));
    const tiles = group.children as Record<string, unknown>[];
    expect(tiles.length).toBe(2);
    expect((tiles[0] as Record<string, unknown>).role).toBe('avatar-group-item');
    expect((tiles[1] as Record<string, unknown>).role).toBe('avatar-group-item');
  });

  it('items without initial render as empty colored disks', async () => {
    const fp = await fresh('no-init.op');
    await handleAddAvatarGroupV0({
      filePath: fp,
      items: [{}, { color: '#000000' }],
    });
    const group = getRoot(await readDoc(fp));
    const tiles = group.children as Record<string, unknown>[];
    for (const tile of tiles) {
      expect(tile.children).toEqual([]);
    }
    const customFill = (tiles[1].fill as Array<{ color: string }>)[0].color;
    expect(customFill).toBe('#000000');
  });

  it('clamps size to [24, 64] and max_visible to [1, 10]', async () => {
    const fp = await fresh('clamp.op');
    await handleAddAvatarGroupV0({
      filePath: fp,
      size: 12,
      max_visible: 99,
      items: Array.from({ length: 12 }, (_, i) => ({ initial: String.fromCharCode(65 + i) })),
    });
    const small = getRoot(await readDoc(fp));
    const tiles = small.children as Record<string, unknown>[];
    expect((tiles[0] as Record<string, unknown>).width).toBe(24);
    // max_visible clamped to 10 — 12 items, 10 visible, "+2" overflow tile
    expect(tiles.length).toBe(11);
    const overflow = tiles[10] as Record<string, unknown>;
    const count = (overflow.children as Record<string, unknown>[])[0];
    expect(count.content).toBe('+2');

    invalidateCache(fp);
    await writeFile(fp, EMPTY_DOC, 'utf-8');
    await handleAddAvatarGroupV0({
      filePath: fp,
      size: 999,
      items: [{ initial: 'X' }],
    });
    const big = getRoot(await readDoc(fp));
    const bigTiles = big.children as Record<string, unknown>[];
    expect((bigTiles[0] as Record<string, unknown>).width).toBe(64);
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('parent.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddAvatarGroupV0({
        filePath: fp,
        items: [{ initial: 'A' }],
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
