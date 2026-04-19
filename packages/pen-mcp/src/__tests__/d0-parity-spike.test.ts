/**
 * D0 Parity Spike — validates the N-tool "additive-only" hypothesis by
 * adding a minimal `add_section_v0` tool and asserting:
 *
 *   1. `add_section_v0` is registered (new tool appears)
 *   2. Existing design tools' names and definitions are unchanged
 *      (snapshot-pinned to detect any drift)
 *   3. `batch_design` keeps producing identical output on a baseline
 *      fixture (proves no shared-helper contamination)
 *   4. `add_section_v0` produces expected PenNode via sugar over
 *      handleBatchDesign
 *
 * See spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSectionV0 } from '../tools/add-section-v0';
import { handleBatchDesign } from '../tools/batch-design';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-d0-parity-spike');

const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['add-h.op', 'add-v.op', 'batch-baseline.op', 'roundtrip.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('D0 parity spike — additive tool registration', () => {
  it('add_section_v0 is registered in DESIGN_TOOL_DEFINITIONS', () => {
    const names = DESIGN_TOOL_DEFINITIONS.map((t) => t.name);
    expect(names).toContain('add_section_v0');
  });

  it('add_section_v0 is registered in DESIGN_TOOL_NAMES', () => {
    expect(DESIGN_TOOL_NAMES.has('add_section_v0')).toBe(true);
  });

  it('existing design tool names are exactly the pre-spike set', () => {
    const names = DESIGN_TOOL_DEFINITIONS.map((t) => t.name)
      .filter((n) => n !== 'add_section_v0')
      .sort();
    expect(names).toEqual([
      'batch_design',
      'design_content',
      'design_refine',
      'design_skeleton',
      'get_design_prompt',
    ]);
  });

  it('existing tool DEFINITIONS are unchanged (snapshot)', () => {
    const existing = DESIGN_TOOL_DEFINITIONS.filter((t) => t.name !== 'add_section_v0');
    expect(existing).toMatchSnapshot('existing-design-tool-definitions');
  });
});

describe('D0 parity spike — batch_design non-interference', () => {
  it('batch_design inserts a rectangle (baseline)', async () => {
    const fp = join(TMP_DIR, 'batch-baseline.op');
    await writeFile(fp, EMPTY_DOC, 'utf-8');

    const result = await handleBatchDesign({
      filePath: fp,
      operations:
        'rect=I(null, { "type": "rectangle", "x": 0, "y": 0, "width": 100, "height": 50 })',
      postProcess: false,
    });

    expect(result.results).toHaveLength(1);
    expect(result.results[0]).toEqual({ binding: 'rect', nodeId: expect.any(String) });
    expect(result.nodeCount).toBe(1);
    expect(result.errors).toBeUndefined();
  });
});

describe('D0 parity spike — add_section_v0 behavior', () => {
  it('inserts a horizontal section frame', async () => {
    const fp = join(TMP_DIR, 'add-h.op');
    await writeFile(fp, EMPTY_DOC, 'utf-8');

    const result = await handleAddSectionV0({
      filePath: fp,
      title: 'Hero',
      layout: 'horizontal',
    });

    expect(result.results).toHaveLength(1);
    expect(result.results[0].binding).toBe('sec');
    expect(result.nodeCount).toBe(1);
    expect(result.errors).toBeUndefined();
  });

  it('inserts a vertical section frame', async () => {
    const fp = join(TMP_DIR, 'add-v.op');
    await writeFile(fp, EMPTY_DOC, 'utf-8');

    const result = await handleAddSectionV0({
      filePath: fp,
      title: 'Sidebar',
      layout: 'vertical',
    });

    expect(result.results).toHaveLength(1);
    expect(result.nodeCount).toBe(1);
  });

  it('persists title as frame.name and layout on disk', async () => {
    const fp = join(TMP_DIR, 'roundtrip.op');
    await writeFile(fp, EMPTY_DOC, 'utf-8');

    await handleAddSectionV0({
      filePath: fp,
      title: 'Test Section Title',
      layout: 'horizontal',
    });

    const raw = await readFile(fp, 'utf-8');
    const doc = JSON.parse(raw);
    const root = doc.pages?.[0]?.children?.[0] ?? doc.children?.[0];
    expect(root).toBeDefined();
    expect(root.type).toBe('frame');
    expect(root.name).toBe('Test Section Title');
    expect(root.layout).toBe('horizontal');
  });
});
