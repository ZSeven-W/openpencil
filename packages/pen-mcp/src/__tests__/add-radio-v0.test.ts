import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddRadioV0 } from '../tools/add-radio-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-radio-v0');
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

describe('add_radio_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_radio_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_radio_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('unselected (default): ring, no inner dot', async () => {
    const fp = await fresh('a.op');
    await handleAddRadioV0({ filePath: fp, label: 'Option A' });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('radio-row');
    const kids = row.children as Record<string, unknown>[];
    const outer = kids[0];
    expect(outer.role).toBe('radio');
    expect(outer.cornerRadius).toBe(10);
    expect(outer.fill).toEqual([]);
    expect((outer.children as unknown[]).length).toBe(0);
    expect(kids[1].content).toBe('Option A');
  });

  it('selected=true: ring + centered dot (cornerRadius=5)', async () => {
    const fp = await fresh('a.op');
    await handleAddRadioV0({ filePath: fp, label: 'Option B', selected: true });
    const outer = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(outer.role).toBe('radio-selected');
    const dot = (outer.children as Record<string, unknown>[])[0];
    expect(dot.role).toBe('radio-dot');
    expect(dot.width).toBe(10);
    expect(dot.cornerRadius).toBe(5);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddRadioV0({ filePath: fp, label: 'X', parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
