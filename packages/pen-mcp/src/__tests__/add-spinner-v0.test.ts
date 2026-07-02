import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSpinnerV0 } from '../tools/add-spinner-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-spinner-v0');
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

describe('add_spinner_v0', () => {
  it('registered; no required params', () => {
    expect(DESIGN_TOOL_NAMES.has('add_spinner_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_spinner_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default: 32×32 frame + 2 ellipses (track + active arc)', async () => {
    const fp = await fresh('a.op');
    await handleAddSpinnerV0({ filePath: fp });
    const sp = getRoot(await readDoc(fp));
    expect(sp.role).toBe('spinner');
    expect(sp.width).toBe(32);
    expect(sp.layout).toBe('none');
    const kids = sp.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].role).toBe('spinner-track');
    expect(kids[1].role).toBe('spinner-arc');
    // Track is full ring (no startAngle); arc is 270° starting at -90
    expect(kids[0].startAngle).toBeUndefined();
    expect(kids[1].startAngle).toBe(-90);
    expect(kids[1].sweepAngle).toBe(270);
  });

  it('size clamped 16..128', async () => {
    const fp = await fresh('a.op');
    await handleAddSpinnerV0({ filePath: fp, size: 8 });
    expect(getRoot(await readDoc(fp)).width).toBe(16);

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddSpinnerV0({ filePath: fp, size: 999 });
    expect(getRoot(await readDoc(fp)).width).toBe(128);
  });

  it('thickness clamped 1..16', async () => {
    const fp = await fresh('a.op');
    await handleAddSpinnerV0({ filePath: fp, thickness: 99 });
    const arc = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const stroke = arc.stroke as { thickness: number };
    expect(stroke.thickness).toBe(16);
  });

  it('custom colors applied', async () => {
    const fp = await fresh('a.op');
    await handleAddSpinnerV0({
      filePath: fp,
      track_color: '#FFFFFF',
      active_color: '#FF0000',
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const trackStroke = kids[0].stroke as { fill: Array<{ color: string }> };
    const arcStroke = kids[1].stroke as { fill: Array<{ color: string }> };
    expect(trackStroke.fill[0].color).toBe('#FFFFFF');
    expect(arcStroke.fill[0].color).toBe('#FF0000');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddSpinnerV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
