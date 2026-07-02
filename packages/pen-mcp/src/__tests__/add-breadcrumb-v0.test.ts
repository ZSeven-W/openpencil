import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddBreadcrumbV0 } from '../tools/add-breadcrumb-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-breadcrumb-v0');
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

describe('add_breadcrumb_v0', () => {
  it('registered + required items', () => {
    expect(DESIGN_TOOL_NAMES.has('add_breadcrumb_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_breadcrumb_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('3 items: 3 texts + 2 separators, last item auto-active', async () => {
    const fp = await fresh('a.op');
    await handleAddBreadcrumbV0({
      filePath: fp,
      items: [{ label: 'Home' }, { label: 'Settings' }, { label: 'Billing' }],
    });
    const bc = getRoot(await readDoc(fp));
    expect(bc.role).toBe('breadcrumb');
    const kids = bc.children as Record<string, unknown>[];
    expect(kids.length).toBe(5); // 3 + 2
    expect(kids[0].role).toBe('breadcrumb-item');
    expect(kids[0].content).toBe('Home');
    expect(kids[0].fontWeight).toBe(400);
    expect(kids[1].role).toBe('breadcrumb-separator');
    expect(kids[1].iconFontName).toBe('chevron-right');
    expect(kids[2].role).toBe('breadcrumb-item');
    expect(kids[2].content).toBe('Settings');
    expect(kids[3].role).toBe('breadcrumb-separator');
    expect(kids[4].role).toBe('breadcrumb-item-active');
    expect(kids[4].content).toBe('Billing');
    expect(kids[4].fontWeight).toBe(600);
  });

  it('single item: 1 child, no separator', async () => {
    const fp = await fresh('a.op');
    await handleAddBreadcrumbV0({ filePath: fp, items: [{ label: 'Home' }] });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('breadcrumb-item-active');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddBreadcrumbV0({ filePath: fp, items: [{ label: 'A' }], parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
