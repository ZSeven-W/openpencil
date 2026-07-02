import type { StyleGuideValues } from './style-guide-parser';

export interface PropertyReplacement {
  fillColor?: { from: string; to: string }[];
  textColor?: { from: string; to: string }[];
  strokeColor?: { from: string; to: string }[];
  fontFamily?: { from: string; to: string }[];
  cornerRadius?: { from: number | number[]; to: number | number[] }[];
}

/**
 * Build a property replacement mapping from one style guide's values to another.
 * Used with replace_all_matching_properties MCP tool for "style switching".
 *
 * Routes colors to the correct channel:
 * - background/surface/accent → fillColor (frame backgrounds)
 * - textPrimary/textSecondary/textMuted → textColor (text fills)
 * - border → strokeColor (border strokes)
 *
 * Corner radii are emitted as scalar numbers (not arrays) to match PenNode storage.
 */
export function buildStyleMapping(
  from: StyleGuideValues,
  to: StyleGuideValues,
): PropertyReplacement {
  const fillColor: PropertyReplacement['fillColor'] = [];
  const textColor: PropertyReplacement['textColor'] = [];
  const strokeColor: PropertyReplacement['strokeColor'] = [];
  const fontFamily: PropertyReplacement['fontFamily'] = [];
  const cornerRadius: PropertyReplacement['cornerRadius'] = [];

  // --- Fill colors (backgrounds, surfaces, accents) ---
  const fillKeys = ['background', 'surface', 'accent'] as const;
  for (const key of fillKeys) {
    const f = from.colors[key];
    const t = to.colors[key];
    if (f && t && f !== t) {
      fillColor.push({ from: f, to: t });
    }
  }

  // --- Text colors ---
  const textKeys = ['textPrimary', 'textSecondary', 'textMuted'] as const;
  for (const key of textKeys) {
    const f = from.colors[key];
    const t = to.colors[key];
    if (f && t && f !== t) {
      textColor.push({ from: f, to: t });
    }
  }

  // --- Border/stroke colors ---
  if (from.colors.border && to.colors.border && from.colors.border !== to.colors.border) {
    strokeColor.push({ from: from.colors.border, to: to.colors.border });
  }

  // --- Font family replacements ---
  const fontKeys = ['displayFont', 'bodyFont', 'dataFont'] as const;
  for (const key of fontKeys) {
    const f = from.typography[key];
    const t = to.typography[key];
    if (f && t && f !== t) {
      fontFamily.push({ from: f, to: t });
    }
  }

  // --- Corner radius replacements (scalar numbers, matching PenNode storage) ---
  const radiusKeys = ['card', 'button'] as const;
  for (const key of radiusKeys) {
    const f = from.radius[key];
    const t = to.radius[key];
    if (f !== undefined && t !== undefined && f !== t) {
      cornerRadius.push({ from: f, to: t });
    }
  }

  // Build result, omitting empty arrays
  const result: PropertyReplacement = {};
  if (fillColor.length > 0) result.fillColor = fillColor;
  if (textColor.length > 0) result.textColor = textColor;
  if (strokeColor.length > 0) result.strokeColor = strokeColor;
  if (fontFamily.length > 0) result.fontFamily = fontFamily;
  if (cornerRadius.length > 0) result.cornerRadius = cornerRadius;

  return result;
}
