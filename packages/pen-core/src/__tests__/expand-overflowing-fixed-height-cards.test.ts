import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { expandOverflowingFixedHeightCards } from '../layout/expand-overflowing-fixed-height-cards';
import { fitContentHeight } from '../layout/engine';

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

  it('does NOT touch image-card frames (fixed crop / aspect ratio is intentional)', () => {
    // image-card exists specifically to lock in a fixed crop or
    // aspect ratio (a 16:9 photo tile, a 1:1 thumbnail). Auto-
    // expanding it would silently break the intended visual
    // proportion. Authors that want an image card to auto-grow with
    // content should use `role: 'card'` instead.
    //
    // Test must trigger the bug condition (natural > declared) so
    // it would FAIL if image-card were back in CARD_ROLES. We use
    // a small declared height (80px crop) and a multi-paragraph
    // caption wrapped to a narrow width — the caption alone forces
    // natural height past 200px, well above the 80 we declared.
    // Identical structure under `role: 'card'` exercises the
    // expand path; under `role: 'image-card'` the pass must skip.
    const longCaption =
      'This caption deliberately spans many wrapped lines so the natural ' +
      'content height pushes well past the declared crop. Without the role-based ' +
      'skip in CARD_ROLES, the expand pass would convert the image-card to ' +
      'fit_content and break the intended visual proportion.';
    const imgCard = frame({
      id: 'img-card',
      role: 'image-card',
      width: 300,
      height: 80, // 1:3.75 crop — wildly smaller than caption text
      layout: 'vertical',
      padding: 0,
      gap: 8,
      children: [
        frame({
          id: 'photo',
          type: 'image',
          width: 'fill_container',
          height: 'fill_container',
        } as Partial<PenNode>),
        text({
          id: 'caption',
          content: longCaption,
          fontSize: 14,
          lineHeight: 1.5,
          width: 'fill_container',
        }),
      ],
    });
    const root = frame({ id: 'root', width: 375, children: [imgCard] });

    // Sanity: confirm the test setup actually triggers the bug
    // condition. Compute natural height the same way the pass does;
    // it must exceed `declared` for this to be a real regression
    // test rather than a vacuous pass.
    const natural = fitContentHeight(imgCard);
    expect(natural).toBeGreaterThan(80);

    const changed = expandOverflowingFixedHeightCards(root);
    expect(changed).toBe(false);
    expect((imgCard as PenNode & { height?: unknown }).height).toBe(80);

    // And mirror: a `role: 'card'` clone of the same shape DOES get
    // expanded. Asserting the contrast here makes the role-based
    // gate the clear difference between pass and fail.
    const cardClone = frame({
      id: 'card-clone',
      role: 'card',
      width: 300,
      height: 80,
      layout: 'vertical',
      padding: 0,
      gap: 8,
      children: [
        frame({
          id: 'photo2',
          type: 'image',
          width: 'fill_container',
          height: 'fill_container',
        } as Partial<PenNode>),
        text({
          id: 'caption2',
          content: longCaption,
          fontSize: 14,
          lineHeight: 1.5,
          width: 'fill_container',
        }),
      ],
    });
    const cloneRoot = frame({ id: 'r2', width: 375, children: [cardClone] });
    const cloneChanged = expandOverflowingFixedHeightCards(cloneRoot);
    expect(cloneChanged).toBe(true);
    expect((cardClone as PenNode & { height?: unknown }).height).toBe('fit_content');
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
