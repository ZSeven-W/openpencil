import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddNavChipRowV0 } from '../tools/add-nav-chip-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-nav-chip-row-v0');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
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

describe('add_nav_chip_row_v0', () => {
  it('registered + required items (label only; icon optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_nav_chip_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_nav_chip_row_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
    const itemSchema = (def?.inputSchema.properties as any)?.items?.items;
    // Matches pre-split behavior: add_scroll_row_v0(children_type='nav_item')
    // allowed label-only items. Regressing that is unacceptable.
    expect(itemSchema?.required).toEqual(['label']);
  });

  it('builds label-only chips when icon is omitted (text filter tags)', async () => {
    const fp = await fresh('a.op');
    await handleAddNavChipRowV0({
      filePath: fp,
      items: [{ label: 'All' }, { label: 'Videos' }, { label: 'Photos' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    const chips = row.children as Record<string, unknown>[];
    expect(chips.length).toBe(3);
    for (const chip of chips) {
      const kids = chip.children as Record<string, unknown>[];
      // only label text, no icon_font
      expect(kids.length).toBe(1);
      expect(kids[0].type).toBe('text');
      expect(kids[0].role).toBe('label');
    }
  });

  it('builds 72-wide chips with role nav-chip / nav-chip-active + alignItems=center', async () => {
    const fp = await fresh('a.op');
    await handleAddNavChipRowV0({
      filePath: fp,
      items: [
        { label: 'All', icon: 'grid', active: true },
        { label: 'Videos', icon: 'video' },
      ],
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    const chips = row.children as Record<string, unknown>[];
    expect(chips.length).toBe(2);
    expect(chips[0].width).toBe(72);
    expect(chips[0].alignItems).toBe('center');
    expect(chips[0].role).toBe('nav-chip-active');
    expect(chips[1].role).toBe('nav-chip');
    // active chip has bolder label
    expect((chips[0].children as Record<string, unknown>[])[1].fontWeight).toBe(600);
    expect((chips[1].children as Record<string, unknown>[])[1].fontWeight).toBe(500);
  });

  it('throws on bogus parent_id', async () => {
    const fp = await fresh('a.op');
    await expect(
      handleAddNavChipRowV0({
        filePath: fp,
        items: [{ label: 'X', icon: 'x' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
  });
});
