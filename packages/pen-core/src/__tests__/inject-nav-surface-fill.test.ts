import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { injectMissingNavSurfaceFill } from '../layout/inject-nav-surface-fill';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f1',
    type: 'frame',
    ...props,
  }) as PenNode;

const solidFill = (color: string) => [{ type: 'solid' as const, color }];

describe('injectMissingNavSurfaceFill', () => {
  it('injects $color-surface on a top-level navbar that has no fill', () => {
    const navbar = frame({
      id: 'bottom-nav',
      role: 'navbar',
      children: [
        frame({ id: 'home', role: 'icon-button' }),
        frame({ id: 'search', role: 'icon-button' }),
      ],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [navbar],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(true);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual([
      { type: 'solid', color: '$color-surface' },
    ]);
  });

  it('preserves an existing nav fill (sub-agent intent)', () => {
    const navbar = frame({
      id: 'top-bar',
      role: 'top-app-bar',
      fill: solidFill('#0F172A'), // intentionally dark
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFFFFF'),
      children: [navbar],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(false);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0F172A'));
  });

  it('preserves a linear-gradient nav fill (does not overwrite with solid)', () => {
    const gradientFill = [
      {
        type: 'linear_gradient' as const,
        stops: [
          { offset: 0, color: '#F97316' },
          { offset: 1, color: '#EA580C' },
        ],
        angle: 90,
      },
    ];
    const navbar = frame({
      id: 'top-bar',
      role: 'top-app-bar',
      fill: gradientFill as never,
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFFFFF'),
      children: [navbar],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(false);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual(gradientFill);
  });

  it('preserves a radial-gradient nav fill', () => {
    const radial = [
      {
        type: 'radial_gradient' as const,
        stops: [
          { offset: 0, color: '#FFFFFF' },
          { offset: 1, color: '#000000' },
        ],
        center: { x: 0.5, y: 0.5 },
      },
    ];
    const navbar = frame({
      id: 'splash-nav',
      role: 'navbar',
      fill: radial as never,
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [navbar],
    });
    injectMissingNavSurfaceFill(root);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual(radial);
  });

  it('preserves an image nav fill (branded nav surface with photo)', () => {
    const imageFill = [
      {
        type: 'image' as const,
        src: 'https://example.com/banner.jpg',
        scaleMode: 'cover',
      },
    ];
    const navbar = frame({
      id: 'hero-nav',
      role: 'top-app-bar',
      fill: imageFill as never,
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFFFFF'),
      children: [navbar],
    });
    injectMissingNavSurfaceFill(root);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual(imageFill);
  });

  it('does not touch nav frames nested inside cards/sections', () => {
    const nestedNav = frame({
      id: 'inner-nav',
      role: 'tab-row',
      // no fill — but it's nested, so left alone
    });
    const card = frame({
      id: 'card',
      role: 'card',
      fill: solidFill('#FFFFFF'),
      children: [nestedNav],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [card],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(false);
    expect((nestedNav as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('matches all known navigation role names', () => {
    const navRoles = [
      'navbar',
      'nav',
      'tab-bar',
      'bottom-tab-bar',
      'top-nav-bar',
      'top-app-bar',
      'tab-row',
    ];
    const children = navRoles.map((role) => frame({ id: `nav-${role}`, role, children: [] }));
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children,
    });
    injectMissingNavSurfaceFill(root);
    for (const navFrame of children) {
      expect((navFrame as PenNode & { fill?: unknown }).fill).toEqual([
        { type: 'solid', color: '$color-surface' },
      ]);
    }
  });

  it('skips non-frame children and unrelated roles', () => {
    const text = { id: 'h', type: 'text', role: 'heading', content: 'Foo' } as PenNode;
    const sectionWithoutFill = frame({ id: 'sec', role: 'section', children: [] });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [text, sectionWithoutFill],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(false);
    expect((sectionWithoutFill as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });
});
