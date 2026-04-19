/**
 * Unit tests for add_activity_ring_v0 — MVP element tool #3.
 * Anti-pattern origin: layout.md §RING / CIRCLE WITH CENTER CONTENT.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddActivityRingV0 } from '../tools/add-activity-ring-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-activity-ring-v0-tests');
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
  for (const f of ['ring.op', 'custom.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_activity_ring_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_activity_ring_v0');
    expect(DESIGN_TOOL_NAMES.has('add_activity_ring_v0')).toBe(true);
  });
});

describe('add_activity_ring_v0 — structure matches anti-pattern fix', () => {
  it('emits frame+cornerRadius=size/2+stroke+centered text (NOT ellipse+sibling)', async () => {
    const fp = await fresh('ring.op');
    await handleAddActivityRingV0({
      filePath: fp,
      center_text: '8,432',
    });
    const ring = getRoot(await readDoc(fp));
    // Must be frame, not ellipse (ellipse has no children)
    expect(ring.type).toBe('frame');
    expect(ring.role).toBe('activity-ring');
    // Default size 80 → cornerRadius 40 = width/2
    expect(ring.width).toBe(80);
    expect(ring.height).toBe(80);
    expect(ring.cornerRadius).toBe(40);
    // fill=[] keeps it hollow; stroke renders the ring
    expect(ring.fill).toEqual([]);
    expect(ring.stroke).toEqual({
      thickness: 8,
      fill: [{ type: 'solid', color: '#000000' }],
    });
    // Flex centering, NOT layout=none
    expect(ring.layout).toBe('horizontal');
    expect(ring.alignItems).toBe('center');
    expect(ring.justifyContent).toBe('center');
    // Single text child (no siblings)
    const kids = ring.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('8,432');
    expect(kids[0].fontSize).toBe(16);
    expect(kids[0].fontWeight).toBe(700);
  });

  it('respects geometric overrides (size / thickness); typography is hardcoded (style orthogonal)', async () => {
    const fp = await fresh('custom.op');
    await handleAddActivityRingV0({
      filePath: fp,
      size: 120,
      thickness: 12,
      center_text: '75%',
    });
    const ring = getRoot(await readDoc(fp));
    expect(ring.width).toBe(120);
    expect(ring.height).toBe(120);
    expect(ring.cornerRadius).toBe(60); // always size / 2
    expect(ring.stroke).toEqual({
      thickness: 12,
      fill: [{ type: 'solid', color: '#000000' }], // hardcoded, not tunable
    });
    const text = (ring.children as Record<string, unknown>[])[0];
    // typography is hardcoded per spec D6 — override via batch_design U-op
    expect(text.fontSize).toBe(16);
    expect(text.fontWeight).toBe(700);
  });

  it('every node has a valid unique id', async () => {
    const fp = await fresh('ring.op');
    await handleAddActivityRingV0({
      filePath: fp,
      center_text: 'A',
    });
    const ring = getRoot(await readDoc(fp));
    const ringId = ring.id;
    const textId = (ring.children as Record<string, unknown>[])[0].id;
    expect(typeof ringId).toBe('string');
    expect(typeof textId).toBe('string');
    expect(ringId).not.toBe(textId);
    expect((ringId as string).length).toBeGreaterThan(0);
    expect((textId as string).length).toBeGreaterThan(0);
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('ring.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddActivityRingV0({
        filePath: fp,
        center_text: 'X',
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
