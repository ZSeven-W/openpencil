import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { getSemanticPaletteHex } from '../packages/pen-core/src/variables/semantic-palette.js';

const BUILDERS_DIR = 'packages/pen-core/src/element-builders';
const HEX_REGEX = /#[0-9A-Fa-f]{6}\b/g;

function extractHexFromFile(path: string): string[] {
  const content = readFileSync(path, 'utf-8');
  return Array.from(content.matchAll(HEX_REGEX), (m) => m[0].toUpperCase());
}

function main() {
  const v0Files = readdirSync(BUILDERS_DIR).filter(
    (f) =>
      f.endsWith('.ts') &&
      !f.includes('-v1') &&
      f !== 'index.ts' &&
      f !== 'helpers.ts' &&
      f !== 'resolve-theme.ts',
  );

  const v0HexSet = new Set<string>();
  const hexUsageMap = new Map<string, string[]>(); // hex → list of files using it
  for (const f of v0Files) {
    const hexes = extractHexFromFile(join(BUILDERS_DIR, f));
    hexes.forEach((h) => {
      v0HexSet.add(h);
      if (!hexUsageMap.has(h)) hexUsageMap.set(h, []);
      hexUsageMap.get(h)!.push(f);
    });
  }

  const lightTokens = getSemanticPaletteHex('Light');
  const darkTokens = getSemanticPaletteHex('Dark');
  const tokenHexSet = new Set<string>([
    ...Object.values(lightTokens)
      .filter((v): v is string => typeof v === 'string')
      .map((h) => h.toUpperCase()),
    ...Object.values(darkTokens)
      .filter((v): v is string => typeof v === 'string')
      .map((h) => h.toUpperCase()),
  ]);

  const covered = [...v0HexSet].filter((h) => tokenHexSet.has(h)).sort();
  const uncovered = [...v0HexSet].filter((h) => !tokenHexSet.has(h)).sort();
  const coverRate = covered.length / v0HexSet.size;

  console.log(`v0 distinct hex literals: ${v0HexSet.size}`);
  console.log(`covered by palette: ${covered.length}`);
  console.log(`uncovered: ${uncovered.length}`);
  console.log(`cover rate: ${(coverRate * 100).toFixed(1)}%`);
  console.log(`\nFirst 30 uncovered hex (with usage):`);
  uncovered.slice(0, 30).forEach((h) => {
    const users = hexUsageMap.get(h)!;
    console.log(
      `  ${h} — used in ${users.length} file(s): ${users.slice(0, 3).join(', ')}${users.length > 3 ? ` (+${users.length - 3} more)` : ''}`,
    );
  });

  if (coverRate < 0.95) {
    console.error(`\n❌ HARD GATE FAILED: cover rate ${(coverRate * 100).toFixed(1)}% < 95%`);
    console.error('Per spec §7.4, must pause and revisit D2 namespace boundary before P2.');
    process.exit(1);
  }
  console.log(`\n✅ HARD GATE PASSED (${(coverRate * 100).toFixed(1)}% ≥ 95%)`);
}

main();
