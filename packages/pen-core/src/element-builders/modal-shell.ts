import type { ElementTree } from './helpers.js';

export interface ModalShellParams {
  /** Modal dialog title (rendered as heading inside the card). */
  title: string;
  /** Optional subtitle / description below the title. */
  subtitle?: string;
  /** Width of the centered card in px. Default 400. Min 280. */
  card_width?: number;
  /**
   * Padding inside the card (all 4 sides equal). Default 24.
   */
  card_padding?: number;
  /**
   * Backdrop scrim opacity 0..1. Default 0.5 (standard dimmed
   * overlay). Set to 0 for a borderless modal without scrim.
   */
  scrim_opacity?: number;
}

/**
 * Modal dialog shell: full-bleed dimmed backdrop + centered card
 * with title + optional subtitle. Body content (form fields, CTA
 * button, etc.) is composed by the caller via batch_design into
 * the `modal-shell-card` role.
 *
 * Structure:
 *   frame(fill_container × fill_container, role='modal-scrim', fill=black@opacity,
 *         layout=horizontal, alignItems=center, justifyContent=center)
 *     └ frame(card_width × fit_content, role='modal-shell-card',
 *             cornerRadius=16, padding, fill=white, layout=vertical, gap=12,
 *             shadow effect)
 *         ├ text(title, 20/600)
 *         └ optional text(subtitle, 14/400 slate-500)
 *
 * The `modal-shell-card` role is the intended insertion target for
 * body content. Caller does a follow-up `add_*_v0` or batch_design
 * with `parent_id` pointing to it.
 *
 * "modal-shell" naming (not "modal" / "dialog") is deliberate: this
 * emits ONLY the chrome (scrim + card + header). The body slot is
 * up to the caller — the tool does not pretend to solve the full
 * "modal with form" case in one shot.
 */
export function buildModalShell(params: ModalShellParams): ElementTree {
  const cardWidth = Math.max(280, Math.floor(params.card_width ?? 400));
  const cardPadding = Math.max(12, Math.floor(params.card_padding ?? 24));
  const scrimOpacity = Math.max(0, Math.min(1, params.scrim_opacity ?? 0.5));

  const cardChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'modal-title',
      content: params.title,
      fontSize: 20,
      fontWeight: 600,
    },
  ];
  if (params.subtitle) {
    cardChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'modal-subtitle',
      content: params.subtitle,
      fontSize: 14,
      fontWeight: 400,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }

  const scrimFill: Array<{ type: 'solid'; color: string; opacity?: number }> =
    scrimOpacity > 0 ? [{ type: 'solid', color: '#000000', opacity: scrimOpacity }] : [];

  return {
    type: 'frame',
    name: 'Modal Shell',
    role: 'modal-scrim',
    width: 'fill_container',
    height: 'fill_container',
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    fill: scrimFill,
    children: [
      {
        type: 'frame',
        name: 'Modal Card',
        role: 'modal-shell-card',
        width: cardWidth,
        height: 'fit_content',
        cornerRadius: 16,
        padding: cardPadding,
        layout: 'vertical',
        gap: 12,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        effects: [
          {
            type: 'shadow',
            offsetX: 0,
            offsetY: 16,
            blur: 40,
            spread: 0,
            color: '#00000026',
          },
        ],
        children: cardChildren,
      },
    ],
  };
}
