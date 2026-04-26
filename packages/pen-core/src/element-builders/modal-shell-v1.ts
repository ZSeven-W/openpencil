import type { ElementTree } from './helpers.js';

export type ModalShellV1Theme = 'light' | 'dark' | 'system';

export interface ModalShellV1Params {
  /** Modal dialog title (rendered as heading inside the card). */
  title: string;
  /** Optional subtitle / description below the title. */
  subtitle?: string;
  /** Width of the centered card in px. Default 400. Min 280. */
  card_width?: number;
  /** Padding inside the card (all 4 sides equal). Default 24. */
  card_padding?: number;
  /**
   * Backdrop scrim opacity 0..1. Default 0.5 (standard dimmed
   * overlay). Set to 0 for a borderless modal without scrim.
   *
   * Note: scrim is black in both light AND dark themes (a modal
   * backdrop is a "dim everything below" effect regardless of
   * theme), so scrim_opacity tunes only the darkness level, not
   * the color.
   */
  scrim_opacity?: number;
  /**
   * Theme variant. Default `'light'` — byte-parity with v0 so
   * existing callers can upgrade by changing the tool name
   * without visual regression.
   *
   * - `'light'`: hardcoded light-theme hex (same as v0)
   * - `'dark'`: hardcoded dark-theme hex (slate-800 card + slate-
   *   200 text)
   * - `'system'`: emit `$color-surface` + `$color-text-muted`
   *   refs; renders follow `themes.Mode` at paint time. Caller
   *   MUST have seeded the document with `applySemanticPalette()`
   *   first — otherwise refs resolve to undefined and nothing
   *   paints.
   */
  theme?: ModalShellV1Theme;
}

interface ResolvedColors {
  cardFill: string;
  titleFill: string | null;
  subtitleFill: string;
}

/**
 * Resolve theme to the concrete color strings the builder emits.
 * Returns either hex literals (light/dark) or `$color-*` refs
 * (system). `titleFill: null` means "don't emit a fill" —
 * match v0 behavior where title was unstyled (picks up the
 * default text color).
 */
function resolveTheme(theme: ModalShellV1Theme): ResolvedColors {
  if (theme === 'system') {
    return {
      cardFill: '$color-surface',
      titleFill: '$color-text-primary',
      subtitleFill: '$color-text-muted',
    };
  }
  if (theme === 'dark') {
    return {
      cardFill: '#1E293B', // color-surface Dark
      titleFill: '#F1F5F9', // color-text-primary Dark
      subtitleFill: '#94A3B8', // color-text-muted Dark
    };
  }
  // Default: light — byte-parity with v0
  return {
    cardFill: '#FFFFFF',
    titleFill: null, // v0 emitted no fill on the title text
    subtitleFill: '#64748B',
  };
}

/**
 * Modal dialog shell (v1) — same structure as v0 with an added
 * `theme` param for theme-aware rendering. See
 * `openpencil-docs/superpowers/notes/2026-04-22-dark-theme-defaults-audit.md`
 * for the v0 → v1 migration rationale.
 *
 * v0 parity: calling `buildModalShellV1({ title, theme: 'light' })`
 * (or omitting `theme`) produces a tree byte-equivalent to v0's
 * output for the same title + subtitle + card_width + card_padding
 * + scrim_opacity, modulo ids.
 *
 * See `buildModalShell` for structure details — unchanged here.
 */
export function buildModalShellV1(params: ModalShellV1Params): ElementTree {
  const cardWidth = Math.max(280, Math.floor(params.card_width ?? 400));
  const cardPadding = Math.max(12, Math.floor(params.card_padding ?? 24));
  const scrimOpacity = Math.max(0, Math.min(1, params.scrim_opacity ?? 0.5));
  const theme = params.theme ?? 'light';
  const colors = resolveTheme(theme);

  const titleNode: ElementTree = {
    type: 'text',
    name: 'Title',
    role: 'modal-title',
    content: params.title,
    fontSize: 20,
    fontWeight: 600,
  };
  if (colors.titleFill !== null) {
    titleNode.fill = [{ type: 'solid', color: colors.titleFill }];
  }

  const cardChildren: ElementTree[] = [titleNode];
  if (params.subtitle) {
    cardChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'modal-subtitle',
      content: params.subtitle,
      fontSize: 14,
      fontWeight: 400,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: colors.subtitleFill }],
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
        fill: [{ type: 'solid', color: colors.cardFill }],
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
