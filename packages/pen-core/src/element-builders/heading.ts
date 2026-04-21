import { cjkFontFamily, detectCjkScript } from './cjk-detect.js';
import type { ElementTree } from './helpers.js';

export type HeadingLevel = 'display' | 'h1' | 'h2' | 'h3';

export interface HeadingParams {
  content: string;
  level?: HeadingLevel;
}

interface HeadingPreset {
  fontSize: number;
  fontWeight: number;
  lineHeight: number;
  letterSpacing?: number;
  fontFamily?: string;
}

const LATIN_PRESETS: Record<HeadingLevel, HeadingPreset> = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.0, letterSpacing: -0.5 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.1 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.2 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.25 },
};

// CJK presets (from project_pencil_optimization + text-rules.md):
//   - lineHeight 1.3-1.4 for headings (NOT 1.1-1.2 like Latin)
//   - letterSpacing: 0, NEVER negative (would cause CJK character overlap)
//   - fontFamily dispatched per script (SC/JP/KR); heading bar.
const CJK_BASE: Record<HeadingLevel, Omit<HeadingPreset, 'fontFamily'>> = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.3 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.3 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.35 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.4 },
};

/**
 * Typographic heading — single text node, typography preset chosen
 * by `level` + CJK detection. Structure is always one text node so
 * no "应拆尽拆" violation (enum controls typography, not structure).
 */
export function buildHeading(params: HeadingParams): ElementTree {
  const level = params.level ?? 'h2';
  const script = detectCjkScript(params.content);
  const cjkFont = cjkFontFamily(script);
  const preset: HeadingPreset = cjkFont
    ? { ...CJK_BASE[level], fontFamily: cjkFont }
    : LATIN_PRESETS[level];
  const heading: ElementTree = {
    type: 'text',
    name: `Heading (${level})`,
    role: 'heading',
    content: params.content,
    fontSize: preset.fontSize,
    fontWeight: preset.fontWeight,
    lineHeight: preset.lineHeight,
  };
  if (preset.letterSpacing !== undefined) heading.letterSpacing = preset.letterSpacing;
  if (preset.fontFamily !== undefined) heading.fontFamily = preset.fontFamily;
  return heading;
}
