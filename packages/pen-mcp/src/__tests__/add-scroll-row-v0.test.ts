/**
 * Unit tests for add_scroll_row_v0 — MVP element tool (Step 2 of N-tool spec v0).
 *
 * The tool builds a horizontal scroll row that strictly follows the
 * nested wrapper pattern taught in
 * `packages/pen-ai-skills/skills/phases/generation/overflow.md` §HORIZONTAL
 * SCROLL ROWS. These tests lock down:
 *
 *   - Registration in production DESIGN_TOOL_DEFINITIONS (not gated)
 *   - Outer wrapper: fill_container / fit_content / vertical / clipContent=true
 *   - Inner row:     fit_content / horizontal / gap / padding=[0,20]
 *   - Each child kind has the correct fixed width, cornerRadius, padding
 *   - icon / subtitle are optional; title always present
 *   - card_width / gap overrides propagate
 *   - Parent resolution (null root vs parent_id)
 *   - Round-trip through handleBatchDesign persists to disk
 *
 * See spec openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7.1
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddScrollRowV0 } from '../tools/add-scroll-row-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-scroll-row-v0-tests');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP_DIR, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
  return fp;
}

async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}

function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const pageChildren = pages?.[0]?.children;
  const topChildren = doc['children'] as Record<string, unknown>[] | undefined;
  const root = pageChildren?.[0] ?? topChildren?.[0];
  if (!root) throw new Error('expected root child');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['cards.op', 'metrics.op', 'nav.op', 'min.op', 'override.op', 'persist.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_scroll_row_v0 — registration', () => {
  it('is registered in production DESIGN_TOOL_DEFINITIONS', () => {
    const names = DESIGN_TOOL_DEFINITIONS.map((t) => t.name);
    expect(names).toContain('add_scroll_row_v0');
  });

  it('is in DESIGN_TOOL_NAMES', () => {
    expect(DESIGN_TOOL_NAMES.has('add_scroll_row_v0')).toBe(true);
  });

  it('has required fields children_type and items', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_scroll_row_v0');
    expect(def).toBeDefined();
    expect(def?.inputSchema.required).toEqual(['children_type', 'items']);
  });
});

describe('add_scroll_row_v0 — wrapper structure invariants', () => {
  it('outer wrapper has fill_container + clipContent + vertical layout', async () => {
    const fp = await fresh('cards.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'Hello' }],
    });
    const doc = await readDoc(fp);
    const wrapper = getRoot(doc);
    expect(wrapper.type).toBe('frame');
    expect(wrapper.role).toBe('scroll-row-wrapper');
    expect(wrapper.width).toBe('fill_container');
    expect(wrapper.height).toBe('fit_content');
    expect(wrapper.layout).toBe('vertical');
    expect(wrapper.clipContent).toBe(true);
  });

  it('inner row has fit_content + horizontal + padding=[0,20]', async () => {
    const fp = await fresh('cards.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'A' }, { title: 'B' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const innerRow = (wrapper.children as Record<string, unknown>[])[0];
    expect(innerRow.role).toBe('scroll-row');
    expect(innerRow.width).toBe('fit_content');
    expect(innerRow.height).toBe('fit_content');
    expect(innerRow.layout).toBe('horizontal');
    expect(innerRow.padding).toEqual([0, 20]);
    expect(innerRow.gap).toBe(12); // default
    expect((innerRow.children as unknown[]).length).toBe(2);
  });
});

describe('add_scroll_row_v0 — children_type: card', () => {
  it('default card is 140×160, cornerRadius=20, padding=16', async () => {
    const fp = await fresh('cards.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'Only' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    const card = (row.children as Record<string, unknown>[])[0];
    expect(card.width).toBe(140);
    expect(card.height).toBe(160);
    expect(card.cornerRadius).toBe(20);
    expect(card.padding).toBe(16);
    expect(card.role).toBe('card');
  });

  it('card with title + subtitle + icon has all 3 children with correct roles', async () => {
    const fp = await fresh('cards.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'Hiit', subtitle: '30 min', icon: 'flame' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    const card = (row.children as Record<string, unknown>[])[0];
    const kids = card.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('flame');
    expect(kids[0].iconFontFamily).toBe('lucide');
    expect(kids[1].type).toBe('text');
    expect(kids[1].content).toBe('Hiit');
    expect(kids[1].role).toBe('heading');
    expect(kids[2].type).toBe('text');
    expect(kids[2].content).toBe('30 min');
    expect(kids[2].role).toBe('body');
  });

  it('card with only title has just one text child', async () => {
    const fp = await fresh('min.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'Bare' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const card = (
      (wrapper.children as Record<string, unknown>[])[0].children as Record<string, unknown>[]
    )[0];
    const kids = card.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('Bare');
  });
});

describe('add_scroll_row_v0 — children_type: metric_tile', () => {
  it('default metric_tile is 120×100, cornerRadius=16', async () => {
    const fp = await fresh('metrics.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'metric_tile',
      items: [{ title: 'Steps', subtitle: '8,432' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    const tile = (
      (wrapper.children as Record<string, unknown>[])[0].children as Record<string, unknown>[]
    )[0];
    expect(tile.width).toBe(120);
    expect(tile.height).toBe(100);
    expect(tile.cornerRadius).toBe(16);
    expect(tile.role).toBe('metric-tile');
    const kids = tile.children as Record<string, unknown>[];
    expect(kids[0].content).toBe('Steps');
    expect(kids[0].role).toBe('body');
    expect(kids[1].content).toBe('8,432');
    expect(kids[1].role).toBe('heading');
    expect(kids[1].fontSize).toBe(28);
  });
});

describe('add_scroll_row_v0 — children_type: nav_item', () => {
  it('default nav_item is 72 wide with fit_content height + alignItems=center', async () => {
    const fp = await fresh('nav.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'nav_item',
      items: [
        { title: 'Home', icon: 'home' },
        { title: 'Search', icon: 'search' },
      ],
    });
    const wrapper = getRoot(await readDoc(fp));
    const items = (wrapper.children as Record<string, unknown>[])[0].children as Record<
      string,
      unknown
    >[];
    expect(items.length).toBe(2);
    expect(items[0].width).toBe(72);
    expect(items[0].height).toBe('fit_content');
    expect(items[0].alignItems).toBe('center');
    expect(items[0].role).toBe('nav-item');
  });
});

describe('add_scroll_row_v0 — overrides', () => {
  it('respects card_width and gap overrides', async () => {
    const fp = await fresh('override.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'A' }, { title: 'B' }],
      card_width: 220,
      gap: 24,
    });
    const wrapper = getRoot(await readDoc(fp));
    const row = (wrapper.children as Record<string, unknown>[])[0];
    expect(row.gap).toBe(24);
    const cards = row.children as Record<string, unknown>[];
    expect(cards[0].width).toBe(220);
    expect(cards[1].width).toBe(220);
  });
});

describe('add_scroll_row_v0 — persistence', () => {
  it('result.nodeCount reflects full node tree (wrapper+row+cards+children)', async () => {
    const fp = await fresh('persist.op');
    const result = await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [{ title: 'A', subtitle: 's' }, { title: 'B', icon: 'x' }, { title: 'C' }],
    });
    // wrapper (1) + row (1) + 3 cards + [2 card-A children + 2 card-B children + 1 card-C children] = 10
    expect(result.nodeCount).toBe(10);
    expect(result.results).toHaveLength(1);
    expect(result.results[0].binding).toBe('row');
    expect(result.errors).toBeUndefined();
  });
});

describe('add_scroll_row_v0 — snapshot golden output', () => {
  it('golden: 3 cards with title+subtitle+icon', async () => {
    const fp = await fresh('cards.op');
    await handleAddScrollRowV0({
      filePath: fp,
      children_type: 'card',
      items: [
        { title: 'Hiit', subtitle: '30 min', icon: 'flame' },
        { title: 'Strength', subtitle: '45 min', icon: 'dumbbell' },
        { title: 'Yoga', subtitle: '25 min', icon: 'leaf' },
      ],
    });
    const wrapper = getRoot(await readDoc(fp));
    // Strip ids for stable snapshot
    const stripIds = (n: unknown): unknown => {
      if (Array.isArray(n)) return n.map(stripIds);
      if (n && typeof n === 'object') {
        const o = { ...(n as Record<string, unknown>) };
        delete o.id;
        for (const k of Object.keys(o)) o[k] = stripIds(o[k]);
        return o;
      }
      return n;
    };
    expect(stripIds(wrapper)).toMatchSnapshot('scroll-row-cards-golden');
  });
});
