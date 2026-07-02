import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSwitchV0 } from '../tools/add-switch-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-switch-v0');
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

describe('add_switch_v0', () => {
  it('registered + no required fields', () => {
    expect(DESIGN_TOOL_NAMES.has('add_switch_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_switch_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('off (default): 51x31, cornerRadius=16, thumb left, gray track', async () => {
    const fp = await fresh('a.op');
    await handleAddSwitchV0({ filePath: fp });
    const s = getRoot(await readDoc(fp));
    expect(s.role).toBe('switch');
    expect(s.width).toBe(51);
    expect(s.height).toBe(31);
    expect(s.cornerRadius).toBe(16);
    expect(s.justifyContent).toBe('flex-start');
    const fills = s.fill as Array<{ color: string }>;
    expect(fills[0].color).toBe('#E5E5EA');
    const kids = s.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('switch-thumb');
    expect(kids[0].width).toBe(27);
  });

  it('on: green track + thumb flex-end', async () => {
    const fp = await fresh('a.op');
    await handleAddSwitchV0({ filePath: fp, active: true });
    const s = getRoot(await readDoc(fp));
    expect(s.justifyContent).toBe('flex-end');
    const fills = s.fill as Array<{ color: string }>;
    expect(fills[0].color).toBe('#34C759');
  });

  it('2 unique ids (track + thumb)', async () => {
    const fp = await fresh('a.op');
    await handleAddSwitchV0({ filePath: fp });
    const s = getRoot(await readDoc(fp));
    const ids = [s.id, (s.children as Record<string, unknown>[])[0].id];
    expect(new Set(ids).size).toBe(2);
    for (const id of ids) expect(typeof id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddSwitchV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
