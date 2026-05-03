import type { ElementTree } from './helpers.js';

export interface FormFieldParams {
  label: string;
  placeholder?: string;
  leading_icon?: string;
  trailing_icon?: string;
  required?: boolean;
}

/**
 * Form field: label above input box. ALL inputs AND primary button
 * use width=fill_container (DESIGN_GUIDELINES). input height=48,
 * horizontal padding=[12,16]. required=true appends "*" to label.
 */
export function buildFormField(params: FormFieldParams): ElementTree {
  const labelText = params.required ? `${params.label} *` : params.label;
  const inputChildren: ElementTree[] = [];
  if (params.leading_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  inputChildren.push({
    type: 'text',
    name: 'Placeholder',
    content: params.placeholder ?? '',
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.trailing_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  return {
    type: 'frame',
    name: 'Form Field',
    role: 'form-field',
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
        role: 'form-input',
        width: 'fill_container',
        height: 48,
        cornerRadius: 8,
        layout: 'horizontal',
        alignItems: 'center',
        gap: 8,
        padding: [12, 16],
        children: inputChildren,
      },
    ],
  };
}
