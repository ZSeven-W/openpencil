import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type DrawerShellV1Side = 'right' | 'left';

export interface DrawerShellV1Params {
  title: string;
  /** Side the drawer slides from. Default 'right'. */
  side?: DrawerShellV1Side;
  /** Drawer width. Default 400. Clamped 280..640. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_drawer_shell_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Slide-in drawer shell — theme-aware version of buildDrawerShell.
 * Light mode is byte-equal to add_drawer_shell_v0.
 *
 * Color mapping:
 *   drawer bg (#FFFFFF)          → surface
 *   header border (#E2E8F0)      → border
 *   title (#0F172A)              → textPrimary
 *   close icon (#475569)         → textMuted
 *   shadow (#0F172A1F)           → kept as-is (shadow is theme-agnostic)
 */
export function buildDrawerShellV1(params: DrawerShellV1Params): ElementTree {
  const side: DrawerShellV1Side = params.side ?? 'right';
  const width = Math.min(640, Math.max(280, Math.floor(params.width ?? 400)));
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const surfaceColor = isLight ? '#FFFFFF' : t.colors.surface;
  const borderColor = isLight ? '#E2E8F0' : t.colors.border;
  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const closeIconColor = isLight ? '#475569' : t.colors.textMuted;

  return {
    type: 'frame',
    name: 'Drawer Shell',
    role: side === 'left' ? 'drawer-shell-left' : 'drawer-shell-right',
    width,
    height: 'fill_container',
    layout: 'vertical',
    fill: [{ type: 'solid', color: surfaceColor }],
    effects: [
      {
        type: 'shadow',
        offsetX: side === 'left' ? 8 : -8,
        offsetY: 0,
        blur: 24,
        spread: 0,
        color: '#0F172A1F',
      },
    ],
    children: [
      {
        type: 'frame',
        name: 'Header',
        role: 'drawer-shell-header',
        width: 'fill_container',
        height: 56,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'space_between',
        padding: [0, 20],
        stroke: { thickness: [0, 0, 1, 0], fill: [{ type: 'solid', color: borderColor }] },
        children: [
          {
            type: 'text',
            name: 'Title',
            role: 'drawer-shell-title',
            content: params.title,
            fontSize: 16,
            fontWeight: 600,
            fill: [{ type: 'solid', color: titleColor }],
          },
          {
            type: 'frame',
            name: 'Close Button',
            role: 'drawer-shell-close',
            width: 32,
            height: 32,
            cornerRadius: 8,
            layout: 'horizontal',
            alignItems: 'center',
            justifyContent: 'center',
            children: [
              {
                type: 'icon_font',
                name: 'Icon',
                iconFontName: 'x',
                iconFontFamily: 'lucide',
                width: 18,
                height: 18,
                fill: [{ type: 'solid', color: closeIconColor }],
              },
            ],
          },
        ],
      },
    ],
  };
}
