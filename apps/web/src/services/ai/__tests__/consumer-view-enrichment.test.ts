import { describe, expect, it } from 'vitest';
import { enrichNodeForAIConsumerView } from '../consumer-view-enrichment';

describe('consumer-view-enrichment', () => {
  it('adds image fill explain and keeps existing original size as the source of truth', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-1',
      type: 'rectangle',
      name: 'Background 11',
      x: 0.05,
      y: -0.39,
      width: 2560,
      height: 1600,
      fill: [
        {
          type: 'image',
          url: './assets/11-33.png',
          mode: 'stretch',
          originalSize: {
            width: 2644,
            height: 1696,
          },
          transform: {
            m00: 0.9682299494743347,
            m01: 0,
            m02: 0.019307976588606834,
            m10: 0,
            m11: 0.9433962106704712,
            m12: 0.041042111814022064,
          },
        },
      ],
    } as any);

    expect((enriched as any).fill[0]).toEqual({
      type: 'image',
      url: './assets/11-33.png',
      mode: 'stretch',
      originalSize: {
        width: 2644,
        height: 1696,
      },
      transform: {
        m00: 0.9682299494743347,
        m01: 0,
        m02: 0.019307976588606834,
        m10: 0,
        m11: 0.9433962106704712,
        m12: 0.041042111814022064,
      },
      explain:
        'This is not a full-image stretch; the source image is cropped before being mapped into the target bounds',
    });
  });

  it('can still infer original size from axis-aligned transform when upstream size is missing', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-2',
      type: 'rectangle',
      name: 'Poster',
      width: 2560,
      height: 1600,
      fill: [
        {
          type: 'image',
          url: './assets/poster.png',
          mode: 'stretch',
          transform: {
            m00: 0.9682299494743347,
            m01: 0,
            m02: 0.019307976588606834,
            m10: 0,
            m11: 0.9433962106704712,
            m12: 0.041042111814022064,
          },
        },
      ],
    } as any);

    expect((enriched as any).fill[0].originalSize).toEqual({
      width: 2644,
      height: 1696,
    });
    expect((enriched as any).fill[0].explain).toBe(
      'This is not a full-image stretch; the source image is cropped before being mapped into the target bounds',
    );
  });

  it('adds explain for gradients, auto-layout, clipContent, and image node objectFit', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'frame-1',
      type: 'frame',
      name: 'Hero',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      gap: 24,
      padding: [32, 24],
      justifyContent: 'space_between',
      alignItems: 'center',
      clipContent: true,
      fill: [
        {
          type: 'linear_gradient',
          angle: 135,
          stops: [
            { offset: 0, color: '#111111' },
            { offset: 1, color: '#999999' },
          ],
        },
      ],
      children: [
        {
          id: 'image-1',
          type: 'image',
          name: 'Hero Image',
          src: './assets/hero.png',
          objectFit: 'crop',
          width: 320,
          height: 180,
        },
      ],
    } as any);

    expect((enriched as any).fill[0].explain).toBe(
      'This is a linear gradient fill angled at 135deg with 2 color stops, so colors transition smoothly along that direction',
    );
    expect((enriched as any).explain).toBe(
      'This is a horizontal auto-layout container, Child gap is 24, Container padding is 32 24, Main-axis alignment is space between, Cross-axis alignment is center aligned. Width stretches to fill the available space in the parent container, Height grows automatically with its content. This container clips children that overflow its bounds',
    );
    expect((enriched as any).children[0].explain).toBe(
      'This is an image node. objectFit=crop uses cover to fill the container and may crop the edges. Width is fixed at 320px, Height is fixed at 180px',
    );
  });

  it('describes sizingBehavior hints such as fill_container(300) and fit_content(120)', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-3',
      type: 'frame',
      width: 'fill_container(300)',
      height: 'fit_content(120)',
    } as any);

    expect((enriched as any).explain).toBe(
      'Width stretches to fill the available space in the parent container, with a suggested value of about 300px, Height grows automatically with its content, with a suggested value of about 120px',
    );
  });

  it('adds explain for effects and reusable/component-instance semantics', () => {
    const reusable = enrichNodeForAIConsumerView({
      id: 'component-1',
      type: 'frame',
      name: 'Card Component',
      reusable: true,
      slot: ['media', 'actions'],
      effects: [
        {
          type: 'shadow',
          offsetX: 0,
          offsetY: 4,
          blur: 12,
          spread: -2,
          color: 'rgba(0,0,0,0.12)',
        },
      ],
    } as any);

    expect((reusable as any).explain).toBe(
      'Has shadow with offset 0px 4px, blur 12px, spread -2px. This is a reusable component definition node that other instances can reference. It declares slot regions: media, actions',
    );

    const instance = enrichNodeForAIConsumerView({
      id: 'instance-1',
      type: 'ref',
      ref: 'component-1',
      descendants: {
        'child-1': { visible: false },
        'child-2': { opacity: 0.5 },
      },
    } as any);

    expect((instance as any).explain).toBe(
      'This is a component instance node referencing source node component-1. This instance overrides 2 descendant nodes',
    );
  });

  it('adds explain for textGrowth, lineHeight, and text alignment semantics', () => {
    const textNode = enrichNodeForAIConsumerView({
      id: 'text-hero',
      type: 'text',
      content: 'Hello world',
      width: 'fill_container',
      textGrowth: 'fixed-width',
      lineHeight: 1.5,
      textAlign: 'center',
      textAlignVertical: 'middle',
    } as any);

    expect((textNode as any).explain).toBe(
      'This is a text node. textGrowth=fixed-width wraps text to the current width and grows vertically with the content. Line-height multiplier is 1.5. Horizontal alignment is center. Vertical alignment is middle. Width stretches to fill the available space in the parent container',
    );
  });

  it('adds explain for variable refs and theme overrides', () => {
    const themed = enrichNodeForAIConsumerView({
      id: 'node-theme',
      type: 'frame',
      opacity: '$opacity-soft',
      theme: {
        ColorScheme: 'Dark',
        Density: 'Compact',
      },
      fill: [{ type: 'solid', color: '$surface-bg' }],
      stroke: {
        thickness: '$border-width',
        fill: [{ type: 'solid', color: '$border-color' }],
      },
      effects: [
        {
          type: 'shadow',
          offsetX: '$shadow-x',
          offsetY: '$shadow-y',
          blur: '$shadow-blur',
          spread: '$shadow-spread',
          color: '$shadow-color',
        },
      ],
    } as any);

    expect((themed as any).explain).toBe(
      'Has shadow effect. opacity uses design token $opacity-soft. fill color uses design token $surface-bg. stroke thickness uses design token $border-width. stroke color uses design token $border-color. shadow color uses design token $shadow-color. shadow blur radius uses design token $shadow-blur. shadow X offset uses design token $shadow-x. shadow Y offset uses design token $shadow-y. shadow spread uses design token $shadow-spread. These values come from design-system tokens rather than hard-coded constants. This node carries theme override context: ColorScheme=Dark, Density=Compact',
    );
  });
});
