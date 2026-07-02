import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCheckboxV0 } from '../tools/add-checkbox-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-checkbox-v0');
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

describe('add_checkbox_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_checkbox_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_checkbox_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('unchecked (default): empty box with stroke, no check icon', async () => {
    const fp = await fresh('a.op');
    await handleAddCheckboxV0({ filePath: fp, label: 'Accept terms' });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('checkbox-row');
    expect(row.layout).toBe('horizontal');
    const kids = row.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    const box = kids[0];
    expect(box.role).toBe('checkbox');
    expect(box.width).toBe(20);
    expect(box.cornerRadius).toBe(4);
    expect(box.fill).toEqual([]);
    expect((box.stroke as Record<string, unknown>).thickness).toBe(1.5);
    expect((box.children as unknown[]).length).toBe(0);
    expect(kids[1].content).toBe('Accept terms');
  });

  it('checked=true: filled box + check icon inside', async () => {
    const fp = await fresh('a.op');
    await handleAddCheckboxV0({ filePath: fp, label: 'Done', checked: true });
    const box = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(box.role).toBe('checkbox-checked');
    const fills = box.fill as Array<{ color: string }>;
    expect(fills[0].color).toBe('#2563EB');
    const inner = (box.children as Record<string, unknown>[])[0];
    expect(inner.type).toBe('icon_font');
    expect(inner.iconFontName).toBe('check');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddCheckboxV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
