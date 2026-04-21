/**
 * Script detection for text content. Used by element-tool builders to
 * pick the correct `fontFamily` per the repo's CJK font contract
 * documented in
 * `packages/pen-ai-skills/skills/phases/generation/text-rules.md`:
 *
 *   "CJK font selection: heading='Noto Sans SC' (Chinese) /
 *    'Noto Sans JP' (Japanese) / 'Noto Sans KR' (Korean),
 *    body='Inter'. NEVER use 'Space Grotesk' or 'Manrope' for CJK
 *    content — they have no CJK glyphs."
 *
 * Detection order matters:
 *   1. Hiragana / Katakana present → Japanese (these scripts are
 *      unique to Japanese even when mixed with Han ideographs)
 *   2. Hangul syllables present → Korean
 *   3. Any Han ideograph / CJK punctuation → Chinese (Simplified default)
 *   4. Otherwise null (Latin / other)
 *
 * Lives in `element-builders/` (not the more general `layout/` module)
 * because the output — a Noto Sans family name — is specifically the
 * font-selection policy shared between pen-mcp handlers and apps/web
 * client shims. Pen-core's broader `hasCjkText` helper answers a
 * boolean; this one answers "which font family".
 */
export type CjkScript = null | 'chinese' | 'japanese' | 'korean';

export function detectCjkScript(s: string): CjkScript {
  // Hiragana U+3040-309F + Katakana U+30A0-30FF → Japanese
  if (/[぀-ゟ゠-ヿ]/.test(s)) return 'japanese';
  // Hangul Syllables U+AC00-D7AF → Korean
  if (/[가-힯]/.test(s)) return 'korean';
  // CJK Symbols / Punctuation U+3000-303F + CJK Unified Ideographs U+4E00-9FFF → Chinese
  if (/[　-〿一-鿿]/.test(s)) return 'chinese';
  return null;
}

/**
 * Map a CJK script to its Noto Sans font family. Returns undefined
 * for non-CJK so callers can fall back to theme default (Inter for
 * body, theme heading for headings).
 */
export function cjkFontFamily(script: CjkScript): string | undefined {
  switch (script) {
    case 'japanese':
      return 'Noto Sans JP';
    case 'korean':
      return 'Noto Sans KR';
    case 'chinese':
      return 'Noto Sans SC';
    default:
      return undefined;
  }
}
