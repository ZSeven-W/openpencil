import type { ElementTree } from './helpers.js';

export interface AlertParams {
  message: string;
  icon?: string;
  dismissible?: boolean;
}

/**
 * Inline alert / callout banner. fill_container row of [icon?] +
 * message + [close-x?]. Neutral colors; semantic color applied via
 * batch_design U-op.
 */
export function buildAlert(params: AlertParams): ElementTree {
  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'alert-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.dismissible) {
    children.push({
      type: 'icon_font',
      name: 'Close',
      role: 'alert-close',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'Alert',
    role: 'alert',
    width: 'fill_container',
    cornerRadius: 8,
    padding: [12, 16],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    children,
  };
}
