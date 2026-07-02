import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTooltipV0 } from '../tools/add-tooltip-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-tooltip-v0');
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

describe('add_tooltip_v0', () => {
  it('registered; required text', () => {
    expect(DESIGN_TOOL_NAMES.has('add_tooltip_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_tooltip_v0');
    expect(def?.inputSchema.required).toEqual(['text']);
  });

  it('default: tooltip-top role + dark pill + white text', async () => {
    const fp = await fresh('a.op');
    await handleAddTooltipV0({ filePath: fp, text: 'Click to edit' });
    const tt = getRoot(await readDoc(fp));
    expect(tt.role).toBe('tooltip-top');
    expect(tt.cornerRadius).toBe(6);
    expect(tt.padding).toEqual([6, 10]);
    const fill = tt.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#111827');
    const kids = tt.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].content).toBe('Click to edit');
    const textFill = kids[0].fill as Array<{ color: string }>;
    expect(textFill[0].color).toBe('#FFFFFF');
  });

  it('position sets role variant', async () => {
    const fp = await fresh('a.op');
    await handleAddTooltipV0({ filePath: fp, text: 'Bottom', position: 'bottom' });
    expect(getRoot(await readDoc(fp)).role).toBe('tooltip-bottom');

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddTooltipV0({ filePath: fp, text: 'Left', position: 'left' });
    expect(getRoot(await readDoc(fp)).role).toBe('tooltip-left');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTooltipV0({ filePath: fp, text: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
