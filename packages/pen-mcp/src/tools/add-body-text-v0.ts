import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
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
 * Body / description text. Auto-detects CJK vs Latin to emit the correct
 * fontFamily + lineHeight — both are documented failure modes for
 * non-Claude models:
 *
 *   - CJK-detected content gets fontFamily='Noto Sans SC' and lineHeight=1.6
 *     (memory: "NEVER use Space Grotesk / Manrope for CJK — no CJK glyphs"
 *      and "CJK body lineHeight 1.6-1.8 NOT 1.4-1.6 like Latin").
 *   - Latin content gets fontFamily='Inter' and lineHeight=1.5.
 *
 * Also sets width=fill_container + textGrowth='fixed-width' per
 * overflow.md: long body text MUST wrap, not horizontally overflow.
 * This tool is intended for use inside VERTICAL-layout parents (the only
 * context where the fill_container+fixed-width rule takes effect per
 * the layout engine's documented behavior).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
const CJK_REGEX = /[\u3000-\u303f\u3040-\u309f\u30a0-\u30ff\u4e00-\u9fff\uac00-\ud7af]/;

export async function handleAddBodyTextV0(
  params: AddBodyTextV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const isCjk = CJK_REGEX.test(params.content);
  const body = {
    type: 'text',
    name: 'Body',
    role: 'body',
    content: params.content,
    fontSize: 16,
    fontWeight: 400,
    fontFamily: isCjk ? 'Noto Sans SC' : 'Inter',
    lineHeight: isCjk ? 1.6 : 1.5,
    letterSpacing: isCjk ? 0 : undefined,
    width: 'fill_container',
    textGrowth: 'fixed-width',
  };
  // Drop undefined fields so they don't surface in the emitted JSON.
  if (body.letterSpacing === undefined) {
    delete (body as Record<string, unknown>).letterSpacing;
  }
  assignIdsRecursively(body as Record<string, unknown>);
  return insertElementTree({ binding: 'body', tree: body as Record<string, unknown>, ...params });
}
