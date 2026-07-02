import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type InputWithActionV1Kind = 'text' | 'icon';

export interface InputWithActionV1Params {
  /** Placeholder text shown when value is empty. Required. */
  placeholder: string;
  /** Pre-filled input value. Omit for placeholder state. */
  value?: string;
  /** Action button label. Required when action_kind="text". */
  action_label?: string;
  /** Lucide icon name. Required when action_kind="icon". */
  action_icon?: string;
  /** Kind of action button. Default `'text'`. */
  action_kind?: InputWithActionV1Kind;
  /** Optional leading icon shown inside the input. */
  leading_icon?: string;
  /** Total field width in px. Default 400. Min 280. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_input_with_action_v0.
   * - `'dark'`: input bg → surface, stroke → border, text → textPrimary,
   *             placeholder/leading icon → textMuted, button bg → accent
   *             (brand-invariant), button text/icon → white (#FFFFFF).
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

const FIELD_HEIGHT = 44;

/**
 * Input field with inline action button — theme-aware variant of
 * buildInputWithAction. Light mode is byte-equal to
 * add_input_with_action_v0.
 *
 * Color mapping:
 *   input bg         (#FFFFFF white)      → surface
 *   input stroke     (#CBD5E1 slate-300)  → border
 *   input value      (#0F172A slate-950)  → textPrimary
 *   placeholder/icon (#94A3B8 slate-400)  → textMuted
 *   leading icon     (#64748B slate-500)  → textMuted
 *   button bg        (#2563EB blue-600)   → accent (brand-invariant)
 *   button text/icon (#FFFFFF white)      → #FFFFFF (white on accent)
 */
export function buildInputWithActionV1(params: InputWithActionV1Params): ElementTree {
  const width = Math.max(280, Math.floor(params.width ?? 400));
  const kind: InputWithActionV1Kind = params.action_kind ?? 'text';
  const isFilled = params.value !== undefined && params.value !== '';
  const inputContent = isFilled ? params.value! : params.placeholder;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const inputBg = isLight ? '#FFFFFF' : t.colors.surface;
  const inputStroke = isLight ? '#CBD5E1' : t.colors.border;
  const inputTextColor = isLight
    ? isFilled
      ? '#0F172A'
      : '#94A3B8'
    : isFilled
      ? t.colors.textPrimary
      : t.colors.textMuted;
  const leadingIconColor = isLight ? '#64748B' : t.colors.textMuted;
  // accent + white-on-accent are brand-invariant
  const accentColor = isLight ? '#2563EB' : t.colors.accent;
  const onAccentColor = '#FFFFFF';

  const inputChildren: ElementTree[] = [];
  if (params.leading_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      role: 'input-with-action-leading-icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: leadingIconColor }],
    });
  }
  inputChildren.push({
    type: 'text',
    name: 'Text',
    role: 'input-with-action-text',
    content: inputContent,
    fontSize: 14,
    fontWeight: 400,
    fill: [{ type: 'solid', color: inputTextColor }],
  });

  const inputFrame: ElementTree = {
    type: 'frame',
    name: 'Input',
    role: 'input-with-action-input',
    width: 'fill_container',
    height: FIELD_HEIGHT,
    cornerRadius: 10,
    layout: 'horizontal',
    alignItems: 'center',
    gap: params.leading_icon ? 8 : 0,
    padding: [0, 14],
    fill: [{ type: 'solid', color: inputBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: inputStroke }] },
    children: inputChildren,
  };

  let buttonFrame: ElementTree;
  if (kind === 'icon') {
    const icon = params.action_icon ?? 'arrow-right';
    buttonFrame = {
      type: 'frame',
      name: 'Action',
      role: 'input-with-action-button',
      width: FIELD_HEIGHT,
      height: FIELD_HEIGHT,
      cornerRadius: 10,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [{ type: 'solid', color: accentColor }],
      children: [
        {
          type: 'icon_font',
          name: 'Action Icon',
          role: 'input-with-action-icon',
          iconFontName: icon,
          iconFontFamily: 'lucide',
          width: 18,
          height: 18,
          fill: [{ type: 'solid', color: onAccentColor }],
        },
      ],
    };
  } else {
    const label = params.action_label ?? 'Submit';
    buttonFrame = {
      type: 'frame',
      name: 'Action',
      role: 'input-with-action-button',
      width: 'fit_content',
      height: FIELD_HEIGHT,
      cornerRadius: 10,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      padding: [0, 20],
      fill: [{ type: 'solid', color: accentColor }],
      children: [
        {
          type: 'text',
          name: 'Action Label',
          role: 'input-with-action-label',
          content: label,
          fontSize: 14,
          fontWeight: 600,
          fill: [{ type: 'solid', color: onAccentColor }],
        },
      ],
    };
  }

  return {
    type: 'frame',
    name: 'Input With Action',
    role: 'input-with-action',
    width,
    height: FIELD_HEIGHT,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children: [inputFrame, buttonFrame],
  };
}
