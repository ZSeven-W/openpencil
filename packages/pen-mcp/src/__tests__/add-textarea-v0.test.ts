import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTextareaV0 } from '../tools/add-textarea-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-textarea-v0');
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

describe('add_textarea_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_textarea_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_textarea_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('minimal: label + multi-line input (rows=4 default → height=120)', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({ filePath: fp, label: 'Bio' });
    const ta = getRoot(await readDoc(fp));
    expect(ta.role).toBe('textarea');
    expect(ta.width).toBe('fill_container');
    expect(ta.layout).toBe('vertical');
    expect(ta.gap).toBe(6);
    const kids = ta.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    // label
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('Bio');
    // input: 4 rows × 24 lh + 24 padding = 120
    const input = kids[1];
    expect(input.role).toBe('textarea-input');
    expect(input.height).toBe(120);
    expect(input.layout).toBe('vertical'); // differs from form-field (horizontal)
    expect(input.alignItems).toBe('start');
    expect(input.padding).toEqual([12, 16]);
  });

  it('rows controls initial height linearly: 2 rows → 72, 8 rows → 216', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({ filePath: fp, label: 'Small', rows: 2 });
    const small = getRoot(await readDoc(fp));
    const smallInput = (small.children as Record<string, unknown>[])[1];
    expect(smallInput.height).toBe(72); // 2*24 + 24

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddTextareaV0({ filePath: fp, label: 'Big', rows: 8 });
    const big = getRoot(await readDoc(fp));
    const bigInput = (big.children as Record<string, unknown>[])[1];
    expect(bigInput.height).toBe(216); // 8*24 + 24
  });

  it('rows clamped to [2, 12]: 0 → 2, 99 → 12', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({ filePath: fp, label: 'X', rows: 0 });
    const low = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    expect(low.height).toBe(72); // clamped to 2 rows

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddTextareaV0({ filePath: fp, label: 'Y', rows: 99 });
    const high = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    expect(high.height).toBe(312); // clamped to 12 rows: 12*24 + 24
  });

  it('required=true appends " *" to the label (same shape as form-field)', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({ filePath: fp, label: 'Feedback', required: true });
    const label = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(label.content).toBe('Feedback *');
  });

  it('placeholder lands inside the input area', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({
      filePath: fp,
      label: 'Bio',
      placeholder: 'Tell us about yourself',
    });
    const input = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const inputKids = input.children as Record<string, unknown>[];
    expect(inputKids[0].type).toBe('text');
    expect(inputKids[0].content).toBe('Tell us about yourself');
    // placeholder has lineHeight 1.5 so multi-line wrapping is visually correct
    expect(inputKids[0].lineHeight).toBe(1.5);
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddTextareaV0({ filePath: fp, label: 'X', placeholder: 'p' });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // textarea + label + input + placeholder = 4 nodes
    expect(ids.length).toBe(4);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTextareaV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
