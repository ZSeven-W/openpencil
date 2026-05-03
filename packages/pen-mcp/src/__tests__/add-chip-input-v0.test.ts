import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddChipInputV0 } from '../tools/add-chip-input-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-chip-input-v0');
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

describe('add_chip_input_v0', () => {
  it('registered; required=[label]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_chip_input_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_chip_input_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('with chips: pill per value + trailing caret', async () => {
    const fp = await fresh('a.op');
    await handleAddChipInputV0({
      filePath: fp,
      label: 'Tags',
      chips: ['design', 'mobile', 'a11y'],
    });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('chip-input');
    const kids = root.children as Record<string, unknown>[];
    const field = kids.find((k) => k.role === 'chip-input-field')!;
    const fieldKids = field.children as Record<string, unknown>[];
    // 3 chips + caret
    expect(fieldKids.length).toBe(4);
    const chipCount = fieldKids.filter((c) => c.role === 'chip').length;
    expect(chipCount).toBe(3);
    expect(fieldKids[fieldKids.length - 1].role).toBe('chip-input-caret');
  });

  it('empty chips → only caret with default placeholder', async () => {
    const fp = await fresh('a.op');
    await handleAddChipInputV0({ filePath: fp, label: 'Tags' });
    const root = getRoot(await readDoc(fp));
    const field = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'chip-input-field',
    )!;
    const fieldKids = field.children as Record<string, unknown>[];
    expect(fieldKids.length).toBe(1);
    expect(fieldKids[0].role).toBe('chip-input-caret');
    expect(fieldKids[0].content).toBe('Add tag…');
  });

  it('custom placeholder wins over default', async () => {
    const fp = await fresh('a.op');
    await handleAddChipInputV0({
      filePath: fp,
      label: 'Send to',
      chips: [],
      placeholder: 'Enter emails',
    });
    const root = getRoot(await readDoc(fp));
    const field = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'chip-input-field',
    )!;
    const caret = (field.children as Record<string, unknown>[])[0];
    expect(caret.content).toBe('Enter emails');
  });

  it('required=true → label gets " *"', async () => {
    const fp = await fresh('a.op');
    await handleAddChipInputV0({ filePath: fp, label: 'Tags', required: true });
    const root = getRoot(await readDoc(fp));
    const label = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'chip-input-label',
    )!;
    expect(label.content).toBe('Tags *');
  });

  it('field wraps (layoutWrap=wrap) for overflow behavior', async () => {
    const fp = await fresh('a.op');
    await handleAddChipInputV0({ filePath: fp, label: 'T', chips: ['a', 'b'] });
    const root = getRoot(await readDoc(fp));
    const field = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'chip-input-field',
    )!;
    expect(field.layoutWrap).toBe('wrap');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddChipInputV0({ filePath: fp, label: 'T', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
