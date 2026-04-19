import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddMetricRowV0 } from '../tools/add-metric-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-metric-row-v0');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
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

describe('add_metric_row_v0', () => {
  it('registered + required items (label+value)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_metric_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_metric_row_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
    const itemSchema = (def?.inputSchema.properties as any)?.items?.items;
    expect(itemSchema?.required).toEqual(['label', 'value']);
  });

  it('builds 120×100 tiles with label (body) + value (heading 28/700)', async () => {
    const fp = await fresh('a.op');
    await handleAddMetricRowV0({
      filePath: fp,
      items: [
        { label: 'Steps', value: '8,432', icon: 'activity' },
        { label: 'Kcal', value: '512' },
      ],
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    const tiles = row.children as Record<string, unknown>[];
    expect(tiles.length).toBe(2);
    expect(tiles[0].width).toBe(120);
    expect(tiles[0].height).toBe(100);
    expect(tiles[0].cornerRadius).toBe(16);
    expect(tiles[0].role).toBe('metric-tile');
    // icon + label + value
    const k0 = tiles[0].children as Record<string, unknown>[];
    expect(k0.length).toBe(3);
    expect(k0[0].type).toBe('icon_font');
    expect(k0[1].role).toBe('body');
    expect(k0[1].content).toBe('Steps');
    expect(k0[2].role).toBe('heading');
    expect(k0[2].content).toBe('8,432');
    expect(k0[2].fontSize).toBe(28);
    expect(k0[2].fontWeight).toBe(700);
    // no-icon variant: label + value only
    const k1 = tiles[1].children as Record<string, unknown>[];
    expect(k1.length).toBe(2);
    expect(k1[0].content).toBe('Kcal');
    expect(k1[1].content).toBe('512');
  });

  it('throws on bogus parent_id', async () => {
    const fp = await fresh('a.op');
    await expect(
      handleAddMetricRowV0({
        filePath: fp,
        items: [{ label: 'X', value: '1' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
  });
});
