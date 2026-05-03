import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSelectV0 } from '../tools/add-select-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-select-v0');
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

describe('add_select_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_select_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_select_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('with value set: black value text + chevron-down trailing', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({ filePath: fp, label: 'Country', value: 'United States' });
    const sel = getRoot(await readDoc(fp));
    expect(sel.role).toBe('select');
    expect(sel.layout).toBe('vertical');
    const [label, input] = sel.children as Record<string, unknown>[];
    expect(label.content).toBe('Country');
    expect(input.role).toBe('select-input');
    expect(input.justifyContent).toBe('space_between');
    const [textNode, trailing] = input.children as Record<string, unknown>[];
    expect(textNode.content).toBe('United States');
    // NOT gray when value is set
    expect(textNode.fill).toBeUndefined();
    expect(trailing.type).toBe('icon_font');
    expect(trailing.iconFontName).toBe('chevron-down');
  });

  it('no value: placeholder text with gray fill #94A3B8', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({ filePath: fp, label: 'Country' });
    const sel = getRoot(await readDoc(fp));
    const input = (sel.children as Record<string, unknown>[])[1];
    const textNode = (input.children as Record<string, unknown>[])[0];
    expect(textNode.content).toBe('Select…');
    const fill = textNode.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#94A3B8');
  });

  it('custom placeholder overrides default "Select…"', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({
      filePath: fp,
      label: 'Currency',
      placeholder: 'Choose currency',
    });
    const input = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const textNode = (input.children as Record<string, unknown>[])[0];
    expect(textNode.content).toBe('Choose currency');
  });

  it('custom trailing_icon overrides default chevron-down', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({
      filePath: fp,
      label: 'Country',
      value: 'US',
      trailing_icon: 'chevrons-up-down',
    });
    const input = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const trailing = (input.children as Record<string, unknown>[])[1];
    expect(trailing.iconFontName).toBe('chevrons-up-down');
  });

  it('required=true appends " *" to label', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({ filePath: fp, label: 'Country', required: true });
    const label = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(label.content).toBe('Country *');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddSelectV0({ filePath: fp, label: 'X', value: 'Y' });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // select + label + input + (value text + trailing icon) = 5
    expect(ids.length).toBe(5);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddSelectV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
