import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface TextareaV1Params {
  label: string;
  placeholder?: string;
  /** Visible text rows. Default 4. Clamped to [2, 12]. */
  rows?: number;
  required?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_textarea_v0.
   * - `'dark'`: identical — no hardcoded surface colors in v0.
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Multi-line text input (v1) — theme-aware variant of buildTextarea.
 * Light mode is byte-equal to add_textarea_v0.
 *
 * No hardcoded surface colors in v0 (label/placeholder inherit canvas default,
 * no border fill). All theme modes produce identical trees.
 */
export function buildTextareaV1(params: TextareaV1Params): ElementTree {
  const labelText = params.required ? `${params.label} *` : params.label;
  const rows = Math.max(2, Math.min(12, params.rows ?? 4));
  const inputHeight = rows * 24 + 24;
  return {
    type: 'frame',
    name: 'Textarea',
    role: 'textarea',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: labelText,
        fontSize: 14,
        fontWeight: 500,
      },
      {
        type: 'frame',
        name: 'Input',
        role: 'textarea-input',
        width: 'fill_container',
        height: inputHeight,
        cornerRadius: 8,
        layout: 'vertical',
        alignItems: 'start',
        padding: [12, 16],
        children: [
          {
            type: 'text',
            name: 'Placeholder',
            content: params.placeholder ?? '',
            fontSize: 14,
            fontWeight: 400,
            lineHeight: 1.5,
          },
        ],
      },
    ],
  };
}
