import type { ElementTree } from './helpers.js';

export type DrawerSide = 'right' | 'left';

export interface DrawerShellParams {
  title: string;
  /** Side the drawer slides from. Default 'right' (more common for
   *  desktop edit / detail panels). */
  side?: DrawerSide;
  /** Drawer width. Default 400. Clamped 280..640. */
  width?: number;
}

/**
 * Slide-in drawer shell — full-height side panel for edit / detail
 * surfaces. Distinct from `add_modal_shell_v0` (centered card with
 * scrim) and `add_action_menu_v0` (compact dropdown). Tool emits
 * just the drawer card with a header row (title + close ×); body
 * content is composed via subsequent tool calls under the drawer's
 * id.
 *
 * Side is encoded in the role hint (`drawer-shell-right` /
 * `drawer-shell-left`) so post-pass / generators can apply the right
 * slide-in animation and shadow direction without re-parsing.
 */
export function buildDrawerShell(params: DrawerShellParams): ElementTree {
  const side: DrawerSide = params.side ?? 'right';
  const width = Math.min(640, Math.max(280, Math.floor(params.width ?? 400)));
  return {
    type: 'frame',
    name: 'Drawer Shell',
    role: side === 'left' ? 'drawer-shell-left' : 'drawer-shell-right',
    width,
    height: 'fill_container',
    layout: 'vertical',
    fill: [{ type: 'solid', color: '#FFFFFF' }],
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
        stroke: { thickness: [0, 0, 1, 0], fill: [{ type: 'solid', color: '#E2E8F0' }] },
        children: [
          {
            type: 'text',
            name: 'Title',
            role: 'drawer-shell-title',
            content: params.title,
            fontSize: 16,
            fontWeight: 600,
            fill: [{ type: 'solid', color: '#0F172A' }],
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
                fill: [{ type: 'solid', color: '#475569' }],
              },
            ],
          },
        ],
      },
    ],
  };
}
