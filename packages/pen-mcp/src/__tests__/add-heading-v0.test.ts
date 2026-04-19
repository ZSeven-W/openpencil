import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddHeadingV0 } from '../tools/add-heading-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-heading-v0');
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

describe('add_heading_v0', () => {
  it('registered + required content; level is enum with h2 default', () => {
    expect(DESIGN_TOOL_NAMES.has('add_heading_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_heading_v0');
    expect(def?.inputSchema.required).toEqual(['content']);
    const levelProp = (def?.inputSchema.properties as any)?.level;
    expect(levelProp?.enum).toEqual(['display', 'h1', 'h2', 'h3']);
  });

  it('default h2: 24/600/1.2 — lineHeight enforced (the core invariant)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Welcome' });
    const h = getRoot(await readDoc(fp));
    expect(h.type).toBe('text');
    expect(h.role).toBe('heading');
    expect(h.content).toBe('Welcome');
    expect(h.fontSize).toBe(24);
    expect(h.fontWeight).toBe(600);
    expect(h.lineHeight).toBe(1.2); // NOT 1.5 (default) — which stacks tight
  });

  it('display: 48/700/1.0 + letterSpacing=-0.5', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Hero', level: 'display' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(48);
    expect(h.fontWeight).toBe(700);
    expect(h.lineHeight).toBe(1.0);
    expect(h.letterSpacing).toBe(-0.5);
  });

  it('h1: 32/700/1.1 (no letterSpacing set)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'H1 Title', level: 'h1' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(32);
    expect(h.fontWeight).toBe(700);
    expect(h.lineHeight).toBe(1.1);
    expect(h.letterSpacing).toBeUndefined();
  });

  it('h3: 20/600/1.25 (card / list headers)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Card Title', level: 'h3' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(20);
    expect(h.fontWeight).toBe(600);
    expect(h.lineHeight).toBe(1.25);
  });

  it('has a valid id', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'X' });
    expect(typeof getRoot(await readDoc(fp)).id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddHeadingV0({ filePath: fp, content: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
