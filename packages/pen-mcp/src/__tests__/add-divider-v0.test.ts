import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddDividerV0 } from '../tools/add-divider-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-divider-v0');
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

describe('add_divider_v0', () => {
  it('registered + no required fields (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_divider_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_divider_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default horizontal: rectangle with fill_container width + height=1', async () => {
    const fp = await fresh('a.op');
    await handleAddDividerV0({ filePath: fp });
    const d = getRoot(await readDoc(fp));
    expect(d.type).toBe('rectangle');
    expect(d.role).toBe('divider');
    expect(d.width).toBe('fill_container');
    expect(d.height).toBe(1);
  });

  it('vertical orientation swaps: width=1 + fill_container height', async () => {
    const fp = await fresh('a.op');
    await handleAddDividerV0({ filePath: fp, orientation: 'vertical' });
    const d = getRoot(await readDoc(fp));
    expect(d.width).toBe(1);
    expect(d.height).toBe('fill_container');
  });

  it('custom thickness', async () => {
    const fp = await fresh('a.op');
    await handleAddDividerV0({ filePath: fp, thickness: 2 });
    expect(getRoot(await readDoc(fp)).height).toBe(2);
  });

  it('has a valid id', async () => {
    const fp = await fresh('a.op');
    await handleAddDividerV0({ filePath: fp });
    expect(typeof getRoot(await readDoc(fp)).id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddDividerV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
