import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { expandOverflowingFixedHeightCards } from '../layout/expand-overflowing-fixed-height-cards';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    ...props,
  }) as PenNode;

const text = (props: Partial<PenNode> & { content?: string }): PenNode =>
  ({
    id: 't',
    type: 'text',
    ...props,
  }) as PenNode;

describe('expandOverflowingFixedHeightCards', () => {
  it('switches a card with fixed height to fit_content when content overflows', () => {
    // Banner-style card: model emits height: 165 for an image-on-
    // right layout, but the content side stacks badge + title +
    // body + button which naturally takes ~220px. With clipContent
    // (the card role default), the button gets cut off.
    const card = frame({
      id: 'banner',
      role: 'card',
      width: 343,
      height: 165,
      layout: 'horizontal',
      padding: 14,
      gap: 16,
      clipContent: true,
      children: [
        frame({
          id: 'content',
          width: 'fill_container',
          height: 'fill_container',
          layout: 'vertical',
          gap: 8,
          children: [
            frame({
              id: 'badge',
              width: 'fit_content',
              height: 'fit_content',
              layout: 'horizontal',
              padding: [6, 10],
              children: [text({ content: '30% OFF', fontSize: 12, lineHeight: 1.4 })],
            }),
            text({ id: 'title', content: 'Hot pizza deal', fontSize: 22, lineHeight: 1.2 }),
            text({
              id: 'body',
              content: 'Free delivery on cheesy favorites tonight.',
              fontSize: 14,
              lineHeight: 1.5,
              width: 'fill_container',
            }),
            frame({
              id: 'cta',
              width: 'fit_content',
              height: 'fit_content',
              layout: 'horizontal',
              padding: [10, 16],
              children: [text({ content: 'Order now', fontSize: 14, lineHeight: 1.2 })],
            }),
          ],
        }),
        frame({
          id: 'image-wrap',
          width: 128,
          height: 'fill_container',
          children: [],
        }),
      ],
    });
    const root = frame({
      id: 'root',
      width: 375,
      children: [card],
    });
    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(true);
    expect((card as PenNode & { height?: unknown }).height).toBe('fit_content');
  });

  it('leaves cards alone when content fits', () => {
    const card = frame({
      id: 'sized-right',
      role: 'card',
      width: 200,
      height: 200,
      layout: 'vertical',
      padding: 16,
      children: [text({ content: 'Just a label', fontSize: 14, lineHeight: 1.4 })],
    });
    const root = frame({ id: 'root', width: 400, children: [card] });
    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(false);
    expect((card as PenNode & { height?: unknown }).height).toBe(200);
  });

  it('does not touch frames without a card role', () => {
    // A non-card frame with overflowing content keeps its declared
    // height — the rule is scoped to card-family roles to avoid
    // collateral damage on intentional fixed-size containers.
    const tile = frame({
      id: 'tile',
      role: 'icon-button',
      width: 44,
      height: 44,
      children: [
        text({
          content: 'Way too long to fit in 44',
          fontSize: 16,
          lineHeight: 1.5,
          width: 'fill_container',
        }),
      ],
    });
    const root = frame({ id: 'root', width: 400, children: [tile] });
    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(false);
    expect((tile as PenNode & { height?: unknown }).height).toBe(44);
  });

  it('handles cards with non-numeric height (already auto-sizing)', () => {
    // A card already using fit_content / fill_container is by
    // definition not at risk of clipping its own content; the pass
    // should be a no-op.
    const card = frame({
      id: 'flex-card',
      role: 'card',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      padding: 16,
      children: [text({ content: 'Anything goes', fontSize: 16, lineHeight: 1.4 })],
    });
    const root = frame({ id: 'root', width: 400, children: [card] });
    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(false);
    expect((card as PenNode & { height?: unknown }).height).toBe('fit_content');
  });

  it('walks nested cards (overflow on a card inside a section)', () => {
    const innerCard = frame({
      id: 'nested-card',
      role: 'card',
      width: 280,
      height: 80,
      layout: 'vertical',
      padding: 16,
      children: [
        text({
          content: 'Headline that wraps over multiple lines makes content tall',
          fontSize: 18,
          lineHeight: 1.3,
          width: 'fill_container',
        }),
        text({
          content: 'And there is body text below it as well that adds more height',
          fontSize: 14,
          lineHeight: 1.5,
          width: 'fill_container',
        }),
      ],
    });
    const section = frame({
      id: 'section',
      role: 'section',
      width: 'fill_container',
      height: 'fit_content',
      children: [innerCard],
    });
    const root = frame({ id: 'root', width: 375, children: [section] });
    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(true);
    expect((innerCard as PenNode & { height?: unknown }).height).toBe('fit_content');
  });
});
