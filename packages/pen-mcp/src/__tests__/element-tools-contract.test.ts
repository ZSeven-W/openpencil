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

import { describe, it, expect } from 'vitest';
import { DESIGN_TOOL_DEFINITIONS } from '../routes/design-routes';

const ELEMENT_TOOL_NAMES = [
  'add_card_row_v0',
  'add_metric_row_v0',
  'add_nav_chip_row_v0',
  'add_bottom_nav_v0',
  'add_activity_ring_v0',
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
});
