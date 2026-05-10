/**
 * Color helpers shared across diagnostic detectors.
 *
 * Extracted from detectors.ts in 2026-05-10 when the second contrast-aware
 * detector (detectTextBgContrast) needed the same WCAG luminance math the
 * invisible-container detector already had inline. Kept narrow on purpose —
 * the only export surface is what detectors actually call.
 */

/**
 * Parse `#rgb` / `#rrggbb` / `#rrggbbaa` to `{r, g, b}` (alpha is dropped).
 * Returns null on parse failure or non-string input.
 */
export function parseHexColor(s: unknown): { r: number; g: number; b: number } | null {
  if (typeof s !== 'string') return null;
  const m = s.trim().match(/^#([0-9a-fA-F]{3,8})$/);
  if (!m) return null;
  let hex = m[1];
  if (hex.length === 3) {
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  }
  if (hex.length !== 6 && hex.length !== 8) return null;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return null;
  return { r, g, b };
}

/** WCAG 2.x relative luminance for sRGB. Returns 0.0–1.0. */
export function relativeLuminance(c: { r: number; g: number; b: number }): number {
  const lin = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

/**
 * Compare two color strings via WCAG relative-luminance contrast ratio.
 * Returns 1.0 for identical colors, growing toward 21.0 as they diverge,
 * or Infinity if either color cannot be parsed (e.g. unresolved variable
 * refs — caller should resolve refs upstream).
 *
 * Why WCAG ratio rather than max RGB channel diff: the human eye is much
 * more sensitive to small tonal differences on dark backgrounds than light
 * ones (Weber–Fechner / dark adaptation). Channel-diff would give false
 * positives in dark themes; ratio is luminance-based and matches the metric
 * WCAG / Stark / Figma report.
 */
export function colorContrast(a: string, b: string): number {
  if (a === b) return 1;
  const pa = parseHexColor(a);
  const pb = parseHexColor(b);
  if (!pa || !pb) return Infinity;
  const lumA = relativeLuminance(pa);
  const lumB = relativeLuminance(pb);
  const lighter = Math.max(lumA, lumB);
  const darker = Math.min(lumA, lumB);
  return (lighter + 0.05) / (darker + 0.05);
}
