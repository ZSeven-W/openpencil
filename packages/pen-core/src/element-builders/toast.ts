import type { ElementTree } from './helpers.js';

export interface ToastParams {
  message: string;
  icon?: string;
}

/**
 * Floating pill notification. Dark fill, cornerRadius=24 (pill),
 * content-sized (fit_content) so it doesn't stretch.
 */
export function buildToast(params: ToastParams): ElementTree {
  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'toast-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 500,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
  });
  return {
    type: 'frame',
    name: 'Toast',
    role: 'toast',
    width: 'fit_content',
    cornerRadius: 24,
    padding: [12, 20],
    fill: [{ type: 'solid', color: '#111827' }],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children,
  };
}
