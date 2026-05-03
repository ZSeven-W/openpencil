import type { ElementTree } from './helpers.js';

export interface TextareaParams {
  label: string;
  placeholder?: string;
  /** Visible text rows (heightIn = rows × 24 line-height + vertical padding). Default 4. Clamped to [2, 12]. */
  rows?: number;
  required?: boolean;
}

/**
 * Multi-line text input. Same label-above-input pattern as form-field
 * but the input grows vertically to accommodate longer content (notes,
 * bio, feedback). Size model: rows × 24 + 24 padding — matches native
 * iOS / Material multi-line input behavior where rows controls initial
 * visible height, not hard cap.
 *
 * Differences vs. form-field:
 *   - height computed from `rows` instead of a fixed 48
 *   - input frame layout is vertical (placeholder text aligns to top-left)
 *   - no leading/trailing icon slots (multi-line inputs don't have them)
 */
export function buildTextarea(params: TextareaParams): ElementTree {
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
