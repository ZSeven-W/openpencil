import type { ElementTree } from './helpers.js';

export type ToastV1Theme = 'light' | 'dark' | 'system';

export interface ToastV1Params {
  /** Toast message. Required. */
  message: string;
  /** Optional leading lucide icon (e.g. "check", "info", "alert-triangle"). */
  icon?: string;
  /**
   * Theme variant. Default `'light'` — byte-parity with `add_toast_v0`
   * so existing callers upgrade by changing tool name only.
   *
   * Design intent: toasts use INVERTED contrast — a dark pill sits on
   * light backgrounds, a light pill sits on dark backgrounds. The
   * theme swap is a color inversion, not a surface tint.
   *
   * - `'light'`: dark pill (#111827) + white fg — same as v0
   * - `'dark'`: light pill (#F1F5F9) + dark fg (#0F172A) — the
   *   inverted variant for dark-theme surfaces
   * - `'system'`: emits `$color-text-primary` bg + `$color-surface` fg
   *   refs; render tracks `themes.Mode`. Caller MUST have seeded the
   *   document with `applySemanticPalette()` first.
   */
  theme?: ToastV1Theme;
}

interface ResolvedColors {
  pillFill: string;
  fgColor: string;
}

function resolveTheme(theme: ToastV1Theme): ResolvedColors {
  if (theme === 'system') {
    return {
      // Inverted swap: text-primary becomes the bg, surface becomes the fg.
      // Renders the "inverted pill" pattern automatically across themes.
      pillFill: '$color-text-primary',
      fgColor: '$color-surface',
    };
  }
  if (theme === 'dark') {
    return {
      pillFill: '#F1F5F9', // light pill for dark bg (inverted contrast)
      fgColor: '#0F172A',
    };
  }
  // Default: light — byte-parity with v0
  return {
    pillFill: '#111827',
    fgColor: '#FFFFFF',
  };
}

/**
 * Theme-aware floating toast pill (v1). Same pill shape as
 * `add_toast_v0` with an added `theme` param that inverts contrast
 * for dark-theme surfaces.
 *
 * v0 parity: `buildToastV1({ message, icon, theme: 'light' })`
 * (or theme omitted) produces a tree byte-equivalent to v0 output,
 * modulo ids.
 *
 * See `openpencil-docs/superpowers/notes/2026-04-22-dark-theme-defaults-audit.md`
 * for the v0 → v1 migration rationale.
 */
export function buildToastV1(params: ToastV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const colors = resolveTheme(theme);

  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: colors.fgColor }],
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'toast-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 500,
    fill: [{ type: 'solid', color: colors.fgColor }],
  });

  return {
    type: 'frame',
    name: 'Toast',
    role: 'toast',
    width: 'fit_content',
    cornerRadius: 24,
    padding: [12, 20],
    fill: [{ type: 'solid', color: colors.pillFill }],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children,
  };
}
