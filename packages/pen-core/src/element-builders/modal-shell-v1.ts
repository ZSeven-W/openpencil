import { resolveTheme, type V1Theme } from './resolve-theme.js';
import type { ElementTree } from './helpers.js';

export type ModalShellV1Theme = V1Theme;

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

/**
 * In light mode: titleFill=null to match v0 behavior (title had no fill).
 * In dark/system: titleFill is a concrete color or ref.
 */
function getTitleFill(theme: ModalShellV1Theme): string | null {
  if (theme === 'light') return null; // v0 parity: title unstyled
  const t = resolveTheme(theme);
  return t.colors.textPrimary;
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
  const scrimOpacity = Math.max(0, Math.min(1, params.scrim_opacity ?? 0.5));
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const titleFill = getTitleFill(theme);

  // Padding: user-supplied override takes priority (as a concrete number);
  // when absent and in 'system' mode, emit the $spacing-5 ref (24px).
  // In light/dark mode always use the concrete number (v0 byte-parity for light).
  const rawPadding = params.card_padding;
  const cardPadding: number | string =
    rawPadding !== undefined
      ? Math.max(12, Math.floor(rawPadding))
      : theme === 'system'
        ? (t.spacing.s5 as string)
        : 24;

  const titleNode: ElementTree = {
    type: 'text',
    name: 'Title',
    role: 'modal-title',
    content: params.title,
    fontSize: t.typography.h2Size,
    fontWeight: t.typography.h2Weight,
  };
  if (titleFill !== null) {
    titleNode.fill = [{ type: 'solid', color: titleFill }];
  }

  const cardChildren: ElementTree[] = [titleNode];
  if (params.subtitle) {
    cardChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'modal-subtitle',
      content: params.subtitle,
      fontSize: t.typography.bodySize,
      fontWeight: t.typography.bodyWeight,
      lineHeight: t.typography.bodyLineHeight,
      fill: [{ type: 'solid', color: t.colors.textMuted }],
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
        // cornerRadius=16 is a builder-private value not in the token system
        // (closest token $radius-lg=12 has different semantics). Kept hardcoded
        // across all themes per spec §3.4 / v0 byte-parity contract.
        cornerRadius: 16,
        padding: cardPadding,
        layout: 'vertical',
        gap: t.spacing.s3,
        fill: [{ type: 'solid', color: t.colors.surface }],
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
