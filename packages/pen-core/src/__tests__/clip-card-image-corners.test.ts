import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { clipCardImageCorners } from '../layout/clip-card-image-corners';

const frame = (
  props: Partial<PenNode> & { children?: PenNode[]; cornerRadius?: unknown; clipContent?: boolean },
): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    ...props,
  }) as PenNode;

describe('clipCardImageCorners', () => {
  // 2026-05-10 user report — the Taco Fiesta card showed an image with
  // all 4 corners rounded inside the card; the bottom corners then sat
  // ragged against the title text below. The pass turns the parent into
  // a clipping container and removes the image's own scalar cornerRadius
  // so the card's outer cornerRadius cleanly clips the image.

  it('sets parent clipContent + removes scalar cornerRadius from first-child image', () => {
    const image = frame({
      id: 'img',
      type: 'image',
      cornerRadius: 12,
    } as never);
    const title = frame({ id: 't', type: 'text', content: 'Taco Fiesta' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 12,
      children: [image, title],
    });

    const changed = clipCardImageCorners(card);

    expect(changed).toBe(true);
    expect((card as PenNode & { clipContent?: boolean }).clipContent).toBe(true);
    expect((image as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
  });

  it('also fires when the first child is a canonical image-placeholder frame', () => {
    const placeholder = frame({
      id: 'ph',
      role: 'image-placeholder',
      cornerRadius: 16,
    });
    const title = frame({ id: 't', type: 'text', content: 'Bella Italia' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 16,
      children: [placeholder, title],
    });

    clipCardImageCorners(card);

    expect((card as PenNode & { clipContent?: boolean }).clipContent).toBe(true);
    expect((placeholder as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
  });

  it('does NOT touch a card whose only child is the image (standalone image, not a card)', () => {
    const image = frame({ id: 'img', type: 'image', cornerRadius: 12 } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 12,
      children: [image], // single child = standalone, leave alone
    });

    const changed = clipCardImageCorners(card);

    expect(changed).toBe(false);
    expect((image as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(12);
  });

  it("does NOT touch a card whose first child isn't an image (e.g. title-first card)", () => {
    const title = frame({ id: 't', type: 'text', content: 'Card' } as never);
    const image = frame({ id: 'img', type: 'image', cornerRadius: 12 } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 12,
      children: [title, image],
    });

    const changed = clipCardImageCorners(card);

    // First child is text — pass is silent so the user's title-on-top
    // layout stays intact and the image keeps its own corners.
    expect(changed).toBe(false);
    expect((image as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(12);
  });

  it('does NOT touch a card with array-form cornerRadius (already explicit per-corner)', () => {
    const image = frame({ id: 'img', type: 'image', cornerRadius: 12 } as never);
    const title = frame({ id: 't', type: 'text', content: 'X' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: [12, 12, 0, 0] as never, // already says "round only top"
      children: [image, title],
    });

    const changed = clipCardImageCorners(card);

    // Array-form parent cornerRadius — the helper only fires for scalar
    // because array-form already encodes the explicit choice.
    expect(changed).toBe(false);
  });

  it('does NOT touch a card with cornerRadius 0 (no rounding to clip against)', () => {
    const image = frame({ id: 'img', type: 'image', cornerRadius: 12 } as never);
    const title = frame({ id: 't', type: 'text', content: 'X' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 0,
      children: [image, title],
    });

    const changed = clipCardImageCorners(card);

    expect(changed).toBe(false);
  });

  it('does NOT change image when image has no cornerRadius (nothing to fix)', () => {
    const image = frame({ id: 'img', type: 'image' } as never);
    const title = frame({ id: 't', type: 'text', content: 'X' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 12,
      children: [image, title],
    });

    const changed = clipCardImageCorners(card);

    // Image has no own cornerRadius — already correct, nothing to fix.
    expect(changed).toBe(false);
  });

  it('walks descendants and fixes nested cards', () => {
    const innerImage = frame({ id: 'img', type: 'image', cornerRadius: 8 } as never);
    const innerTitle = frame({ id: 't', type: 'text', content: 'X' } as never);
    const innerCard = frame({
      id: 'inner-card',
      role: 'card',
      cornerRadius: 8,
      children: [innerImage, innerTitle],
    });
    const wrapper = frame({
      id: 'wrap',
      role: 'section',
      children: [innerCard],
    });

    clipCardImageCorners(wrapper);

    expect((innerCard as PenNode & { clipContent?: boolean }).clipContent).toBe(true);
    expect((innerImage as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
  });

  it('preserves existing clipContent: true (does not flip it back)', () => {
    const image = frame({ id: 'img', type: 'image', cornerRadius: 12 } as never);
    const title = frame({ id: 't', type: 'text', content: 'X' } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 12,
      clipContent: true, // already set
      children: [image, title],
    });

    clipCardImageCorners(card);

    // clipContent stays true; image's own cornerRadius gets removed since
    // a positive scalar there would still draw on top of the clip.
    expect((card as PenNode & { clipContent?: boolean }).clipContent).toBe(true);
    expect((image as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
  });
});
