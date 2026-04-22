/**
 * Cross-tool contract tests for the element-tools family. Covers the
 * v0-MUST invariants from
 * openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §4:
 *
 *   - §4.1 Additive-only: pre-existing tools unchanged, new tools only
 *     add to ListTools
 *   - §4.2 schemaVersion: every element tool exposes `schemaVersion`
 *     as an input property (not just in description text)
 *   - §4.3 Error handling contract: throws on invalid parent_id / dsl
 *     failure rather than silent success
 *   - §4.4 CapabilityGate: filePath goes through resolveDocPath; no new
 *     saveDocument call sites (covered structurally — all element tools
 *     route through handleBatchDesign → saveDocument)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS } from '../routes/design-routes';
import { handleAddBottomNavV0 } from '../tools/add-bottom-nav-v0';
import { invalidateCache } from '../document-manager';

const ELEMENT_TOOL_NAMES = [
  'add_card_row_v0',
  'add_metric_row_v0',
  'add_nav_chip_row_v0',
  'add_bottom_nav_v0',
  'add_activity_ring_v0',
  'add_stat_grid_v0',
  'add_section_header_v0',
  'add_top_nav_bar_v0',
  'add_icon_button_v0',
  'add_divider_v0',
  'add_badge_v0',
  'add_avatar_v0',
  'add_text_button_v0',
  'add_heading_v0',
  'add_body_text_v0',
  'add_icon_label_v0',
  'add_list_row_v0',
  'add_search_bar_v0',
  'add_form_field_v0',
  'add_switch_v0',
  'add_checkbox_v0',
  'add_radio_v0',
  'add_tabs_v0',
  'add_segmented_control_v0',
  'add_empty_state_v0',
  'add_alert_v0',
  'add_toast_v0',
  'add_progress_bar_v0',
  'add_fab_v0',
  'add_breadcrumb_v0',
  'add_stepper_v0',
  'add_rating_stars_v0',
  'add_link_v0',
  'add_kbd_v0',
  'add_carousel_dots_v0',
  'add_price_v0',
  'add_quote_block_v0',
  'add_code_block_v0',
  'add_color_swatch_v0',
  'add_chart_bars_v0',
  'add_timeline_v0',
  'add_calendar_grid_v0',
  'add_pagination_v0',
  'add_faq_item_v0',
  'add_chip_input_v0',
  'add_empty_chart_v0',
  'add_action_menu_v0',
  'add_date_picker_v0',
  'add_modal_shell_v1',
  'add_upload_dropzone_v0',
];

describe('element tools — v0-MUST contract', () => {
  it('§4.1: all 5 element tools are registered in DESIGN_TOOL_DEFINITIONS', () => {
    const names = DESIGN_TOOL_DEFINITIONS.map((t) => t.name);
    for (const n of ELEMENT_TOOL_NAMES) {
      expect(names).toContain(n);
    }
  });

  it('§4.2: every element tool exposes schemaVersion as an input property', () => {
    for (const name of ELEMENT_TOOL_NAMES) {
      const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === name);
      expect(def, `tool ${name} must be registered`).toBeDefined();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const props = def?.inputSchema.properties as Record<string, any> | undefined;
      expect(
        props?.schemaVersion,
        `${name} must declare schemaVersion input property (§4.2)`,
      ).toBeDefined();
      expect(props?.schemaVersion.enum).toEqual(['1.0']);
      expect(props?.schemaVersion.type).toBe('string');
    }
  });

  it('§4.2: schemaVersion is NOT in required (backwards compat)', () => {
    for (const name of ELEMENT_TOOL_NAMES) {
      const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === name);
      const required = def?.inputSchema.required as string[] | undefined;
      expect(required ?? [], `${name} required fields`).not.toContain('schemaVersion');
    }
  });

  it('§4.4: every element tool accepts filePath (platform contract)', () => {
    for (const name of ELEMENT_TOOL_NAMES) {
      const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === name);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const props = def?.inputSchema.properties as Record<string, any> | undefined;
      expect(
        props?.filePath,
        `${name} must accept filePath per §4.4 platform contract`,
      ).toBeDefined();
      expect(props?.filePath.type).toBe('string');
    }
  });

  it('§4.4: every element tool accepts parent_id for sub-tree insertion', () => {
    for (const name of ELEMENT_TOOL_NAMES) {
      const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === name);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const props = def?.inputSchema.properties as Record<string, any> | undefined;
      expect(props?.parent_id, `${name} must accept parent_id`).toBeDefined();
      expect(props?.parent_id.type).toBe('string');
    }
  });

  it('names are final: no children_type / variant union fields leaked back in', () => {
    // "应拆尽拆" invariant — no multi-structure union type parameters
    for (const name of ELEMENT_TOOL_NAMES) {
      const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === name);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const props = def?.inputSchema.properties as Record<string, any> | undefined;
      expect(props?.children_type, `${name} must NOT have children_type`).toBeUndefined();
      expect(props?.variant, `${name} must NOT have variant union`).toBeUndefined();
    }
  });

  it('drift-guard: every add_*_v0 has a matching pen-core builder export', async () => {
    // Pairs with the apps/web `SUPPORTED_EMBEDDED_ELEMENT_TOOLS`
    // drift-guard — from the server side, verify each pen-mcp tool
    // name maps to a pen-core `buildX` export. Catches the case
    // where a handler is added/renamed but the pen-core builder
    // (the drift-free source) isn't.
    const penCore = await import('@zseven-w/pen-core');
    const missing: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      // add_xxx_v0 → buildXxx (camelCase)
      const stem = name.replace(/^add_/, '').replace(/_v\d+$/, '');
      const builderKey = `build${stem
        .split('_')
        .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
        .join('')}`;
      if (typeof (penCore as Record<string, unknown>)[builderKey] !== 'function') {
        missing.push(`${name} → ${builderKey}`);
      }
    }
    expect(
      missing,
      `Element tools without a pen-core builder — add the buildX export to ` +
        `packages/pen-core/src/element-builders/ (and wire it through the index.ts + ` +
        `src/index.ts barrel): ${missing.join(', ')}`,
    ).toEqual([]);
  });
});

// Regression test for the post-insert verification path in
// insertElementTree. Triggered when parent_id contains characters that
// batch_design's resolveRef (`/^"|"$/g` quote-strip, no JSON-unescape)
// cannot round-trip with JSON.stringify → literal mismatch → silent
// insertNodeInTree no-op. ensureParentExists happily passes because it
// uses the raw string for direct equality on the pre-insert tree.
describe('element tools — silent no-op guards (P0.1 + post-insert check)', () => {
  const TMP = join(tmpdir(), 'openpencil-element-silent-noop');
  beforeEach(async () => {
    await mkdir(TMP, { recursive: true });
  });
  afterEach(async () => {
    for (const f of ['quoted-parent.op']) {
      try {
        const fp = join(TMP, f);
        invalidateCache(fp);
        await unlink(fp);
      } catch {}
    }
  });

  it('throws (not silent success) when parent_id contains a literal quote', async () => {
    const fp = join(TMP, 'quoted-parent.op');
    // Manually construct a document whose parent frame has a quote in
    // its id. nanoid never produces such an id, but imported / migrated
    // documents could; we must not silently fail.
    await writeFile(
      fp,
      JSON.stringify({
        version: '1.0.0',
        children: [
          {
            id: 'weird"id',
            type: 'frame',
            name: 'Parent',
            width: 1200,
            height: 0,
            layout: 'vertical',
            children: [],
          },
        ],
      }),
      'utf-8',
    );
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddBottomNavV0({
        filePath: fp,
        parent_id: 'weird"id',
        items: [{ title: 'Home', icon: 'home' }],
      }),
    ).rejects.toThrow(
      /cannot be safely passed through batch_design's DSL parser|silently failed|not present in the document/,
    );
    // File must remain unchanged (no orphan tree written)
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });

  it('throws when DSL-resolved parent_id collides with a different node (wrong-parent insert)', async () => {
    // Seed the document with TWO frames:
    //   - intended parent, id = `A"B` (3 chars: A, ", B) — the user's target
    //   - decoy parent,    id = `A\"B` (4 chars: A, \, ", B) — happens to
    //     be what batch_design's resolveRef produces after quote-strip on
    //     our JSON.stringify output for 'A"B'
    // Expected: ensureParentExists finds the intended parent (raw string
    // match ✓), but insertNodeInTree resolves to the decoy (literal match
    // on the 4-char id). Without parent-location verification, the node
    // would silently land under the wrong parent.
    const fp = join(TMP, 'quoted-parent.op');
    await writeFile(
      fp,
      JSON.stringify({
        version: '1.0.0',
        children: [
          {
            id: 'A"B',
            type: 'frame',
            name: 'Intended Parent',
            width: 1200,
            height: 0,
            layout: 'vertical',
            children: [],
          },
          {
            id: 'A\\"B',
            type: 'frame',
            name: 'Decoy Parent',
            width: 1200,
            height: 0,
            layout: 'vertical',
            children: [],
          },
        ],
      }),
      'utf-8',
    );
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddBottomNavV0({
        filePath: fp,
        parent_id: 'A"B',
        items: [{ title: 'Home', icon: 'home' }],
      }),
    ).rejects.toThrow(/cannot be safely passed|wrong parent|not present/);
    // File must remain unchanged — either pre-check prevented the write, or
    // post-check detected a bad insert and rolled back to the snapshot.
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
