import { buildScrollWrapper, type ElementTree } from './helpers.js';

export interface CardRowItem {
  title: string;
  subtitle?: string;
  icon?: string;
}

export interface CardRowParams {
  items: CardRowItem[];
  card_width?: number;
  gap?: number;
}

/**
 * Horizontal scroll row of CARDS (title + subtitle + optional icon).
 * Each card: `card_width`×160 frame, cornerRadius=20, padding=16,
 * vertical layout, gap=8.
 *
 * Mirrors pen-mcp `add_card_row_v0` tree build exactly — both sides
 * call this builder so drift is impossible.
 */
export function buildCardRow(params: CardRowParams): ElementTree {
  const cardWidth = params.card_width ?? 140;
  const gap = params.gap ?? 12;
  const cards = params.items.map((item) => buildCard(item, cardWidth));
  return buildScrollWrapper({ rowName: 'Card Row', innerChildren: cards, gap });
}

function buildCard(item: CardRowItem, cardWidth: number): ElementTree {
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
  children.push({
    type: 'text',
    name: 'Title',
    role: 'heading',
    content: item.title,
    fontSize: 16,
    fontWeight: 600,
    width: 'fill_container',
  });
  if (item.subtitle) {
    children.push({
      type: 'text',
      name: 'Subtitle',
      role: 'body',
      content: item.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
    });
  }
  return {
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
}
