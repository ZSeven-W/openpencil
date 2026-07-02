import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddAvatarV0 } from '../tools/add-avatar-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-avatar-v0');
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

describe('add_avatar_v0', () => {
  it('registered + no required fields', () => {
    expect(DESIGN_TOOL_NAMES.has('add_avatar_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_avatar_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default circle (40×40, cornerRadius=20, flex-centered) with NO initial', async () => {
    const fp = await fresh('a.op');
    await handleAddAvatarV0({ filePath: fp });
    const a = getRoot(await readDoc(fp));
    expect(a.type).toBe('frame');
    expect(a.role).toBe('avatar');
    expect(a.width).toBe(40);
    expect(a.height).toBe(40);
    expect(a.cornerRadius).toBe(20);
    expect(a.layout).toBe('horizontal');
    expect(a.alignItems).toBe('center');
    expect(a.justifyContent).toBe('center');
    expect(a.children).toEqual([]);
  });

  it('with initial: centered text child with auto-sized font (size × 0.4)', async () => {
    const fp = await fresh('a.op');
    await handleAddAvatarV0({ filePath: fp, initial: 'JD', size: 80 });
    const a = getRoot(await readDoc(fp));
    expect(a.width).toBe(80);
    expect(a.cornerRadius).toBe(40);
    const kids = a.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('JD');
    // 80 * 0.4 = 32
    expect(kids[0].fontSize).toBe(32);
    expect(kids[0].fontWeight).toBe(600);
  });

  it('default size initial font floors at 12 for tiny avatars', async () => {
    const fp = await fresh('a.op');
    await handleAddAvatarV0({ filePath: fp, initial: 'X', size: 20 });
    const text = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    // 20 * 0.4 = 8 → floored to 12
    expect(text.fontSize).toBe(12);
  });

  it('cornerRadius is always size/2 (perfect circle, matches activity-ring pattern)', async () => {
    const fp = await fresh('a.op');
    await handleAddAvatarV0({ filePath: fp, size: 56 });
    expect(getRoot(await readDoc(fp)).cornerRadius).toBe(28);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddAvatarV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
