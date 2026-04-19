import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
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
 * Body / description text.
 *
 * Font family: ALWAYS 'Inter' (regardless of script).
 *   Rule source — packages/pen-ai-skills/skills/phases/generation/text-rules.md
 *   (the TEXT_RULES block in design-prompt.ts):
 *     "CJK font selection: heading='Noto Sans SC' (Chinese) / 'Noto Sans JP'
 *      (Japanese) / 'Noto Sans KR' (Korean), body='Inter'."
 *   Inter uses system CJK font fallback at render time — no need for a
 *   script-specific body face. cjk-typography.md also lists 'Inter (system
 *   CJK fallback) or Noto Sans SC' as the body options; we pick Inter to
 *   stay consistent across ALL scripts (no accidental SC for JP/KR).
 *
 *   The heading family DOES dispatch by script (SC/JP/KR) — that's what
 *   add_heading_v0 does. Body does not.
 *
 * lineHeight + letterSpacing (ARE script-sensitive):
 *   - CJK body: lineHeight=1.6, letterSpacing=0
 *     (memory: "CJK body 1.6-1.8 NOT 1.4-1.6"; "letterSpacing 0, NEVER
 *      negative — causes CJK character overlap")
 *   - Latin body: lineHeight=1.5, no letterSpacing override (theme default)
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
  const isCjk = detectCjkScript(params.content) !== null;
  const body: Record<string, unknown> = {
    type: 'text',
    name: 'Body',
    role: 'body',
    content: params.content,
    fontSize: 16,
    fontWeight: 400,
    fontFamily: 'Inter',
    lineHeight: isCjk ? 1.6 : 1.5,
    width: 'fill_container',
    textGrowth: 'fixed-width',
  };
  if (isCjk) body.letterSpacing = 0;
  assignIdsRecursively(body);
  return insertElementTree({ binding: 'body', tree: body, ...params });
}
