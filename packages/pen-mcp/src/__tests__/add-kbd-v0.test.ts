import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddKbdV0 } from '../tools/add-kbd-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-kbd-v0');
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

describe('add_kbd_v0', () => {
  it('registered + required keys', () => {
    expect(DESIGN_TOOL_NAMES.has('add_kbd_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_kbd_v0');
    expect(def?.inputSchema.required).toEqual(['keys']);
  });

  it('3 keys with default separator "+": 3 key cells + 2 separators', async () => {
    const fp = await fresh('a.op');
    await handleAddKbdV0({ filePath: fp, keys: ['Ctrl', 'Shift', 'P'] });
    const kbd = getRoot(await readDoc(fp));
    expect(kbd.role).toBe('kbd');
    const kids = kbd.children as Record<string, unknown>[];
    expect(kids.length).toBe(5);
    expect(kids[0].role).toBe('kbd-key');
    expect(kids[1].role).toBe('kbd-separator');
    expect(kids[1].content).toBe('+');
    expect(kids[2].role).toBe('kbd-key');
    expect(kids[3].role).toBe('kbd-separator');
    expect(kids[4].role).toBe('kbd-key');
    // Key cell has its own text child with the glyph
    const firstCellKids = kids[0].children as Record<string, unknown>[];
    expect(firstCellKids[0].content).toBe('Ctrl');
  });

  it('custom separator " ": inserts " " between keys', async () => {
    const fp = await fresh('a.op');
    await handleAddKbdV0({ filePath: fp, keys: ['⌘', 'K'], separator: ' ' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[1].content).toBe(' ');
  });

  it('empty separator "": omits separator entirely', async () => {
    const fp = await fresh('a.op');
    await handleAddKbdV0({ filePath: fp, keys: ['⌘', 'K'], separator: '' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids.every((k) => k.role === 'kbd-key')).toBe(true);
  });

  it('single key: no separator emitted', async () => {
    const fp = await fresh('a.op');
    await handleAddKbdV0({ filePath: fp, keys: ['Enter'] });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('kbd-key');
  });

  it('throws on empty keys array (after filtering)', async () => {
    const fp = await fresh('a.op');
    await expect(handleAddKbdV0({ filePath: fp, keys: [] })).rejects.toThrow(
      /at least one non-empty key/,
    );
    await expect(handleAddKbdV0({ filePath: fp, keys: ['', ''] })).rejects.toThrow(
      /at least one non-empty key/,
    );
  });
});
