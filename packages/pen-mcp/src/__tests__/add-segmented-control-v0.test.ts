import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSegmentedControlV0 } from '../tools/add-segmented-control-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-segmented-control-v0');
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

describe('add_segmented_control_v0', () => {
  it('registered + required items', () => {
    expect(DESIGN_TOOL_NAMES.has('add_segmented_control_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_segmented_control_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('3 segments, each fill_container (overflow-safe), active white', async () => {
    const fp = await fresh('a.op');
    await handleAddSegmentedControlV0({
      filePath: fp,
      items: [{ label: 'Day' }, { label: 'Week', active: true }, { label: 'Month' }],
    });
    const ctrl = getRoot(await readDoc(fp));
    expect(ctrl.role).toBe('segmented-control');
    expect(ctrl.width).toBe('fill_container');
    expect(ctrl.height).toBe(32);
    expect(ctrl.cornerRadius).toBe(8);
    expect(ctrl.padding).toEqual([4]);
    const segs = ctrl.children as Record<string, unknown>[];
    expect(segs.length).toBe(3);
    // every segment uses fill_container width (the overflow-safe invariant)
    for (const seg of segs) {
      expect(seg.width).toBe('fill_container');
    }
    expect(segs[0].role).toBe('segment');
    expect(segs[1].role).toBe('segment-active');
    expect(segs[2].role).toBe('segment');
    // active has white fill
    const activeFills = segs[1].fill as Array<{ color: string }>;
    expect(activeFills[0].color).toBe('#FFFFFF');
    // non-active has empty fill
    expect(segs[0].fill).toEqual([]);
    // active label weight 600
    const activeLabel = (segs[1].children as Record<string, unknown>[])[0];
    expect(activeLabel.fontWeight).toBe(600);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddSegmentedControlV0({
        filePath: fp,
        items: [{ label: 'A' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
