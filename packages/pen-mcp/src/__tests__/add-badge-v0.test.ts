import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddBadgeV0 } from '../tools/add-badge-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-badge-v0');
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

describe('add_badge_v0', () => {
  it('registered + required label', () => {
    expect(DESIGN_TOOL_NAMES.has('add_badge_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_badge_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('emits pill-style badge (cornerRadius=999, fit_content, horizontal, centered)', async () => {
    const fp = await fresh('a.op');
    await handleAddBadgeV0({ filePath: fp, label: 'NEW' });
    const b = getRoot(await readDoc(fp));
    expect(b.role).toBe('badge');
    expect(b.cornerRadius).toBe(999);
    expect(b.width).toBe('fit_content');
    expect(b.height).toBe('fit_content');
    expect(b.layout).toBe('horizontal');
    expect(b.alignItems).toBe('center');
    expect(b.padding).toEqual([4, 10]);
    const kids = b.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].content).toBe('NEW');
    expect(kids[0].fontSize).toBe(11);
    expect(kids[0].fontWeight).toBe(600);
  });

  it('badge + label both have unique ids', async () => {
    const fp = await fresh('a.op');
    await handleAddBadgeV0({ filePath: fp, label: 'BETA' });
    const b = getRoot(await readDoc(fp));
    const labelNode = (b.children as Record<string, unknown>[])[0];
    expect(b.id).not.toBe(labelNode.id);
    expect(typeof b.id).toBe('string');
    expect(typeof labelNode.id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddBadgeV0({ filePath: fp, label: 'X', parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
