import type { ElementTree } from './helpers.js';

export type CalloutTone = 'info' | 'success' | 'warning' | 'danger' | 'note';

export interface CalloutParams {
  /** Body text. Required. */
  body: string;
  /** Optional bold heading line above the body. */
  title?: string;
  /** Color tone. Default 'note' (slate). */
  tone?: CalloutTone;
}

interface ToneSpec {
  bg: string;
  fg: string;
  icon: string;
}

const TONES: Record<CalloutTone, ToneSpec> = {
  info: { bg: '#DBEAFE', fg: '#1E40AF', icon: 'info' },
  success: { bg: '#DCFCE7', fg: '#166534', icon: 'check-circle' },
  warning: { bg: '#FEF3C7', fg: '#92400E', icon: 'alert-triangle' },
  danger: { bg: '#FEE2E2', fg: '#991B1B', icon: 'alert-octagon' },
  note: { bg: '#F1F5F9', fg: '#0F172A', icon: 'sticky-note' },
};

/**
 * Inline doc callout — standalone tinted block carrying a heading and
 * paragraph of guidance. Distinct from `add_alert_v0` (close-able
 * banner inside chrome — has dismiss × icon, single line of text)
 * and `add_tooltip_v0` (small floating dark pill). Use for docs,
 * onboarding notes, "did you know" panels.
 */
export function buildCallout(params: CalloutParams): ElementTree {
  const tone = TONES[params.tone ?? 'note'];
  const stackChildren: ElementTree[] = [];
  if (params.title) {
    stackChildren.push({
      type: 'text',
      name: 'Title',
      role: 'callout-title',
      content: params.title,
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: tone.fg }],
    });
  }
  stackChildren.push({
    type: 'text',
    name: 'Body',
    role: 'callout-body',
    content: params.body,
    fontSize: 13,
    fontWeight: 400,
    lineHeight: 1.5,
    fill: [{ type: 'solid', color: tone.fg }],
    width: 'fill_container',
    textGrowth: 'fixed-width',
  });
  return {
    type: 'frame',
    name: 'Callout',
    role: 'callout',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    gap: 12,
    padding: [12, 16],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: tone.bg }],
    children: [
      {
        type: 'icon_font',
        name: 'Tone Icon',
        role: 'callout-icon',
        iconFontName: tone.icon,
        iconFontFamily: 'lucide',
        width: 18,
        height: 18,
        fill: [{ type: 'solid', color: tone.fg }],
      },
      {
        type: 'frame',
        name: 'Text Stack',
        role: 'callout-text',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 4,
        children: stackChildren,
      },
    ],
  };
}
