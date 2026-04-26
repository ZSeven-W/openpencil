import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

/**
 * Browser-safety gate: the pure batch_design DSL executor and its
 * transitive import tree MUST NOT reference node-only modules
 * (`node:fs`, `node:os`, `node:path`, etc.) or pen-mcp modules that
 * themselves pull those in (`document-manager` is the canonical
 * offender).
 *
 * This is checked structurally: walk the import graph starting at
 * `batch-design-dsl.ts`, resolve relative imports to .ts files,
 * and fail if any reachable file contains a banned import.
 *
 * apps/web's browser bundle imports `runBatchDesignDsl` directly
 * from this module to skip the HTTP round-trip the M3 fallback
 * used. A regression that pulls in node:fs via a new transitive
 * import would fail at Vite build time in production but might be
 * missed in isolated tests — this guard catches it at unit-test time.
 */

const TOOLS_DIR = join(__dirname, '..', 'tools');
const SRC_ROOT = join(__dirname, '..');

const BANNED_NODE_IMPORTS = [
  "from 'node:fs",
  "from 'node:os'",
  "from 'node:path'",
  "from 'node:child_process'",
  "from 'node:crypto'",
];

const BANNED_INTERNAL_IMPORTS = [
  // document-manager is the node:fs chokepoint
  "from '../document-manager",
  "from './document-manager",
  // hooks is server-injected
  "from '../hooks'",
  "from './hooks'",
];

function collectReachable(entry: string, seen: Set<string> = new Set()): Set<string> {
  const normalized = resolve(entry);
  if (seen.has(normalized)) return seen;
  seen.add(normalized);
  let src: string;
  try {
    src = readFileSync(normalized, 'utf-8');
  } catch {
    return seen;
  }
  // Grab import specifiers
  const importRe = /from\s+['"]([^'"]+)['"]/g;
  for (const match of src.matchAll(importRe)) {
    const spec = match[1];
    if (spec.startsWith('.')) {
      // Relative — resolve to a .ts file in the same module
      const dir = dirname(normalized);
      let candidate = resolve(dir, spec);
      if (!candidate.endsWith('.ts')) {
        // Try `.ts` suffix (TS convention allows `.js` spec for `.ts` src)
        if (spec.endsWith('.js')) candidate = candidate.replace(/\.js$/, '.ts');
        else candidate = `${candidate}.ts`;
      }
      collectReachable(candidate, seen);
    }
    // External specs are skipped — they resolve to node_modules which we
    // treat as opaque (@zseven-w/pen-core, @zseven-w/pen-types, etc.
    // are verified pure by their own package tests).
  }
  return seen;
}

describe('batch-design-dsl — browser-safe transitive import tree', () => {
  const entry = join(TOOLS_DIR, 'batch-design-dsl.ts');
  const reachable = collectReachable(entry);

  it('entry point exists', () => {
    expect([...reachable].some((p) => p.endsWith('batch-design-dsl.ts'))).toBe(true);
  });

  it('reachable set contains at least a few files (sanity)', () => {
    expect(reachable.size).toBeGreaterThanOrEqual(3);
  });

  it('no node:* import in any reachable file', () => {
    const offenders: Array<{ file: string; hit: string }> = [];
    for (const file of reachable) {
      if (!file.startsWith(SRC_ROOT)) continue; // skip node_modules
      const src = readFileSync(file, 'utf-8');
      for (const banned of BANNED_NODE_IMPORTS) {
        if (src.includes(banned)) {
          offenders.push({ file, hit: banned });
        }
      }
    }
    if (offenders.length > 0) {
      const msg = offenders.map((o) => `  ${o.file}: ${o.hit}`).join('\n');
      throw new Error(
        `Pure DSL executor transitively imports node:* modules — browser bundle will break:\n${msg}`,
      );
    }
    expect(offenders).toEqual([]);
  });

  it('no document-manager or hooks import in any reachable file', () => {
    const offenders: Array<{ file: string; hit: string }> = [];
    for (const file of reachable) {
      if (!file.startsWith(SRC_ROOT)) continue;
      const src = readFileSync(file, 'utf-8');
      for (const banned of BANNED_INTERNAL_IMPORTS) {
        if (src.includes(banned)) {
          offenders.push({ file, hit: banned });
        }
      }
    }
    if (offenders.length > 0) {
      const msg = offenders.map((o) => `  ${o.file}: ${o.hit}`).join('\n');
      throw new Error(`Pure DSL executor transitively pulls in server-only module:\n${msg}`);
    }
    expect(offenders).toEqual([]);
  });

  it('batch-design.ts (the wrapper) DOES import node:fs-backed modules (sanity contrast)', () => {
    // Negative control: the wrapper file is NOT browser-safe. If this
    // starts failing, it means the wrapper lost its document-manager
    // import — unexpected.
    const wrapper = readFileSync(join(TOOLS_DIR, 'batch-design.ts'), 'utf-8');
    const importsDocManager = wrapper.includes("from '../document-manager");
    expect(importsDocManager).toBe(true);
  });

  it('package.json exposes `./dsl` subpath export pointing at the browser-safe file', () => {
    // Guards against a regression where someone removes the subpath
    // export — apps/web imports `@zseven-w/pen-mcp/dsl` specifically
    // to skip the barrel (which pulls node:fs via document-manager).
    // If this key disappears, Vite falls back to the `.` export, the
    // barrel resolves, and the browser build breaks.
    const pkgPath = join(__dirname, '..', '..', 'package.json');
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8')) as {
      exports?: Record<string, { types?: string; import?: string }>;
    };
    const dsl = pkg.exports?.['./dsl'];
    expect(dsl, 'package.json must expose `./dsl` subpath export').toBeDefined();
    expect(dsl?.import).toMatch(/batch-design-dsl\.ts$/);
    expect(dsl?.types).toMatch(/batch-design-dsl\.ts$/);
  });
});
