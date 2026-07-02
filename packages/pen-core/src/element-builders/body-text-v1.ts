import { detectCjkScript } from './cjk-detect.js';
import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface BodyTextV1Params {
  content: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_body_text_v0.
   * - `'dark'`: byte-parity with light (body text has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildBodyText` emits no fill literals — text color is inherited from
   * the canvas default. Theme param is accepted for API consistency with
   * other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Body / description text — theme-aware version of buildBodyText.
 * Light mode is byte-equal to add_body_text_v0.
 *
 * CJK detection is preserved: CJK content gets lineHeight=1.6 + letterSpacing=0;
 * Latin content gets lineHeight=1.5 with no letterSpacing override.
 *
 * Since buildBodyText emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildBodyTextV1(params: BodyTextV1Params): ElementTree {
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
