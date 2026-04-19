import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddIconLabelV0 } from '../tools/add-icon-label-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-icon-label-v0');
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

describe('add_icon_label_v0', () => {
  it('registered + required icon + label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_icon_label_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_icon_label_v0');
    expect(def?.inputSchema.required).toEqual(['icon', 'label']);
  });

  it('horizontal frame with centered alignment + 2 children (icon then label)', async () => {
    const fp = await fresh('a.op');
    await handleAddIconLabelV0({ filePath: fp, icon: 'info', label: 'Details' });
    const n = getRoot(await readDoc(fp));
    expect(n.type).toBe('frame');
    expect(n.role).toBe('icon-label');
    expect(n.width).toBe('fit_content');
    expect(n.height).toBe('fit_content');
    expect(n.layout).toBe('horizontal');
    expect(n.alignItems).toBe('center');
    expect(n.gap).toBe(8);
    const kids = n.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('info');
    expect(kids[0].width).toBe(16);
    expect(kids[1].type).toBe('text');
    expect(kids[1].content).toBe('Details');
    expect(kids[1].fontSize).toBe(14);
  });

  it('custom gap', async () => {
    const fp = await fresh('a.op');
    await handleAddIconLabelV0({ filePath: fp, icon: 'x', label: 'y', gap: 4 });
    expect(getRoot(await readDoc(fp)).gap).toBe(4);
  });

  it('3 unique ids (frame + icon + label)', async () => {
    const fp = await fresh('a.op');
    await handleAddIconLabelV0({ filePath: fp, icon: 'x', label: 'y' });
    const n = getRoot(await readDoc(fp));
    const ids = [n.id, ...(n.children as Record<string, unknown>[]).map((c) => c.id)];
    for (const id of ids) {
      expect(typeof id).toBe('string');
      expect((id as string).length).toBeGreaterThan(0);
    }
    expect(new Set(ids).size).toBe(3);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddIconLabelV0({ filePath: fp, icon: 'x', label: 'y', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
