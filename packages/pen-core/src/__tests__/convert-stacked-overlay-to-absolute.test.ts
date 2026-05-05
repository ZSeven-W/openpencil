import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { convertStackedOverlayToAbsolute } from '../layout/convert-stacked-overlay-to-absolute';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    ...props,
  }) as PenNode;

const image = (props: Partial<PenNode>): PenNode =>
  ({
    id: 'img',
    type: 'image',
    ...props,
  }) as PenNode;

const rect = (props: Partial<PenNode>): PenNode =>
  ({
    id: 'rct',
    type: 'rectangle',
    ...props,
  }) as PenNode;

const text = (props: Partial<PenNode>): PenNode =>
  ({
    id: 't',
    type: 'text',
    ...props,
  }) as PenNode;

describe('convertStackedOverlayToAbsolute', () => {
  it("flips a hero with image+overlay+content from layout='vertical' to 'none'", () => {
    // Real M2.7 food-app shape: 200px tall hero, layout=vertical,
    // 3 stacked children — bg image, gradient overlay, content.
    // Sequential stacking overflows the 200 height; switching to
    // layout='none' lets each child sit at its own (default 0,0)
    // origin, layering them as the model intended.
    const hero = frame({
      id: 'hero',
      width: 'fill_container',
      height: 200,
      layout: 'vertical',
      children: [
        image({ id: 'bg', width: 'fill_container', height: 200 }),
        rect({ id: 'overlay', width: 'fill_container', height: 200 }),
        frame({
          id: 'content',
          width: 'fill_container',
          height: 'fit_content',
          layout: 'vertical',
          children: [
            text({ id: 'title', content: 'Hungry?' }),
            frame({ id: 'cta', width: 'fit_content', height: 48 }),
          ],
        }),
      ],
    });
    const root = frame({ id: 'root', children: [hero] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(true);
    expect((hero as PenNode & { layout?: string }).layout).toBe('none');
  });

  it('matches when one of the bg-likes uses fill_container instead of a numeric match', () => {
    const hero = frame({
      id: 'hero2',
      width: 'fill_container',
      height: 220,
      layout: 'vertical',
      children: [
        image({ id: 'bg', width: 'fill_container', height: 220 }),
        rect({ id: 'overlay', width: 'fill_container', height: 'fill_container' }),
        text({ id: 'caption', content: 'Layered above bg' }),
      ],
    });
    const root = frame({ id: 'root', children: [hero] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(true);
    expect((hero as PenNode & { layout?: string }).layout).toBe('none');
  });

  it("doesn't touch normal vertical stacks (only one bg-like child)", () => {
    // Plain content section: section header + body text + button.
    // Only one full-height child (or none). Must not get re-laid-out
    // — converting to absolute would un-stack the content.
    const section = frame({
      id: 'section',
      width: 'fill_container',
      height: 300,
      layout: 'vertical',
      children: [
        text({ id: 'h', content: 'Heading' }),
        text({ id: 'b', content: 'Body text wraps to multiple lines' }),
        frame({ id: 'cta', width: 200, height: 44 }),
      ],
    });
    const root = frame({ id: 'root', children: [section] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(false);
    expect((section as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it("doesn't touch non-fixed-height containers (no overflow risk in fit_content)", () => {
    // fit_content / fill_container containers don't risk the stacked
    // overflow regression — the parent grows to fit children. Skip
    // them so we don't accidentally repair non-bug shapes.
    const card = frame({
      id: 'card',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [
        image({ id: 'bg', width: 'fill_container', height: 200 }),
        rect({ id: 'overlay', width: 'fill_container', height: 200 }),
      ],
    });
    const root = frame({ id: 'root', children: [card] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(false);
    expect((card as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('respects an explicit layout=horizontal (not the bug shape)', () => {
    // A horizontal-layout container with side-by-side image+overlay
    // is NOT the layered-hero pattern — the model likely meant a
    // 2-column row. Don't reach into horizontal layouts.
    const row = frame({
      id: 'row',
      width: 'fill_container',
      height: 200,
      layout: 'horizontal',
      children: [
        image({ id: 'left', width: 'fill_container', height: 200 }),
        rect({ id: 'right', width: 'fill_container', height: 200 }),
      ],
    });
    const root = frame({ id: 'root', children: [row] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(false);
    expect((row as PenNode & { layout?: string }).layout).toBe('horizontal');
  });

  it('walks nested heroes (nested-section regressions)', () => {
    const innerHero = frame({
      id: 'nested-hero',
      width: 'fill_container',
      height: 180,
      layout: 'vertical',
      children: [
        image({ id: 'bg', width: 'fill_container', height: 180 }),
        rect({ id: 'overlay', width: 'fill_container', height: 180 }),
        text({ id: 'cap', content: 'On top' }),
      ],
    });
    const wrapper = frame({
      id: 'wrap',
      role: 'section',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [innerHero],
    });
    const root = frame({ id: 'root', children: [wrapper] });
    const changed = convertStackedOverlayToAbsolute(root);
    expect(changed).toBe(true);
    expect((innerHero as PenNode & { layout?: string }).layout).toBe('none');
    // The wrapper itself was a fit_content section — left alone.
    expect((wrapper as PenNode & { layout?: string }).layout).toBe('vertical');
  });
});
