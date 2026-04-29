import {
  getSemanticPaletteHex,
  getSemanticPalette,
  SEMANTIC_PALETTE_THEME_LIGHT,
  SEMANTIC_PALETTE_THEME_DARK,
} from './semantic-palette.js';

/**
 * Built-in default values for every token in the semantic palette.
 * `resolveVariableRef` consults this map when `vars[refName]` is missing,
 * making "v1 'system' mode + un-seeded doc" byte-equivalent to "v0 hex output"
 * (per spec §5.3).
 *
 * Shape: `{ <token-name>: { light?, dark?, single? } }`
 *  - Light/dark color tokens have `{ light, dark }` (theme-axis tokens)
 *  - Chart color tokens have `{ single: string }` (single-value color)
 *  - Typography/spacing/radius tokens have `{ single: number }`
 */
export interface FallbackEntry {
  light?: string;
  dark?: string;
  single?: string | number;
}

export const DEFAULT_PALETTE_FALLBACK: Record<string, FallbackEntry> = (() => {
  const lightHex = getSemanticPaletteHex(SEMANTIC_PALETTE_THEME_LIGHT);
  const darkHex = getSemanticPaletteHex(SEMANTIC_PALETTE_THEME_DARK);
  const { variables } = getSemanticPalette();
  const fallback: Record<string, FallbackEntry> = {};

  for (const [name, def] of Object.entries(variables)) {
    const lightVal = lightHex[name];
    const darkVal = darkHex[name];

    if (typeof lightVal === 'string' && typeof darkVal === 'string' && lightVal !== darkVal) {
      // Theme-axis color token: { light, dark }
      fallback[name] = { light: lightVal, dark: darkVal };
    } else if (typeof lightVal === 'string') {
      // Single-value color token (chart palette) — same in both modes
      fallback[name] = { single: lightVal };
    } else {
      // Numeric token (typography / spacing / radius) — read from variables
      const val = def.value;
      if (typeof val === 'number') {
        fallback[name] = { single: val };
      }
    }
  }
  return fallback;
})();
