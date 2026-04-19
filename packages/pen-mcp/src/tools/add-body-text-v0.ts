import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  cjkFontFamily,
  detectCjkScript,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddBodyTextV0Params {
  content: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Body / description text with auto script detection.
 *
 * Font family selection (per text-rules.md + memory):
 *   - Japanese → 'Noto Sans JP'
 *   - Korean   → 'Noto Sans KR'
 *   - Chinese  → 'Noto Sans SC'
 *   - Latin    → 'Inter'
 *   NEVER uses 'Space Grotesk' / 'Manrope' for CJK (no CJK glyphs).
 *
 * lineHeight + letterSpacing:
 *   - CJK body lineHeight = 1.6 (memory: "CJK body 1.6-1.8 NOT 1.4-1.6")
 *   - Latin body lineHeight = 1.5
 *   - CJK letterSpacing = 0 (NEVER negative — causes CJK character overlap)
 *   - Latin letterSpacing = default theme (not overridden)
 *
 * Always sets width=fill_container + textGrowth='fixed-width' per
 * overflow.md so long body text wraps. Intended for VERTICAL-layout
 * parents only (the documented context where fill_container+fixed-width
 * takes effect in the layout engine).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBodyTextV0(
  params: AddBodyTextV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const script = detectCjkScript(params.content);
  const cjkFont = cjkFontFamily(script);
  const body: Record<string, unknown> = {
    type: 'text',
    name: 'Body',
    role: 'body',
    content: params.content,
    fontSize: 16,
    fontWeight: 400,
    fontFamily: cjkFont ?? 'Inter',
    lineHeight: cjkFont ? 1.6 : 1.5,
    width: 'fill_container',
    textGrowth: 'fixed-width',
  };
  if (cjkFont) body.letterSpacing = 0;
  assignIdsRecursively(body);
  return insertElementTree({ binding: 'body', tree: body, ...params });
}
