import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { ELEMENT_TOOL_DEFINITIONS, ELEMENT_TOOL_NAMES } from '../routes/element-tool-defs';

/**
 * pen-mcp handler × pen-core builder registry parity.
 *
 * The canonical source of truth: ELEMENT_TOOL_NAMES (derived from
 * ELEMENT_TOOL_DEFINITIONS). Each tool must have:
 *
 *   1. A tool definition in ELEMENT_TOOL_DEFINITIONS
 *   2. A handler file at `src/tools/<name-with-dashes>.ts` that
 *      exports `handle<Name>` and imports the matching pen-core
 *      builder
 *   3. A switch branch in handleElementToolCall that dispatches to
 *      the handler
 *
 * If any of these three diverge — e.g. definition without handler,
 * handler without dispatch, import of the wrong builder — this test
 * catches the drift at CI time, before external MCP clients (Claude
 * Code / Codex / Gemini CLI) get a 500 error on the missing tool or
 * a silent wrong-shape response.
 *
 * This test is the pen-mcp-side counterpart to
 * apps/web's shim-server-parity.test.ts.
 */

const TOOLS_DIR = join(__dirname, '..', 'tools');
const ROUTES_DIR = join(__dirname, '..', 'routes');

function toolNameToHandlerFile(toolName: string): string {
  // add_heading_v0 → add-heading-v0.ts
  return toolName.replace(/_/g, '-') + '.ts';
}

function toolNameToHandlerExport(toolName: string): string {
  // add_heading_v0 → handleAddHeadingV0
  const pascal = toolName
    .split('_')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');
  return 'handle' + pascal;
}

function readRouterFile(): string {
  return readFileSync(join(ROUTES_DIR, 'element-tool-defs.ts'), 'utf-8');
}

describe('pen-mcp element-tool registry parity', () => {
  it('ELEMENT_TOOL_DEFINITIONS has at least 42 tools (the expected coverage floor)', () => {
    expect(ELEMENT_TOOL_DEFINITIONS.length).toBeGreaterThanOrEqual(42);
  });

  it('Every tool name matches the add_X_vN convention (accepts _v0, _v1, …)', () => {
    for (const def of ELEMENT_TOOL_DEFINITIONS) {
      expect(def.name, `tool definition: ${def.name}`).toMatch(/^add_[a-z_]+_v\d+$/);
    }
  });

  it('Every tool has a handler file at the expected kebab-case path', () => {
    const actualFiles = new Set(
      readdirSync(TOOLS_DIR).filter((f) => f.endsWith('.ts') && f.startsWith('add-')),
    );

    const missing: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      const expected = toolNameToHandlerFile(name);
      if (!actualFiles.has(expected)) missing.push(`${name} → expected ${expected}`);
    }
    expect(missing).toEqual([]);
  });

  it('Every handler file exports the expected handle<PascalName> function', () => {
    const errors: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      const fp = join(TOOLS_DIR, toolNameToHandlerFile(name));
      const src = readFileSync(fp, 'utf-8');
      const expectedExport = toolNameToHandlerExport(name);
      const pattern = new RegExp(`export\\s+(async\\s+)?function\\s+${expectedExport}\\b`);
      if (!pattern.test(src)) {
        errors.push(`${name} in ${fp}: missing export \`${expectedExport}\``);
      }
    }
    expect(errors).toEqual([]);
  });

  it('Every handler imports its matching builder from @zseven-w/pen-core', () => {
    // tool name → expected builder name
    const expectedBuilder = (toolName: string): string => {
      const body = toolName.replace(/^add_/, '').replace(/_v0$/, '');
      const pascal = body
        .split('_')
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join('');
      return 'build' + pascal;
    };

    const errors: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      const fp = join(TOOLS_DIR, toolNameToHandlerFile(name));
      const src = readFileSync(fp, 'utf-8');
      const builderName = expectedBuilder(name);
      // Accept import forms: named import block, single-line, etc.
      const pattern = new RegExp(
        `import\\s+[^;]*\\b${builderName}\\b[^;]*from\\s+['\"]@zseven-w/pen-core['\"]`,
      );
      if (!pattern.test(src)) {
        errors.push(
          `${name} in ${fp}: handler does not import \`${builderName}\` from @zseven-w/pen-core`,
        );
      }
    }
    expect(errors).toEqual([]);
  });

  it('Every tool has a switch branch in handleElementToolCall', () => {
    const routerSrc = readRouterFile();
    const missing: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      const caseMatch = new RegExp(`case\\s+['\"]${name.replace(/\./g, '\\.')}['\"]:`);
      if (!caseMatch.test(routerSrc)) {
        missing.push(name);
      }
    }
    expect(missing).toEqual([]);
  });

  it('Every switch case dispatches to the handler exported from its handler file', () => {
    // Verify that inside the switch, the case 'X' body references
    // the corresponding handleX function (not some other name).
    const routerSrc = readRouterFile();
    const errors: string[] = [];
    for (const name of ELEMENT_TOOL_NAMES) {
      const handler = toolNameToHandlerExport(name);
      // Find the case line + grab the following few lines of body
      const caseIdx = routerSrc.indexOf(`case '${name}':`);
      if (caseIdx === -1) continue; // covered by previous test
      const bodyEnd = routerSrc.indexOf('case ', caseIdx + 1);
      const body = routerSrc.slice(caseIdx, bodyEnd === -1 ? undefined : bodyEnd);
      if (!body.includes(handler)) {
        errors.push(`${name}: switch case body does not call \`${handler}\``);
      }
    }
    expect(errors).toEqual([]);
  });

  it('No orphan handler files (every tools/add-*-v0.ts maps to an ELEMENT_TOOL_NAMES entry)', () => {
    const handlerFiles = readdirSync(TOOLS_DIR)
      .filter((f) => /^add-[a-z-]+-v0\.ts$/.test(f))
      .filter((f) => {
        // Skip `add-section-v0.ts` — it's from an older batch and
        // not part of the 42-tool element registry. Discriminate by
        // checking if its handler is imported into element-tool-defs.
        const src = readRouterFile();
        // Map file name back to handler name: add-section-v0.ts → handleAddSectionV0
        const toolName = f.replace(/\.ts$/, '').replace(/-/g, '_');
        const handler = toolNameToHandlerExport(toolName);
        return src.includes(handler);
      });

    const orphans: string[] = [];
    for (const f of handlerFiles) {
      const toolName = f.replace(/\.ts$/, '').replace(/-/g, '_');
      if (!ELEMENT_TOOL_NAMES.has(toolName)) {
        orphans.push(`${f} → expected tool name ${toolName}`);
      }
    }
    expect(orphans).toEqual([]);
  });

  it('No duplicate tool names in ELEMENT_TOOL_DEFINITIONS', () => {
    const names = ELEMENT_TOOL_DEFINITIONS.map((d) => d.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it('ELEMENT_TOOL_NAMES matches ELEMENT_TOOL_DEFINITIONS names exactly', () => {
    const defNames = new Set(ELEMENT_TOOL_DEFINITIONS.map((d) => d.name));
    const namesOnly = new Set(ELEMENT_TOOL_NAMES);
    expect([...namesOnly].sort()).toEqual([...defNames].sort());
  });
});
