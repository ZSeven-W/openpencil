/**
 * Patch srvx
 *
 * NodeResponse 用于 Bun 兼容性。 Problem：srvx 的 Node
 * 适配器将所有 Respo
 * nse 对象包装在 NodeResponse 中。 Although NodeResponse 通过原型链继承自 Response，Bun 的
 *
 * HTTP 运行时使用内部品牌检查来拒绝它。 Fix：Make NodeResponse 构造函数在 Bun 中运行时返回本机
 * Response。 This 是安全的，因为 Bun 不需要 srvx 的 Node.js 流桥接。 Run：bun
 *
 * scripts/patc
 h-srvx-bun.ts（通过安装后自动调用）
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

// Find srvx 节点适配器位于各个可能的位置
const candidates = [
  resolve(import.meta.dir, '../node_modules/srvx/dist/adapters/node.mjs'),
  ...(() => {
    try {
      // Bun 提升到 .bun/ 目录
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
    `// __srvx_bun_patched__ — Bun 绕过：返回本机 Response 而不是 NodeResponse
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
