import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  cjkFontFamily,
  detectCjkScript,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export type AddHeadingV0Level = 'display' | 'h1' | 'h2' | 'h3';

export interface AddHeadingV0Params {
  content: string;
  level?: AddHeadingV0Level;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Typographic heading with Pencil-demo-derived size / weight / lineHeight
 * presets. The `level` parameter does NOT change output structure (always
 * a single `text` node) — only typography — so it is an acceptable enum
 * under "应拆尽拆" (no structural branching).
 *
 * Separate presets for Latin vs CJK per memory Pencil optimization data.
 * Non-Claude failure modes this prevents:
 *   - "1.5 default stacks multi-word Latin headings too tight" (all levels
 *     get explicit lineHeight)
 *   - "Space Grotesk / Manrope has no CJK glyphs" — CJK content gets
 *     fontFamily='Noto Sans SC'
 *   - "CJK headings at lineHeight 1.1 look cramped" — CJK uses 1.3-1.4
 *     per memory rule "CJK headings 1.3-1.4 (NOT 1.1-1.2 like Latin)"
 *   - "Negative letterSpacing causes CJK character overlap" — CJK content
 *     NEVER gets negative letterSpacing (display preset drops it)
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
interface HeadingPreset {
  fontSize: number;
  fontWeight: number;
  lineHeight: number;
  letterSpacing?: number;
  fontFamily?: string;
}

const LATIN_PRESETS: Record<AddHeadingV0Level, HeadingPreset> = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.0, letterSpacing: -0.5 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.1 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.2 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.25 },
};

// CJK presets (from project_pencil_optimization memory + text-rules.md):
//   - lineHeight 1.3-1.4 for headings (NOT 1.1-1.2 like Latin)
//   - letterSpacing: 0, NEVER negative (negative causes character overlap)
//   - fontFamily: script-specific — Chinese Noto Sans SC, Japanese Noto
//     Sans JP, Korean Noto Sans KR. Using SC for JP/KR is technically
//     possible (broad Unicode coverage) but violates the repo's explicit
//     font contract. Each script's dedicated Noto face handles
//     font-native punctuation + glyph variants correctly.
const CJK_BASE: Record<AddHeadingV0Level, Omit<HeadingPreset, 'fontFamily'>> = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.3 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.3 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.35 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.4 },
};

export async function handleAddHeadingV0(
  params: AddHeadingV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const level = params.level ?? 'h2';
  const script = detectCjkScript(params.content);
  const cjkFont = cjkFontFamily(script);
  const preset: HeadingPreset = cjkFont
    ? { ...CJK_BASE[level], fontFamily: cjkFont }
    : LATIN_PRESETS[level];
  const heading: Record<string, unknown> = {
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
  assignIdsRecursively(heading);
  return insertElementTree({ binding: 'h', tree: heading, ...params });
}
