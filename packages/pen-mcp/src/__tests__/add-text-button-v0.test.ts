import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTextButtonV0 } from '../tools/add-text-button-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-text-button-v0');
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

describe('add_text_button_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_text_button_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_text_button_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('padding-based, NO explicit height (Pencil demo pattern)', async () => {
    const fp = await fresh('a.op');
    await handleAddTextButtonV0({ filePath: fp, label: 'Continue' });
    const b = getRoot(await readDoc(fp));
    expect(b.role).toBe('button');
    expect(b.padding).toEqual([12, 20]);
    expect(b.cornerRadius).toBe(8);
    expect(b.width).toBe('fit_content');
    expect(b.height).toBe('fit_content'); // MUST be fit_content, not a fixed px height
    expect(b.layout).toBe('horizontal');
    expect(b.alignItems).toBe('center');
    expect(b.justifyContent).toBe('center');
    expect(b.gap).toBe(8);
    const kids = b.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('Continue');
    expect(kids[0].fontSize).toBe(14);
    expect(kids[0].fontWeight).toBe(500);
  });

  it('with leading_icon: icon + label (2 kids)', async () => {
    const fp = await fresh('a.op');
    await handleAddTextButtonV0({ filePath: fp, label: 'Add', leading_icon: 'plus' });
    const b = getRoot(await readDoc(fp));
    const kids = b.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('plus');
    expect(kids[0].width).toBe(16);
    expect(kids[1].type).toBe('text');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTextButtonV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
