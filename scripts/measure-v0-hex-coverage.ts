import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { getSemanticPaletteHex } from '../packages/pen-core/src/variables/semantic-palette.js';

const BUILDERS_DIR = 'packages/pen-core/src/element-builders';
const HEX_REGEX = /#[0-9A-Fa-f]{6}\b/g;

/**
 * §3.4 Builder-private literals — these hex values are intentionally
 * hardcoded in v0/v1 builders and are NOT candidates for semantic tokens.
 * They are excluded from the cover-rate denominator.
 *
 * Rationale per design decision (2026-05-03 Codex review, B route):
 * - Gray/neutral scale (#111827 … #F9FAFB): CSS-reset / skeleton shades
 *   that don't carry semantic meaning and would pollute the palette.
 * - iOS system green (#34C759), iOS separator (#E5E5EA): platform-specific
 *   literals used only in iOS-flavored builders.
 * - Pure black (#000000): universal default, not a design token.
 */
const BUILDER_PRIVATE_HEX = new Set<string>(
  [
    '#000000',
    '#111827',
    '#4B5563',
    '#6B7280',
    '#9CA3AF',
    '#D1D5DB',
    '#E5E7EB',
    '#F9FAFB',
    '#34C759',
    '#E5E5EA',
  ].map((h) => h.toUpperCase()),
);

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

  const v0HexSetRaw = new Set<string>();
  const hexUsageMap = new Map<string, string[]>(); // hex → list of files using it
  for (const f of v0Files) {
    const hexes = extractHexFromFile(join(BUILDERS_DIR, f));
    hexes.forEach((h) => {
      v0HexSetRaw.add(h);
      if (!hexUsageMap.has(h)) hexUsageMap.set(h, []);
      hexUsageMap.get(h)!.push(f);
    });
  }

  // Apply §3.4 exclusion list — remove builder-private literals from denominator
  const v0HexSet = new Set([...v0HexSetRaw].filter((h) => !BUILDER_PRIVATE_HEX.has(h)));
  const excludedCount = v0HexSetRaw.size - v0HexSet.size;

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

  console.log(`v0 distinct hex literals (raw): ${v0HexSetRaw.size}`);
  console.log(`§3.4 builder-private excluded: ${excludedCount}`);
  console.log(`v0 semantic hex (post-exclusion): ${v0HexSet.size}`);
  console.log(`covered by palette: ${covered.length}`);
  console.log(`uncovered: ${uncovered.length}`);
  console.log(`semantic cover rate: ${(coverRate * 100).toFixed(1)}%`);

  if (uncovered.length > 0) {
    console.log(`\nUncovered semantic hex (with usage):`);
    uncovered.forEach((h) => {
      const users = hexUsageMap.get(h)!;
      console.log(
        `  ${h} — used in ${users.length} file(s): ${users.slice(0, 3).join(', ')}${users.length > 3 ? ` (+${users.length - 3} more)` : ''}`,
      );
    });
  }

  if (coverRate < 0.95) {
    console.error(`\n❌ HARD GATE FAILED: semantic cover rate ${(coverRate * 100).toFixed(1)}% < 95%`);
    console.error('Per spec §7.4, must pause and revisit D2 namespace boundary before P2.');
    process.exit(1);
  }
  console.log(`\n✅ HARD GATE PASSED (${(coverRate * 100).toFixed(1)}% ≥ 95%)`);
}

main();
