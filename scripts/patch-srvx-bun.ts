/**
 * Patch srvx NodeResponse for Bun compatibility.
 *
 * Problem: srvx's Node adapter wraps all Response objects in NodeResponse.
 * Although NodeResponse inherits from Response via prototype chain,
 * Bun's HTTP runtime uses an internal brand check that rejects it.
 *
 * Fix: Make NodeResponse constructor return a native Response when running in Bun.
 * This is safe because Bun doesn't need srvx's Node.js stream bridging.
 *
 * Run: bun scripts/patch-srvx-bun.ts (called automatically via postinstall)
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

// Find srvx node adapter in various possible locations
const candidates = [
  resolve(import.meta.dir, '../node_modules/srvx/dist/adapters/node.mjs'),
  ...(() => {
    try {
      // Bun hoists to .bun/ directory
      const bunDir = resolve(import.meta.dir, '../node_modules/.bun');
      const dirs = require('fs')
        .readdirSync(bunDir)
        .filter((d: string) => d.startsWith('srvx@'));
      return dirs.map((d: string) =>
        resolve(bunDir, d, 'node_modules/srvx/dist/adapters/node.mjs'),
      );
    } catch {
      return [];
    }
  })(),
];

let patched = false;
for (const filePath of candidates) {
  if (!existsSync(filePath)) continue;

  let code = readFileSync(filePath, 'utf-8');
  if (code.includes('__srvx_bun_patched__')) {
    console.log(`[patch-srvx-bun] Already patched: ${filePath}`);
    patched = true;
    continue;
  }

  // Replace the NodeResponse IIFE to return native Response in Bun
  const marker = 'const NodeResponse = /* @__PURE__ */ (() => {';
  const endMarker = 'return NodeResponse;\n})();';

  if (!code.includes(marker)) {
    console.warn(`[patch-srvx-bun] Could not find NodeResponse marker in ${filePath}`);
    continue;
  }

  // Insert a Bun bypass right after the class definition.
  // In Bun, replace NodeResponse with native Response constructor
  // so Bun's internal brand check passes.
  code = code.replace(
    endMarker,
    `// __srvx_bun_patched__ — Bun bypass: return native Response instead of NodeResponse
if (typeof globalThis.Bun !== 'undefined') {
  return NativeResponse;
}
return NodeResponse;
})();`,
  );

  writeFileSync(filePath, code);
  console.log(`[patch-srvx-bun] Patched: ${filePath}`);
  patched = true;
}

if (!patched) {
  console.warn('[patch-srvx-bun] No srvx node adapter found to patch');
}
