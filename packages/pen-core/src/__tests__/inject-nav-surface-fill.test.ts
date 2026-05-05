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

  it('treats malformed solid fills (missing/empty color) as unfilled and injects', () => {
    // Three nav frames each with a different malformed solid:
    //   - fill: [{type:'solid'}]            (missing color)
    //   - fill: [{type:'solid', color: ''}] (empty string)
    //   - fill: [{type:'invalid'}]          (unknown type)
    // None render visibly; the pass should patch all three.
    const noColorNav = frame({
      id: 'nav-1',
      role: 'navbar',
      fill: [{ type: 'solid' }] as never,
    });
    const emptyColorNav = frame({
      id: 'nav-2',
      role: 'tab-bar',
      fill: [{ type: 'solid', color: '' }] as never,
    });
    const unknownTypeNav = frame({
      id: 'nav-3',
      role: 'top-app-bar',
      fill: [{ type: 'mystery' }] as never,
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [noColorNav, emptyColorNav, unknownTypeNav],
    });
    const changed = injectMissingNavSurfaceFill(root);
    expect(changed).toBe(true);
    for (const navFrame of [noColorNav, emptyColorNav, unknownTypeNav]) {
      expect((navFrame as PenNode & { fill?: unknown }).fill).toEqual([
        { type: 'solid', color: '$color-surface' },
      ]);
    }
  });

  it('treats gradient with no stops and image with no src as unfilled', () => {
    const emptyGradient = frame({
      id: 'nav-grad',
      role: 'navbar',
      fill: [{ type: 'linear_gradient', stops: [] }] as never,
    });
    const noSrcImage = frame({
      id: 'nav-img',
      role: 'top-app-bar',
      fill: [{ type: 'image', src: '' }] as never,
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFFFFF'),
      children: [emptyGradient, noSrcImage],
    });
    injectMissingNavSurfaceFill(root);
    for (const navFrame of [emptyGradient, noSrcImage]) {
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

  it('injects an upward shadow on bottom-tab-bar so it lifts off cream pages', () => {
    // Regression: warm-light themes resolve `$color-surface` to white
    // and `$color-bg-deep` to cream (#FFF8F0). The luminance delta is
    // ~0.03 — the nav looks transparent against the page even with a
    // valid surface fill. The inject pass also stamps a soft upward
    // shadow on bottom-positioned nav so the separation survives the
    // low fill-contrast case.
    const nav = frame({
      id: 'bottom-nav',
      role: 'bottom-tab-bar',
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [nav],
    });
    injectMissingNavSurfaceFill(root);
    const effects = (nav as PenNode & { effects?: Array<{ type?: string; offsetY?: number }> })
      .effects;
    expect(Array.isArray(effects)).toBe(true);
    expect(effects?.[0]?.type).toBe('shadow');
    // Bottom nav → negative offsetY → shadow points up.
    expect(effects?.[0]?.offsetY).toBeLessThan(0);
  });

  it('injects a downward shadow on top nav (top-app-bar / top-nav-bar / navbar)', () => {
    // Top-positioned nav can't use an upward shadow (it would cling
    // to the screen edge). The inject pass picks `offsetY > 0` for
    // every non-bottom role.
    const topRoles = ['top-app-bar', 'top-nav-bar', 'navbar'];
    for (const role of topRoles) {
      const nav = frame({ id: `nav-${role}`, role, children: [] });
      const root = frame({
        id: 'root',
        fill: solidFill('#FFF8F0'),
        children: [nav],
      });
      injectMissingNavSurfaceFill(root);
      const effects = (nav as PenNode & { effects?: Array<{ type?: string; offsetY?: number }> })
        .effects;
      expect(effects?.[0]?.type).toBe('shadow');
      expect(effects?.[0]?.offsetY).toBeGreaterThan(0);
    }
  });

  it('preserves existing effects (sub-agent intentional shadow / glow)', () => {
    const intentionalShadow = [
      { type: 'shadow', offsetX: 0, offsetY: 8, blur: 24, spread: 0, color: '#00000033' },
    ];
    const nav = frame({
      id: 'nav-with-effects',
      role: 'bottom-tab-bar',
      effects: intentionalShadow as never,
      children: [],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [nav],
    });
    injectMissingNavSurfaceFill(root);
    expect((nav as PenNode & { effects?: unknown }).effects).toEqual(intentionalShadow);
  });
});
