import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTimelineV0 } from '../tools/add-timeline-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-timeline-v0');
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

describe('add_timeline_v0', () => {
  it('registered + required items', () => {
    expect(DESIGN_TOOL_NAMES.has('add_timeline_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_timeline_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('3 items: first item active, last drops connector', async () => {
    const fp = await fresh('a.op');
    await handleAddTimelineV0({
      filePath: fp,
      items: [
        { title: 'Order placed', subtitle: '10:42 AM', active: true },
        { title: 'Preparing' },
        { title: 'Shipped' },
      ],
    });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('timeline');
    expect(root.layout).toBe('vertical');
    const rows = root.children as Record<string, unknown>[];
    expect(rows.length).toBe(3);

    const firstIconCol = (rows[0].children as Record<string, unknown>[])[0];
    const firstDot = (firstIconCol.children as Record<string, unknown>[])[0];
    expect(firstDot.role).toBe('timeline-dot-active');
    // first has connector with a fixed non-zero height (pen-core has no
    // minHeight, so a fill_container connector would collapse to 0 when
    // content col is shorter than the 24px dot — see tool JSDoc)
    expect((firstIconCol.children as Record<string, unknown>[]).length).toBe(2);
    const firstConnector = (firstIconCol.children as Record<string, unknown>[])[1];
    expect(firstConnector.role).toBe('timeline-connector');
    expect(typeof firstConnector.height).toBe('number');
    expect(firstConnector.height as number).toBeGreaterThan(0);
    // icon column must be fit_content so row cross-axis is driven by the
    // dot+connector sum, guaranteeing the connector survives even when
    // content col is shorter than 24px
    expect(firstIconCol.height).toBe('fit_content');
    // No row padding and no outer gap — connector IS the inter-item
    // spacing. Any padding_bottom on the row (or gap on the outer
    // timeline) would insert empty space between connector-end and
    // next-dot-top, breaking the visual chain. See tool JSDoc.
    expect((rows[0] as Record<string, unknown>).padding).toBeUndefined();
    expect(root.gap).toBe(0);

    const secondDot = (
      (rows[1].children as Record<string, unknown>[])[0].children as Record<string, unknown>[]
    )[0];
    expect(secondDot.role).toBe('timeline-dot');

    // last row has NO connector
    const lastIconCol = (rows[2].children as Record<string, unknown>[])[0];
    expect((lastIconCol.children as Record<string, unknown>[]).length).toBe(1);
  });

  it('subtitle is optional; title-only rows produce single content text node', async () => {
    const fp = await fresh('a.op');
    await handleAddTimelineV0({
      filePath: fp,
      items: [{ title: 'Only title' }],
    });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const contentCol = (rows[0].children as Record<string, unknown>[])[1];
    const kids = contentCol.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].content).toBe('Only title');
  });

  it('empty items throws', async () => {
    const fp = await fresh('a.op');
    await expect(handleAddTimelineV0({ filePath: fp, items: [] })).rejects.toThrow(
      /at least one entry/,
    );
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTimelineV0({ filePath: fp, items: [{ title: 'x' }], parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
