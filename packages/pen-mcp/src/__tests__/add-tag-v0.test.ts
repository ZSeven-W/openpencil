/**
 * Unit tests for add_tag_v0 — single closable filter chip.
 * Distinct surface from `add_badge_v0` (static label, no × icon) and
 * `add_chip_input_v0` (multi-tag input field with inline caret).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTagV0 } from '../tools/add-tag-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-tag-v0-tests');
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
  if (!root) throw new Error('expected root');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['tag.op', 'static.op', 'tone.op', 'parent.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_tag_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_tag_v0');
    expect(DESIGN_TOOL_NAMES.has('add_tag_v0')).toBe(true);
  });
});

describe('add_tag_v0 — structure', () => {
  it('default tag: label + × icon, slate tone, fit_content pill', async () => {
    const fp = await fresh('tag.op');
    await handleAddTagV0({
      filePath: fp,
      label: 'Status: Active',
    });
    const tag = getRoot(await readDoc(fp));
    expect(tag.role).toBe('tag');
    expect(tag.width).toBe('fit_content');
    expect(tag.height).toBe('fit_content');
    expect(tag.cornerRadius).toBe(12);
    expect((tag.fill as Array<{ color: string }>)[0].color).toBe('#F1F5F9');
    const kids = tag.children as Record<string, unknown>[];
    expect(kids.length).toBe(2); // label + remove icon
    expect(kids[0].role).toBe('tag-label');
    expect(kids[0].content).toBe('Status: Active');
    expect((kids[0].fill as Array<{ color: string }>)[0].color).toBe('#475569');
    expect(kids[1].role).toBe('tag-remove');
    expect(kids[1].iconFontName).toBe('x');
    expect(kids[1].iconFontFamily).toBe('lucide');
  });

  it('removable=false drops the × icon, leaving just the label', async () => {
    const fp = await fresh('static.op');
    await handleAddTagV0({
      filePath: fp,
      label: 'Read-only',
      removable: false,
    });
    const tag = getRoot(await readDoc(fp));
    const kids = tag.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('tag-label');
  });

  it('tone enum picks the bg/fg color pair', async () => {
    const expectations: Array<{
      tone: 'accent' | 'success' | 'warning' | 'error';
      bg: string;
      fg: string;
    }> = [
      { tone: 'accent', bg: '#DBEAFE', fg: '#2563EB' },
      { tone: 'success', bg: '#DCFCE7', fg: '#166534' },
      { tone: 'warning', bg: '#FEF3C7', fg: '#B45309' },
      { tone: 'error', bg: '#FEE2E2', fg: '#B91C1C' },
    ];
    for (const { tone, bg, fg } of expectations) {
      const fp = await fresh('tone.op');
      await handleAddTagV0({ filePath: fp, label: tone, tone });
      const tag = getRoot(await readDoc(fp));
      expect((tag.fill as Array<{ color: string }>)[0].color).toBe(bg);
      const label = (tag.children as Record<string, unknown>[])[0];
      expect((label.fill as Array<{ color: string }>)[0].color).toBe(fg);
      invalidateCache(fp);
      await writeFile(fp, EMPTY_DOC, 'utf-8');
    }
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('parent.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTagV0({
        filePath: fp,
        label: 'X',
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
