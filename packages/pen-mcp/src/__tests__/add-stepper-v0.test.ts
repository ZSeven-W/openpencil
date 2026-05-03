import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddStepperV0 } from '../tools/add-stepper-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-stepper-v0');
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

describe('add_stepper_v0', () => {
  it('registered + required total', () => {
    expect(DESIGN_TOOL_NAMES.has('add_stepper_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_stepper_v0');
    expect(def?.inputSchema.required).toEqual(['total']);
  });

  it('3 total, current=1: step 0+1 done, connector 0 active, step 2 pending', async () => {
    const fp = await fresh('a.op');
    await handleAddStepperV0({ filePath: fp, total: 3, current: 1 });
    const s = getRoot(await readDoc(fp));
    expect(s.role).toBe('stepper');
    expect(s.width).toBe('fill_container');
    const kids = s.children as Record<string, unknown>[];
    // 3 steps + 2 connectors = 5
    expect(kids.length).toBe(5);
    expect(kids[0].role).toBe('step-active');
    expect(kids[1].role).toBe('step-connector-active');
    expect(kids[2].role).toBe('step-active');
    expect(kids[3].role).toBe('step-connector');
    expect(kids[4].role).toBe('step');
    // step numbers render 1-based
    const s2Text = (kids[2].children as Record<string, unknown>[])[0];
    expect(s2Text.content).toBe('2');
    // connectors use fill_container so they split space between circles
    expect(kids[1].width).toBe('fill_container');
    expect(kids[3].width).toBe('fill_container');
  });

  it('total=1: no connectors', async () => {
    const fp = await fresh('a.op');
    await handleAddStepperV0({ filePath: fp, total: 1 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('step-active');
  });

  it('current clamped above total-1', async () => {
    const fp = await fresh('a.op');
    await handleAddStepperV0({ filePath: fp, total: 3, current: 99 });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // all steps done, all connectors active
    expect(kids[0].role).toBe('step-active');
    expect(kids[2].role).toBe('step-active');
    expect(kids[4].role).toBe('step-active');
    expect(kids[1].role).toBe('step-connector-active');
    expect(kids[3].role).toBe('step-connector-active');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddStepperV0({ filePath: fp, total: 3, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
