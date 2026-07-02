import { buildScrollWrapper, type ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CardRowV1Item {
  title: string;
  subtitle?: string;
  icon?: string;
}

export interface CardRowV1Params {
  items: CardRowV1Item[];
  card_width?: number;
  gap?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_card_row_v0.
   * - `'dark'`: dark surface + dark-mode text fills.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Horizontal scroll row of CARDS (title + subtitle + optional icon).
 * Theme-aware version of buildCardRow.
 * Light mode is byte-equal to add_card_row_v0 (no fill attrs on text/card).
 * Dark/system modes add card surface fill and text fills.
 */
export function buildCardRowV1(params: CardRowV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const cardWidth = params.card_width ?? 140;
  const gap = params.gap ?? 12;
  const cards = params.items.map((item) => buildCard(item, cardWidth, theme));
  return buildScrollWrapper({ rowName: 'Card Row', innerChildren: cards, gap });
}

function buildCard(item: CardRowV1Item, cardWidth: number, theme: V1Theme): ElementTree {
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }

  const titleNode: ElementTree = {
    type: 'text',
    name: 'Title',
    role: 'heading',
    content: item.title,
    fontSize: 16,
    fontWeight: 600,
    width: 'fill_container',
  };
  if (!isLight) {
    titleNode.fill = [{ type: 'solid', color: t.colors.textPrimary }];
  }
  children.push(titleNode);

  if (item.subtitle) {
    const subtitleNode: ElementTree = {
      type: 'text',
      name: 'Subtitle',
      role: 'body',
      content: item.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
    };
    if (!isLight) {
      subtitleNode.fill = [{ type: 'solid', color: t.colors.textMuted }];
    }
    children.push(subtitleNode);
  }

  const cardNode: ElementTree = {
    type: 'frame',
    name: 'Card',
    role: 'card',
    width: cardWidth,
    height: 160,
    cornerRadius: 20,
    padding: 16,
    layout: 'vertical',
    gap: 8,
    children,
  };
  if (!isLight) {
    cardNode.fill = [{ type: 'solid', color: t.colors.surface }];
  }
  return cardNode;
}
