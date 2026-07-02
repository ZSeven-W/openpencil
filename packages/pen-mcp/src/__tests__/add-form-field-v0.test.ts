import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddFormFieldV0 } from '../tools/add-form-field-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-form-field-v0');
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

describe('add_form_field_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_form_field_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_form_field_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('minimal: label + input, input width=fill_container + height=48 (FORMS rule)', async () => {
    const fp = await fresh('a.op');
    await handleAddFormFieldV0({ filePath: fp, label: 'Email' });
    const field = getRoot(await readDoc(fp));
    expect(field.role).toBe('form-field');
    expect(field.width).toBe('fill_container');
    expect(field.layout).toBe('vertical');
    expect(field.gap).toBe(6);
    const kids = field.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    // label
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('Email'); // no "*" without required
    expect(kids[0].fontSize).toBe(14);
    expect(kids[0].fontWeight).toBe(500);
    // input container
    const input = kids[1];
    expect(input.role).toBe('form-input');
    expect(input.width).toBe('fill_container'); // FORMS rule
    expect(input.height).toBe(48); // ROLE_GUIDE input height
    expect(input.padding).toEqual([12, 16]); // ROLE_GUIDE input padding
    expect(input.layout).toBe('horizontal');
    expect(input.alignItems).toBe('center');
    // input has just placeholder (empty), no icons
    const inputKids = input.children as Record<string, unknown>[];
    expect(inputKids.length).toBe(1);
    expect(inputKids[0].type).toBe('text');
    expect(inputKids[0].content).toBe('');
  });

  it('required=true appends " *" to the label', async () => {
    const fp = await fresh('a.op');
    await handleAddFormFieldV0({ filePath: fp, label: 'Password', required: true });
    const label = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(label.content).toBe('Password *');
  });

  it('leading_icon + trailing_icon + placeholder all wire through', async () => {
    const fp = await fresh('a.op');
    await handleAddFormFieldV0({
      filePath: fp,
      label: 'Password',
      placeholder: 'Enter password',
      leading_icon: 'lock',
      trailing_icon: 'eye',
      required: true,
    });
    const input = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const inputKids = input.children as Record<string, unknown>[];
    expect(inputKids.length).toBe(3);
    expect(inputKids[0].type).toBe('icon_font');
    expect(inputKids[0].iconFontName).toBe('lock');
    expect(inputKids[1].type).toBe('text');
    expect(inputKids[1].content).toBe('Enter password');
    expect(inputKids[2].type).toBe('icon_font');
    expect(inputKids[2].iconFontName).toBe('eye');
  });

  it('leading only (email pattern): icon + placeholder, no trailing', async () => {
    const fp = await fresh('a.op');
    await handleAddFormFieldV0({
      filePath: fp,
      label: 'Email',
      placeholder: 'you@example.com',
      leading_icon: 'mail',
    });
    const input = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const inputKids = input.children as Record<string, unknown>[];
    expect(inputKids.length).toBe(2);
    expect(inputKids[0].iconFontName).toBe('mail');
    expect(inputKids[1].content).toBe('you@example.com');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddFormFieldV0({
      filePath: fp,
      label: 'X',
      leading_icon: 'a',
      trailing_icon: 'b',
    });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // field + label + input + (leading icon + placeholder + trailing icon) = 6
    expect(ids.length).toBe(6);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddFormFieldV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
