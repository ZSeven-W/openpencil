import { detectCjkScript } from './cjk-detect.js';
import type { ElementTree } from './helpers.js';

export interface BodyTextParams {
  content: string;
}

/**
 * Body / description text — always fontFamily='Inter' (per text-rules.md).
 * lineHeight + letterSpacing ARE script-sensitive:
 *   - CJK: lineHeight=1.6, letterSpacing=0 (prevent character overlap)
 *   - Latin: lineHeight=1.5, no letterSpacing override
 *
 * Always width=fill_container + textGrowth='fixed-width' so long
 * paragraphs wrap. Intended for VERTICAL-layout parents.
 */
export function buildBodyText(params: BodyTextParams): ElementTree {
  const isCjk = detectCjkScript(params.content) !== null;
  const body: ElementTree = {
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
  return body;
}
