import { describe, it, expect, vi } from 'vitest';

vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import { ELEMENT_TOOL_NAMES } from '@zseven-w/pen-mcp';
import { ELEMENT_SHIMS, SUPPORTED_EMBEDDED_ELEMENT_TOOLS } from '../element-tool-shims';
import { CASES } from './shim-server-parity-cases';

/**
 * Bilateral drift guard: the tool name must exist identically in
 * three registries at runtime, and the two executable paths (client
 * shim + server SERVER_BUILDERS, both at apps/web) must produce
 * structurally equivalent trees for the same args.
 *
 *   Registry 1: ELEMENT_SHIMS keys (apps/web client shim)
 *   Registry 2: SUPPORTED_EMBEDDED_ELEMENT_TOOLS (exported list)
 *   Registry 3: ELEMENT_TOOL_NAMES (pen-mcp canonical list)
 *
 *   Executable path A: shim → buildX pen-core
 *   Executable path B: server /api/mcp/exec-tool → buildX pen-core
 *
 * The shim and server BOTH delegate to the same pen-core buildX
 * function. So the parity assertion is really "running buildX
 * directly and running buildX through the shim produce the same
 * structural tree" — which makes the shim a pure delegation layer
 * (plus id stamping + meta-param extraction).
 *
 * If any of these diverge, a real production bug becomes possible:
 *   - Name in pen-mcp but not in ELEMENT_SHIMS → client shim skips,
 *     HTTP fallback fires to Nitro, extra latency per tool call
 *   - Name in ELEMENT_SHIMS but not in pen-mcp → external MCP
 *     clients (Claude Code, Codex) don't know the tool exists
 *   - Shim output differs from buildX direct output → canvas render
 *     differs depending on which path was taken (the A/B nightmare)
 *
 * The CASES table itself lives in `shim-server-parity-cases.ts` so
 * this file stays under the 800-line ceiling.
 */

/**
 * Strip ids recursively — shims call `assignIdsRecursively` which
 * stamps random nanoids; direct buildX output has no ids yet. We
 * compare structurally on everything BUT the id field.
 */
function stripIds(n: unknown): unknown {
  if (Array.isArray(n)) return n.map(stripIds);
  if (n && typeof n === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(n as Record<string, unknown>)) {
      if (k === 'id') continue;
      out[k] = stripIds(v);
    }
    return out;
  }
  return n;
}

describe('registry parity — ELEMENT_SHIMS ⇄ ELEMENT_TOOL_NAMES ⇄ CASES', () => {
  it('CASES covers every ELEMENT_SHIMS key (no missing test)', () => {
    const caseNames = new Set(CASES.map((c) => c.toolName));
    const shimNames = Object.keys(ELEMENT_SHIMS);
    const missing = shimNames.filter((n) => !caseNames.has(n));
    expect(missing).toEqual([]);
  });

  it('CASES covers every SUPPORTED_EMBEDDED_ELEMENT_TOOLS entry', () => {
    const caseNames = new Set(CASES.map((c) => c.toolName));
    const missing = [...SUPPORTED_EMBEDDED_ELEMENT_TOOLS].filter((n) => !caseNames.has(n));
    expect(missing).toEqual([]);
  });

  it('SUPPORTED_EMBEDDED_ELEMENT_TOOLS is subset of ELEMENT_TOOL_NAMES (pen-mcp canonical)', () => {
    const canonical = new Set(ELEMENT_TOOL_NAMES);
    const missing = [...SUPPORTED_EMBEDDED_ELEMENT_TOOLS].filter((n) => !canonical.has(n));
    if (missing.length > 0) {
      throw new Error(
        `Shim registry names not present in pen-mcp canonical list: ${missing.join(', ')}. ` +
          `Either remove from shims or add the tool to pen-mcp.`,
      );
    }
    expect(missing).toEqual([]);
  });

  it('No duplicate entries in ELEMENT_SHIMS keys', () => {
    const keys = Object.keys(ELEMENT_SHIMS);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe('structural parity — shim output === buildX direct output', () => {
  for (const c of CASES) {
    it(`${c.toolName}: shim(args) === buildX(args) (ids aside)`, () => {
      const shim = ELEMENT_SHIMS[c.toolName];
      expect(shim, `shim for ${c.toolName}`).toBeDefined();

      // Shim path
      const shimResult = shim(c.args);
      const shimTree = stripIds(shimResult.node);

      // Direct buildX path (same args the shim would strip meta from
      // — but `args` has no meta fields, so it's identical to what
      // the shim would pass to the builder)
      const directTree = stripIds(c.build(c.args));

      expect(shimTree).toEqual(directTree);
    });
  }
});

describe('shim meta-param extraction', () => {
  it('parent_id extracted before builder invocation, not passed to buildX', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const result = shim({ content: 'Hello', parent_id: 'some-parent-id' });
    expect(result.parentId).toBe('some-parent-id');
    // Verify the heading node itself has no spurious parent_id leak
    expect((result.node as unknown as Record<string, unknown>).parent_id).toBeUndefined();
  });

  it('pageId extracted, defaults to null when absent', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const a = shim({ content: 'X', pageId: 'page-1' });
    expect(a.pageId).toBe('page-1');
    const b = shim({ content: 'Y' });
    expect(b.pageId).toBeNull();
  });

  it('filePath extracted, live://canvas sentinel normalized to null', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const a = shim({ content: 'X', filePath: 'live://canvas' });
    expect(a.filePath).toBeNull();
    const b = shim({ content: 'Y', filePath: '/tmp/real.pen' });
    expect(b.filePath).toBe('/tmp/real.pen');
  });
});
