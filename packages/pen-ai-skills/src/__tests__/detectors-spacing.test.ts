import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import {
  detectEdgeSectionPadding,
  detectStackedHorizontalPadding,
} from '../diagnostics/detectors-spacing';

// 2026-05-09 user report — image 6: "Categories" mobile section had its
// chip text glued to the screen edge because both the page root and the
// section frame had 0 left padding. The detector flags the page root
// (single fix point) and suggests a 16px horizontal gutter.

const mobileRoot = (children: PenNode[], padding?: unknown): PenNode =>
  ({
    id: 'page',
    type: 'frame',
    width: 393,
    height: 852,
    layout: 'vertical',
    padding,
    children,
  }) as unknown as PenNode;

const section = (
  id: string,
  children: PenNode[],
  opts: { role?: string; padding?: unknown } = {},
): PenNode =>
  ({
    id,
    type: 'frame',
    layout: 'vertical',
    role: opts.role,
    padding: opts.padding,
    children,
  }) as unknown as PenNode;

const text = (id: string, content = 'Hello'): PenNode =>
  ({ id, type: 'text', content }) as unknown as PenNode;

const icon = (id: string): PenNode =>
  ({ id, type: 'icon_font', iconName: 'star' }) as unknown as PenNode;

const image = (id: string): PenNode => ({ id, type: 'image' }) as unknown as PenNode;

describe('detectEdgeSectionPadding', () => {
  it('flags mobile root with padding=0 and a content section also at padding=0', () => {
    const root = mobileRoot([section('categories', [text('t1', 'Tacos'), text('t2', 'Pizza')])]);
    // Need >= 2 children on root to qualify as a real page; add a second.
    (root as unknown as { children: PenNode[] }).children.push(section('news', [text('t3')]));

    const issues = detectEdgeSectionPadding(root);

    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('page');
    expect(issues[0].category).toBe('edge-section-padding');
    expect(issues[0].suggestedValue).toEqual([0, 16, 0, 16]);
  });

  it('flags when offending section contains an icon (not just text)', () => {
    const root = mobileRoot([
      section('iconbar', [icon('i1'), icon('i2')]),
      section('body', [text('t')]),
    ]);

    const issues = detectEdgeSectionPadding(root);

    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('page');
  });

  it('does NOT flag when root already has horizontal padding', () => {
    const root = mobileRoot(
      [section('cat', [text('t1')]), section('news', [text('t2')])],
      [0, 16, 0, 16],
    );

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag desktop-shaped roots (width > 480)', () => {
    const root = {
      id: 'page',
      type: 'frame',
      width: 1280,
      height: 800,
      layout: 'vertical',
      children: [section('cat', [text('t1')]), section('news', [text('t2')])],
    } as unknown as PenNode;

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag tiny component-shaped frames (width < 320)', () => {
    const root = {
      id: 'card',
      type: 'frame',
      width: 240,
      height: 100,
      layout: 'vertical',
      children: [section('a', [text('t1')]), section('b', [text('t2')])],
    } as unknown as PenNode;

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag full-bleed chrome roles (top-nav, bottom-nav, status-bar)', () => {
    const root = mobileRoot([
      section('topnav', [text('back'), icon('back-icon')], { role: 'top-nav' }),
      section('botnav', [icon('home'), icon('search')], { role: 'bottom-nav' }),
    ]);

    // Both children are full-bleed chrome — no offending content sections.
    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag image-only sections (intentional full-bleed media)', () => {
    const root = mobileRoot([section('hero', [image('img1')]), section('banner', [image('img2')])]);

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag sections that have their own left padding > 0', () => {
    const root = mobileRoot([
      section('a', [text('t1')], { padding: [16, 16, 16, 16] }),
      section('b', [text('t2')], { padding: 16 }),
    ]);

    // Both sections take care of gutters themselves — root staying at 0
    // is intentional.
    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('preserves vertical padding when suggesting the fix', () => {
    const root = mobileRoot(
      [section('a', [text('t1')]), section('b', [text('t2')])],
      [24, 0, 32, 0],
    );

    const issues = detectEdgeSectionPadding(root);

    expect(issues).toHaveLength(1);
    expect(issues[0].suggestedValue).toEqual([24, 16, 32, 16]);
  });

  it('expands shorthand number padding (e.g. padding: 0) to 4-tuple keeping top/bottom 0', () => {
    const root = mobileRoot([section('a', [text('t1')]), section('b', [text('t2')])], 0);

    const issues = detectEdgeSectionPadding(root);

    expect(issues).toHaveLength(1);
    expect(issues[0].suggestedValue).toEqual([0, 16, 0, 16]);
  });

  it('walks descendants — nested mobile page mockup in a desktop wrapper', () => {
    const innerRoot = mobileRoot([
      section('cat', [text('t1'), text('t2')]),
      section('news', [text('t3')]),
    ]);
    const wrapper = {
      id: 'wrap',
      type: 'frame',
      width: 1440,
      height: 900,
      layout: 'horizontal',
      children: [innerRoot],
    } as unknown as PenNode;

    const issues = detectEdgeSectionPadding(wrapper);

    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('page');
  });

  it('does NOT flag mobile root with only a single chrome child (no content sections)', () => {
    const root = mobileRoot([
      section('topnav', [text('Title')], { role: 'top-nav' }),
      section('botnav', [icon('home')], { role: 'bottom-nav' }),
    ]);

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  // 2026-05-10 Codex stop-hook review — detector was firing on any
  // mobile-width frame regardless of height, so a 393×88 bottom-nav
  // component being designed in isolation got page-level horizontal
  // padding forced on it. Height-and-aspect guards keep components out
  // of the page-detection path.

  it('does NOT flag a mobile-width bottom-nav component (393×88)', () => {
    const root = {
      id: 'botnav',
      type: 'frame',
      width: 393,
      height: 88,
      layout: 'horizontal',
      children: [icon('home'), icon('search'), text('Cart'), text('Profile')],
    } as unknown as PenNode;

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag a mobile-width card-shaped component (393×200)', () => {
    const root = {
      id: 'card',
      type: 'frame',
      width: 393,
      height: 200,
      layout: 'vertical',
      children: [section('row1', [text('t1'), text('t2')]), section('row2', [text('t3')])],
    } as unknown as PenNode;

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('does NOT flag a mobile-width short component (393×500)', () => {
    // Aspect ratio 500/393 = 1.27 < 1.5 — fails the aspect guard even
    // though height 500 alone would clear the absolute minimum.
    const root = {
      id: 'panel',
      type: 'frame',
      width: 393,
      height: 500,
      layout: 'vertical',
      children: [section('a', [text('t1')]), section('b', [text('t2')])],
    } as unknown as PenNode;

    expect(detectEdgeSectionPadding(root)).toHaveLength(0);
  });

  it('flags an iPhone SE shaped page (320×568) — minimum legitimate phone', () => {
    const root = {
      id: 'page',
      type: 'frame',
      width: 320,
      height: 568,
      layout: 'vertical',
      children: [section('cat', [text('t1')]), section('news', [text('t2')])],
    } as unknown as PenNode;

    const issues = detectEdgeSectionPadding(root);

    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('page');
  });
});

// 2026-05-10 user report — DeepSeek "Bistro" mobile food app shipped with
// root padding [0,16,0,16] AND a "Today's Specials" section padding [0,24],
// effective 40px gutter on a 375px page. Looked pinched / "too much
// padding". detectStackedHorizontalPadding catches the page-vs-section
// double-pad pattern (info-only — section can legitimately want larger
// inset for emphasis, so detect-only and let user/agent decide).

describe('detectStackedHorizontalPadding', () => {
  it('flags section whose horizontal padding stacks with mobile root', () => {
    const root = mobileRoot(
      [section('specials', [text('t1')], { padding: [0, 24] }), section('news', [text('t2')])],
      [0, 16, 0, 16],
    );
    const issues = detectStackedHorizontalPadding(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('specials');
    expect(issues[0].category).toBe('stacked-horizontal-padding');
    expect(issues[0].severity).toBe('info');
    expect(issues[0].reason).toMatch(/stacks with root/);
  });

  it('flags multiple offending sections in one root', () => {
    const root = mobileRoot(
      [
        section('a', [text('t1')], { padding: [0, 24] }),
        section('b', [text('t2')], { padding: 16 }),
        section('c', [text('t3')]),
      ],
      [0, 16, 0, 16],
    );
    const issues = detectStackedHorizontalPadding(root);
    expect(issues).toHaveLength(2);
    expect(new Set(issues.map((i) => i.nodeId))).toEqual(new Set(['a', 'b']));
  });

  it('does NOT flag when only root has horizontal padding (the goal state)', () => {
    const root = mobileRoot(
      [section('a', [text('t1')]), section('b', [text('t2')])],
      [0, 16, 0, 16],
    );
    expect(detectStackedHorizontalPadding(root)).toHaveLength(0);
  });

  it('does NOT flag when root has 0 horizontal padding (sections own gutter is the only one)', () => {
    const root = mobileRoot([
      section('a', [text('t1')], { padding: [0, 24] }),
      section('b', [text('t2')], { padding: [0, 24] }),
    ]);
    expect(detectStackedHorizontalPadding(root)).toHaveLength(0);
  });

  it('does NOT flag full-bleed roles (top-nav / hero / banner) even when their padding stacks', () => {
    const root = mobileRoot(
      [
        section('topnav', [text('Title')], { role: 'top-nav', padding: [0, 24] }),
        section('hero', [text('h')], { role: 'hero', padding: [0, 24] }),
      ],
      [0, 16, 0, 16],
    );
    expect(detectStackedHorizontalPadding(root)).toHaveLength(0);
  });

  it('does NOT flag desktop-shaped roots (component-internal padding stacking is legitimate)', () => {
    // The detector is page-shape-only: a 1200px desktop card with a chip
    // having internal padding is a totally normal nested-component pattern,
    // not the page-gutter doubling we care about.
    const root = {
      id: 'page',
      type: 'frame',
      width: 1200,
      height: 800,
      layout: 'vertical',
      padding: [0, 24, 0, 24],
      children: [
        section('a', [text('t1')], { padding: [0, 16] }),
        section('b', [text('t2')], { padding: [0, 16] }),
      ],
    } as unknown as PenNode;
    expect(detectStackedHorizontalPadding(root)).toHaveLength(0);
  });
});
