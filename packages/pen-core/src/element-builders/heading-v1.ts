import { cjkFontFamily, detectCjkScript } from './cjk-detect.js';
import { coerceEnum } from './coerce-params.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';
import type { ElementTree } from './helpers.js';
import type { HeadingLevel, HeadingParams } from './heading.js';

export interface HeadingV1Params extends HeadingParams {
  /** Theme variant. Default `'light'` — byte-parity with v0 output. */
  theme?: V1Theme;
}

/**
 * Map heading level to the corresponding typography token keys in ThemeResolution.
 * Note: semantic palette sizes differ from v0 presets (v0: display=48/h1=32/h2=24/h3=20;
 * palette: display=64/h1=24/h2=20/h3=16). In 'light' mode, v0 values are used directly
 * to preserve byte-parity (Plan 14 contract). In 'system' mode, token refs are emitted
 * and resolve to palette values at paint time.
 */
const LEVEL_TO_TYPOGRAPHY = {
  display: {
    sizeKey: 'displaySize',
    weightKey: 'displayWeight',
    lineHeightKey: 'displayLineHeight',
    letterSpacingKey: 'displayLetterSpacing',
  },
  h1: { sizeKey: 'h1Size', weightKey: 'h1Weight', lineHeightKey: 'h1LineHeight' },
  h2: { sizeKey: 'h2Size', weightKey: 'h2Weight', lineHeightKey: 'h2LineHeight' },
  h3: { sizeKey: 'h3Size', weightKey: 'h3Weight', lineHeightKey: 'h3LineHeight' },
} as const;

/**
 * v0 byte-parity preset values for light mode (MUST match buildHeading exactly).
 * These are the actual v0 hardcoded values, NOT the palette token values
 * (palette: display=64/h1=24/h2=20/h3=16; v0: display=48/h1=32/h2=24/h3=20).
 */
const V0_LATIN_PRESETS = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.0, letterSpacing: -0.5 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.1 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.2 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.25 },
} as const;

const V0_CJK_BASE = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.3 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.3 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.35 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.4 },
} as const;

/**
 * Theme-aware typographic heading (v1).
 *
 * In 'light' mode (default): produces a tree byte-identical to v0's
 * `buildHeading()` output — same hardcoded size/weight/lineHeight presets
 * including CJK detection. (Plan 14 byte-parity contract).
 *
 * In 'dark' mode: same preset sizes but adds `$color-text-primary Dark`
 * (#F1F5F9) fill for visibility on dark surfaces.
 *
 * In 'system' mode: emits `$type-*` token refs for size/weight/lineHeight
 * and `$color-text-primary` ref for fill. Token values may differ from v0
 * presets (spec §3.1 accepts this drift).
 *
 * CJK detection always applies — letterSpacing is set to 0 for CJK content
 * (never negative), and fontFamily is set to the CJK-appropriate family.
 * In 'system' mode, only the color and numeric typography become refs;
 * fontFamily stays the concrete CJK family string if detected.
 */
export function buildHeadingV1(params: HeadingV1Params): ElementTree {
  const level = coerceEnum<HeadingLevel>(
    params.level,
    ['display', 'h1', 'h2', 'h3'],
    'h2',
    'buildHeadingV1',
    'level',
  );
  const theme = params.theme ?? 'light';
  const script = detectCjkScript(params.content);
  const cjkFont = cjkFontFamily(script);
  const typo = LEVEL_TO_TYPOGRAPHY[level];

  if (theme === 'light') {
    // Byte-parity with v0: use v0's hardcoded Latin / CJK preset tables
    const heading: ElementTree = {
      type: 'text',
      name: `Heading (${level})`,
      role: 'heading',
      content: params.content,
      fontSize: cjkFont ? V0_CJK_BASE[level].fontSize : V0_LATIN_PRESETS[level].fontSize,
      fontWeight: cjkFont ? V0_CJK_BASE[level].fontWeight : V0_LATIN_PRESETS[level].fontWeight,
      lineHeight: cjkFont ? V0_CJK_BASE[level].lineHeight : V0_LATIN_PRESETS[level].lineHeight,
    };
    if (cjkFont) {
      heading.fontFamily = cjkFont;
    } else if (level === 'display') {
      // Only display has letterSpacing in v0 Latin presets
      heading.letterSpacing = V0_LATIN_PRESETS.display.letterSpacing;
    }
    return heading;
  }

  // dark or system: use resolveTheme for color + typography refs
  const t = resolveTheme(theme);
  const heading: ElementTree = {
    type: 'text',
    name: `Heading (${level})`,
    role: 'heading',
    content: params.content,
    fontSize: cjkFont ? V0_CJK_BASE[level].fontSize : t.typography[typo.sizeKey],
    fontWeight: cjkFont ? V0_CJK_BASE[level].fontWeight : t.typography[typo.weightKey],
    lineHeight: cjkFont ? V0_CJK_BASE[level].lineHeight : t.typography[typo.lineHeightKey],
    fill: [{ type: 'solid', color: t.colors.textPrimary }],
  };
  if (cjkFont) {
    heading.fontFamily = cjkFont;
  } else if (level === 'display') {
    // letterSpacing for display: use sparse token in system, concrete value in dark
    heading.letterSpacing = t.typography.displayLetterSpacing;
  }
  return heading;
}
