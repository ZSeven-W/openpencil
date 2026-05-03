import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddEmptyStateV0 } from '../tools/add-empty-state-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-empty-state-v0');
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

describe('add_empty_state_v0', () => {
  it('registered + required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_empty_state_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_empty_state_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('minimal (title only): 1 child', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyStateV0({ filePath: fp, title: 'Nothing here yet' });
    const es = getRoot(await readDoc(fp));
    expect(es.role).toBe('empty-state');
    expect(es.width).toBe('fill_container');
    expect(es.layout).toBe('vertical');
    expect(es.alignItems).toBe('center');
    expect(es.padding).toEqual([48, 24]);
    const kids = es.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('empty-state-title');
    expect(kids[0].content).toBe('Nothing here yet');
    expect(kids[0].fontSize).toBe(18);
  });

  it('full 4-piece: icon + title + subtitle + CTA', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyStateV0({
      filePath: fp,
      title: 'No items',
      subtitle: 'Add one to get started',
      icon: 'inbox',
      cta_label: 'Create new',
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(4);
    expect(kids[0].role).toBe('empty-state-icon');
    expect(kids[0].iconFontName).toBe('inbox');
    expect(kids[1].role).toBe('empty-state-title');
    expect(kids[2].role).toBe('empty-state-subtitle');
    expect(kids[2].content).toBe('Add one to get started');
    expect(kids[3].role).toBe('button');
    const ctaLabel = (kids[3].children as Record<string, unknown>[])[0];
    expect(ctaLabel.content).toBe('Create new');
  });

  it('subtitle only (no icon, no CTA): 2 children', async () => {
    const fp = await fresh('a.op');
    await handleAddEmptyStateV0({
      filePath: fp,
      title: 'No results',
      subtitle: 'Try a different keyword',
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].role).toBe('empty-state-title');
    expect(kids[1].role).toBe('empty-state-subtitle');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddEmptyStateV0({ filePath: fp, title: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
