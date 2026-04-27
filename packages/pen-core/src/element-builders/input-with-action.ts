import type { ElementTree } from './helpers.js';

export type InputWithActionKind = 'text' | 'icon';

export interface InputWithActionParams {
  /** Placeholder text shown when value is empty. Required. */
  placeholder: string;
  /** Pre-filled input value. Omit for placeholder state. */
  value?: string;
  /** Action button label (e.g. "Subscribe", "Apply", "Send"). Required when action_kind="text". */
  action_label?: string;
  /** Lucide icon name. Required when action_kind="icon". */
  action_icon?: string;
  /**
   * Kind of action button. Default `'text'` (label-only pill). Use
   * `'icon'` for icon-only square button (chat send / search apply).
   */
  action_kind?: InputWithActionKind;
  /** Optional leading icon shown inside the input. */
  leading_icon?: string;
  /** Total field width in px. Default 400. Min 280. */
  width?: number;
}

const FIELD_HEIGHT = 44;

/**
 * Input field with inline action button on the right — the
 * "Subscribe to newsletter" / "Apply discount" / "Send message"
 * pattern. Different from:
 *
 *   - `add_form_field_v0`: input with label-above, no inline button
 *   - `add_search_bar_v0`: input with leading search icon, no
 *     trailing action button
 *   - `add_chip_input_v0`: input that grows chip pills inline
 *
 * Two action variants:
 *
 *   - `text` (default): pill button with label like "Subscribe".
 *     Padding [12, 20], accent fill, white text.
 *   - `icon`: square 44×44 icon button (chat send arrow, search-
 *     apply magnifying glass). Centered icon only.
 *
 * Structure:
 *   frame(width, h=44, horizontal, gap=8, role='input-with-action')
 *     ├ frame(input, fill_container, cornerRadius=10, stroke=slate-300,
 *     │       layout=horizontal, align=center, role='input-with-action-input')
 *     │   ├ icon_font(leading_icon, role='input-with-action-leading-icon') ← if leading_icon
 *     │   └ text(value or placeholder, role='input-with-action-text')
 *     └ frame(action button, fit_content or 44×44 square, role='input-with-action-button')
 *         └ text(label) OR icon_font(action_icon)
 */
export function buildInputWithAction(params: InputWithActionParams): ElementTree {
  const width = Math.max(280, Math.floor(params.width ?? 400));
  const kind: InputWithActionKind = params.action_kind ?? 'text';
  const isFilled = params.value !== undefined && params.value !== '';
  const inputContent = isFilled ? params.value! : params.placeholder;
  const inputColor = isFilled ? '#0F172A' : '#94A3B8';

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
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }
  inputChildren.push({
    type: 'text',
    name: 'Text',
    role: 'input-with-action-text',
    content: inputContent,
    fontSize: 14,
    fontWeight: 400,
    fill: [{ type: 'solid', color: inputColor }],
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
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#CBD5E1' }] },
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
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          type: 'icon_font',
          name: 'Action Icon',
          role: 'input-with-action-icon',
          iconFontName: icon,
          iconFontFamily: 'lucide',
          width: 18,
          height: 18,
          fill: [{ type: 'solid', color: '#FFFFFF' }],
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
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          type: 'text',
          name: 'Action Label',
          role: 'input-with-action-label',
          content: label,
          fontSize: 14,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#FFFFFF' }],
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
