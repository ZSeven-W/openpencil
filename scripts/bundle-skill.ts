/**
 * Pre-build
 * 脚本：读取 ../openpencil-skill/ 并生成 JSON 包，该包由 esbuild 嵌入到 CLI 二进制文件中。
 *
 * Usage：小圆面包 scripts/bundle-skill.ts
 * Output：apps/
 cli/src/commands/skill-bundle.json
 */

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// 通过 env 覆盖 Allow（对于单独克隆技能存储库的 CI 很有用）
const SKILL_ROOT = process.env.SKILL_ROOT
  ? resolve(process.env.SKILL_ROOT)
  : resolve(__dirname, '../../openpencil-skill');
const OUT = resolve(__dirname, '../apps/cli/src/commands/skill-bundle.json');

// Files 嵌入（相对于 openpencil-skill/）
const FILES = [
  'skills/openpencil-design/SKILL.md',
  '.claude-plugin/plugin.json',
  '.claude-plugin/marketplace.json',
  '.cursor-plugin/plugin.json',
  'package.json',
  'GEMINI.md',
  'gemini-extension.json',
];

function main(): void {
  if (!existsSync(SKILL_ROOT)) {
    console.error(`Skill repo not found at ${SKILL_ROOT}`);
    console.error('Skipping skill bundle — install command will use git clone fallback.');
    // Write an empty bundle so the build doesn't break
    writeFileSync(OUT, JSON.stringify({ version: '', files: {} }, null, 2) + '\n');
    return;
  }

  const pkg = JSON.parse(readFileSync(join(SKILL_ROOT, 'package.json'), 'utf-8'));
  const bundle: Record<string, string> = {};

  for (const file of FILES) {
    const fullPath = join(SKILL_ROOT, file);
    if (existsSync(fullPath)) {
      bundle[file] = readFileSync(fullPath, 'utf-8');
    }
  }

  const output = { version: pkg.version as string, files: bundle };
  writeFileSync(OUT, JSON.stringify(output, null, 2) + '\n');
  console.log(`Bundled ${Object.keys(bundle).length} skill files (v${pkg.version}) → ${OUT}`);
}

main();
