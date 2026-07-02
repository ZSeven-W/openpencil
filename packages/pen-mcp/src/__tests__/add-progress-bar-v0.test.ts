import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddProgressBarV0 } from '../tools/add-progress-bar-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-progress-bar-v0');
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

describe('add_progress_bar_v0', () => {
  it('registered + no required fields', () => {
    expect(DESIGN_TOOL_NAMES.has('add_progress_bar_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_progress_bar_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('defaults: bar_width=240, value=50 → fill_width=120', async () => {
    const fp = await fresh('a.op');
    await handleAddProgressBarV0({ filePath: fp });
    const track = getRoot(await readDoc(fp));
    expect(track.role).toBe('progress-bar');
    expect(track.width).toBe(240);
    expect(track.height).toBe(8);
    expect(track.cornerRadius).toBe(4);
    const kids = track.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('progress-bar-fill');
    expect(kids[0].width).toBe(120);
  });

  it('custom value + bar_width', async () => {
    const fp = await fresh('a.op');
    await handleAddProgressBarV0({ filePath: fp, value: 25, bar_width: 400 });
    const track = getRoot(await readDoc(fp));
    expect(track.width).toBe(400);
    expect((track.children as Record<string, unknown>[])[0].width).toBe(100);
  });

  it('value=0: no fill child emitted', async () => {
    const fp = await fresh('a.op');
    await handleAddProgressBarV0({ filePath: fp, value: 0 });
    const track = getRoot(await readDoc(fp));
    expect((track.children as unknown[]).length).toBe(0);
  });

  it('value clamped above 100 and below 0', async () => {
    const fp = await fresh('a.op');
    await handleAddProgressBarV0({ filePath: fp, value: 150 });
    const track = getRoot(await readDoc(fp));
    expect((track.children as Record<string, unknown>[])[0].width).toBe(240);
    invalidateCache(fp);
    await writeFile(fp, EMPTY, 'utf-8');
    await handleAddProgressBarV0({ filePath: fp, value: -50 });
    const track2 = getRoot(await readDoc(fp));
    expect((track2.children as unknown[]).length).toBe(0);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddProgressBarV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
