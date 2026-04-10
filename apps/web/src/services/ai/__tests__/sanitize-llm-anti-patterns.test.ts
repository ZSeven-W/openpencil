import { describe, it, expect } from 'vitest';
import type { PenNode } from '@/types/pen';
import {
  rewriteStackedEllipsesToRingFrames,
  rewriteAlternatingBarLabelSiblings,
  normalizeRingTrackProgressGeometry,
  rewritePseudoRingFrames,
  stripRingFrameFills,
  rewriteLlmAntiPatterns,
} from '../sanitize-llm-anti-patterns';

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

const strokeEllipse = (id: string, w: number, h: number, color: string, thickness = 10): PenNode =>
  ({
    id,
    type: 'ellipse',
    width: w,
    height: h,
    fill: [{ type: 'solid', color: '#00000000' }],
    stroke: { thickness, fill: [{ type: 'solid', color }] },
  }) as unknown as PenNode;

const text = (id: string, content: string): PenNode =>
  ({
    id,
    type: 'text',
    content,
    fontSize: 14,
    fontWeight: 500,
  }) as unknown as PenNode;

const bar = (id: string, height: number, color: string): PenNode =>
  ({
    id,
    type: 'frame',
    width: 44,
    height,
    fill: [{ type: 'solid', color }],
    children: [],
  }) as unknown as PenNode;

// ---------------------------------------------------------------------------
// rewriteStackedEllipsesToRingFrames
// ---------------------------------------------------------------------------

describe('rewriteStackedEllipsesToRingFrames', () => {
  it('rewrites 3 concentric ellipses + center text into nested ring frames (activity-ring regression)', () => {
    // The exact bug from the fitness app debug session:
    // layout='none' parent with 3 transparent-fill stroke-only ellipses
    // (120/84/52) and a center percentage text. With layout='none' all
    // 4 children render at (0,0) overlapping top-left. Rewriter must
    // produce a nested frame tree where each ring is a frame centered
    // inside its parent.
    const rings: PenNode = {
      id: 'rings-visual',
      type: 'frame',
      width: 120,
      height: 120,
      layout: 'none',
      children: [
        strokeEllipse('outer', 120, 120, '#00D09C', 12),
        strokeEllipse('middle', 84, 84, '#FF8A65', 10),
        strokeEllipse('inner', 52, 52, '#4FC3F7', 8),
        text('label', '78%'),
      ],
    } as unknown as PenNode;

    rewriteStackedEllipsesToRingFrames(rings);

    // The original node should now be a centering frame holding the
    // outermost ring.
    const root = rings as PenNode & {
      layout?: string;
      alignItems?: string;
      justifyContent?: string;
      children: PenNode[];
      width?: number;
      height?: number;
    };
    expect(root.id).toBe('rings-visual');
    expect(root.layout).toBe('horizontal');
    expect(root.alignItems).toBe('center');
    expect(root.justifyContent).toBe('center');
    expect(root.width).toBe(120);
    expect(root.height).toBe(120);
    expect(root.children).toHaveLength(1);

    // Outer ring: 120×120 frame with cornerRadius=60, stroke from outer ellipse
    const outer = root.children[0] as PenNode & {
      width?: number;
      height?: number;
      cornerRadius?: number;
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      layout?: string;
      alignItems?: string;
      justifyContent?: string;
      children: PenNode[];
      fill?: unknown[];
    };
    expect(outer.id).toBe('outer');
    expect(outer.width).toBe(120);
    expect(outer.height).toBe(120);
    expect(outer.cornerRadius).toBe(60);
    expect(outer.stroke?.thickness).toBe(12);
    expect(outer.stroke?.fill?.[0]?.color).toBe('#00D09C');
    expect(outer.layout).toBe('horizontal');
    expect(outer.alignItems).toBe('center');
    expect(outer.justifyContent).toBe('center');
    // Empty fill so the inner content shows through.
    expect(Array.isArray(outer.fill) && outer.fill.length === 0).toBe(true);
    expect(outer.children).toHaveLength(1);

    // Middle ring: 84×84 frame with cornerRadius=42
    const middle = outer.children[0] as PenNode & {
      width?: number;
      cornerRadius?: number;
      stroke?: { fill?: Array<{ color?: string }> };
      children: PenNode[];
    };
    expect(middle.id).toBe('middle');
    expect(middle.width).toBe(84);
    expect(middle.cornerRadius).toBe(42);
    expect(middle.stroke?.fill?.[0]?.color).toBe('#FF8A65');
    expect(middle.children).toHaveLength(1);

    // Inner ring: 52×52 frame with cornerRadius=26
    const inner = middle.children[0] as PenNode & {
      width?: number;
      cornerRadius?: number;
      stroke?: { fill?: Array<{ color?: string }> };
      children: PenNode[];
    };
    expect(inner.id).toBe('inner');
    expect(inner.width).toBe(52);
    expect(inner.cornerRadius).toBe(26);
    expect(inner.stroke?.fill?.[0]?.color).toBe('#4FC3F7');

    // The text label is the innermost child of the innermost ring.
    expect(inner.children).toHaveLength(1);
    const label = inner.children[0] as PenNode & { content?: string };
    expect(label.id).toBe('label');
    expect(label.content).toBe('78%');
  });

  it('rewrites 2-ring stack without any center text (empty progress ring)', () => {
    // Distinct sizes (60 vs 48) required by the rewriter — equal-size
    // pairs are rejected as ambiguous (likely a dot grid).
    const rings: PenNode = {
      id: 'simple-ring',
      type: 'frame',
      width: 60,
      height: 60,
      layout: 'none',
      children: [
        strokeEllipse('bg', 60, 60, '#333333', 6),
        strokeEllipse('fg', 48, 48, '#00D09C', 6),
      ],
    } as unknown as PenNode;

    rewriteStackedEllipsesToRingFrames(rings);
    const root = rings as PenNode & { layout?: string; children: PenNode[] };
    expect(root.layout).toBe('horizontal');
    expect(root.children).toHaveLength(1);
    // The innermost ring has no label children.
    const outer = root.children[0] as PenNode & { children: PenNode[] };
    const inner = outer.children[0] as PenNode & { children: PenNode[] };
    expect(inner.children).toHaveLength(0);
  });

  it('does NOT rewrite when parent layout is not "none"', () => {
    // A layout=horizontal parent is behaving; don't touch it.
    const parent: PenNode = {
      id: 'horiz-parent',
      type: 'frame',
      width: 200,
      height: 60,
      layout: 'horizontal',
      alignItems: 'center',
      children: [strokeEllipse('a', 40, 40, '#00D09C'), strokeEllipse('b', 40, 40, '#FF8A65')],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('does NOT rewrite when ellipses have the same size (not concentric)', () => {
    // Two 40×40 ellipses at (0,0) under layout=none is probably a dot
    // row with bad positioning — ambiguous intent, don't touch.
    const parent: PenNode = {
      id: 'dots',
      type: 'frame',
      width: 100,
      height: 40,
      layout: 'none',
      children: [strokeEllipse('a', 40, 40, '#00D09C'), strokeEllipse('b', 40, 40, '#FF8A65')],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('does NOT rewrite when an ellipse has a solid fill (not a ring)', () => {
    const parent: PenNode = {
      id: 'filled',
      type: 'frame',
      width: 100,
      height: 100,
      layout: 'none',
      children: [
        {
          id: 'solid1',
          type: 'ellipse',
          width: 100,
          height: 100,
          fill: [{ type: 'solid', color: '#FF0000' }],
          stroke: { thickness: 4, fill: [{ type: 'solid', color: '#000' }] },
        } as unknown as PenNode,
        strokeEllipse('ring2', 60, 60, '#00D09C'),
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('does NOT rewrite when an ellipse has explicit x or y (deliberate absolute positioning)', () => {
    const parent: PenNode = {
      id: 'positioned',
      type: 'frame',
      width: 120,
      height: 120,
      layout: 'none',
      children: [
        strokeEllipse('a', 80, 80, '#00D09C'),
        { ...strokeEllipse('b', 40, 40, '#FF8A65'), x: 20, y: 20 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('does NOT rewrite when a child is a frame (mixed intent — LLM wanted structure)', () => {
    const parent: PenNode = {
      id: 'mixed',
      type: 'frame',
      width: 100,
      height: 100,
      layout: 'none',
      children: [
        strokeEllipse('ring', 100, 100, '#00D09C'),
        { id: 'inner-frame', type: 'frame', width: 60, height: 60 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('does NOT rewrite when there is only one ellipse (no concentric intent)', () => {
    const parent: PenNode = {
      id: 'single',
      type: 'frame',
      width: 60,
      height: 60,
      layout: 'none',
      children: [strokeEllipse('only', 60, 60, '#00D09C'), text('t', '100')],
    } as unknown as PenNode;
    const before = JSON.stringify(parent);
    rewriteStackedEllipsesToRingFrames(parent);
    expect(JSON.stringify(parent)).toBe(before);
  });

  it('recurses into nested containers', () => {
    const outer: PenNode = {
      id: 'outer',
      type: 'frame',
      layout: 'vertical',
      children: [
        {
          id: 'ring-parent',
          type: 'frame',
          width: 100,
          height: 100,
          layout: 'none',
          children: [
            strokeEllipse('r1', 100, 100, '#00D09C'),
            strokeEllipse('r2', 60, 60, '#FF8A65'),
            text('pct', '75%'),
          ],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    rewriteStackedEllipsesToRingFrames(outer);
    const ringParent = (outer as unknown as { children: PenNode[] }).children[0] as PenNode & {
      layout?: string;
      children: PenNode[];
    };
    expect(ringParent.layout).toBe('horizontal');
    expect(ringParent.children).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// rewriteAlternatingBarLabelSiblings
// ---------------------------------------------------------------------------

describe('rewriteAlternatingBarLabelSiblings', () => {
  it('groups 5-bar/5-label alternation into 5 vertical columns (bar chart regression)', () => {
    // The exact bug from the fitness app: 10 flat children arranged as
    // [bar, label, bar, label, …]. With justifyContent=space_between
    // the labels land between bars instead of under them.
    const chart: PenNode = {
      id: 'weekly-chart',
      type: 'frame',
      width: 343,
      height: 110,
      layout: 'horizontal',
      justifyContent: 'space_between',
      alignItems: 'end',
      children: [
        bar('mon-bar', 40, '#2A2A2A'),
        text('mon-label', 'Mon'),
        bar('tue-bar', 80, '#00D09C'),
        text('tue-label', 'Tue'),
        bar('wed-bar', 40, '#2A2A2A'),
        text('wed-label', 'Wed'),
        bar('thu-bar', 70, '#00D09C'),
        text('thu-label', 'Thu'),
        bar('fri-bar', 40, '#2A2A2A'),
        text('fri-label', 'Fri'),
      ],
    } as unknown as PenNode;

    rewriteAlternatingBarLabelSiblings(chart);

    const root = chart as PenNode & {
      layout?: string;
      justifyContent?: string;
      children: PenNode[];
    };
    // Parent keeps layout=horizontal and justifyContent=space_between
    // so the columns distribute across the full width.
    expect(root.layout).toBe('horizontal');
    expect(root.justifyContent).toBe('space_between');
    expect(root.children).toHaveLength(5);

    for (const col of root.children) {
      const c = col as PenNode & {
        layout?: string;
        alignItems?: string;
        children: PenNode[];
      };
      expect(c.layout).toBe('vertical');
      expect(c.alignItems).toBe('center');
      expect(c.children).toHaveLength(2);
      expect(c.children[0].type).toBe('frame'); // bar
      expect(c.children[1].type).toBe('text'); // label
    }

    // Verify the first column is Mon (preserves order).
    const firstCol = root.children[0] as PenNode & { children: PenNode[] };
    const firstBar = firstCol.children[0] as PenNode & { id: string };
    const firstLabel = firstCol.children[1] as PenNode & { content?: string };
    expect(firstBar.id).toBe('mon-bar');
    expect(firstLabel.content).toBe('Mon');
  });

  it('does NOT rewrite a horizontal row of 2 bars (too few pairs)', () => {
    // Minimum 3 pairs (6 children) required — 2 bars + 2 labels is
    // likely a stat widget, not a chart.
    const row: PenNode = {
      id: 'two-bar-row',
      type: 'frame',
      width: 200,
      height: 80,
      layout: 'horizontal',
      children: [
        bar('a', 40, '#00D09C'),
        text('a-label', 'A'),
        bar('b', 60, '#FF8A65'),
        text('b-label', 'B'),
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(row);
    rewriteAlternatingBarLabelSiblings(row);
    expect(JSON.stringify(row)).toBe(before);
  });

  it('does NOT rewrite when the pattern is not strictly alternating (bar, text, text, bar, …)', () => {
    const broken: PenNode = {
      id: 'broken-pattern',
      type: 'frame',
      layout: 'horizontal',
      children: [
        bar('a', 40, '#000'),
        text('a-label', 'A'),
        text('extra', 'rogue'), // breaks alternation
        bar('b', 60, '#000'),
        text('b-label', 'B'),
        bar('c', 80, '#000'),
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(broken);
    rewriteAlternatingBarLabelSiblings(broken);
    expect(JSON.stringify(broken)).toBe(before);
  });

  it('does NOT rewrite when the frames contain text (they are cards, not bars)', () => {
    // If a "bar" frame already has a text descendant, it's not a
    // plain bar — could be a card with a title and a value next to a
    // label. Refuse to touch.
    const withTitles: PenNode = {
      id: 'labeled-cards',
      type: 'frame',
      layout: 'horizontal',
      children: [
        {
          id: 'card1',
          type: 'frame',
          children: [text('c1-title', 'Card 1')],
        } as unknown as PenNode,
        text('t1', 'Desc 1'),
        {
          id: 'card2',
          type: 'frame',
          children: [text('c2-title', 'Card 2')],
        } as unknown as PenNode,
        text('t2', 'Desc 2'),
        {
          id: 'card3',
          type: 'frame',
          children: [text('c3-title', 'Card 3')],
        } as unknown as PenNode,
        text('t3', 'Desc 3'),
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(withTitles);
    rewriteAlternatingBarLabelSiblings(withTitles);
    expect(JSON.stringify(withTitles)).toBe(before);
  });

  it('does NOT rewrite a vertical layout parent (not a chart row)', () => {
    const stacked: PenNode = {
      id: 'vstack',
      type: 'frame',
      layout: 'vertical',
      children: [
        bar('a', 40, '#000'),
        text('al', 'A'),
        bar('b', 40, '#000'),
        text('bl', 'B'),
        bar('c', 40, '#000'),
        text('cl', 'C'),
      ],
    } as unknown as PenNode;
    const before = JSON.stringify(stacked);
    rewriteAlternatingBarLabelSiblings(stacked);
    expect(JSON.stringify(stacked)).toBe(before);
  });

  it('recurses into nested containers', () => {
    const outer: PenNode = {
      id: 'outer',
      type: 'frame',
      layout: 'vertical',
      children: [
        {
          id: 'chart-wrap',
          type: 'frame',
          layout: 'horizontal',
          children: [
            bar('a', 40, '#000'),
            text('al', 'A'),
            bar('b', 40, '#000'),
            text('bl', 'B'),
            bar('c', 40, '#000'),
            text('cl', 'C'),
          ],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    rewriteAlternatingBarLabelSiblings(outer);
    const chartWrap = (outer as unknown as { children: PenNode[] }).children[0] as PenNode & {
      children: PenNode[];
    };
    expect(chartWrap.children).toHaveLength(3);
    expect((chartWrap.children[0] as PenNode & { layout?: string }).layout).toBe('vertical');
  });
});

describe('normalizeRingTrackProgressGeometry', () => {
  it('aligns a progress arc path to its sibling ellipse track bbox', () => {
    const parent: PenNode = {
      id: 'ring',
      type: 'frame',
      layout: 'none',
      children: [
        {
          id: 'track',
          type: 'ellipse',
          name: 'Steps Track',
          width: 80,
          height: 80,
          x: 5,
          y: 5,
          stroke: { thickness: 8, fill: [{ type: 'solid', color: '#111111' }] },
        } as unknown as PenNode,
        {
          id: 'arc',
          type: 'path',
          name: 'Steps Progress',
          d: 'M 45 5 A 40 40 0 1 1 44.99 5',
          width: 90,
          height: 90,
          stroke: {
            thickness: 8,
            fill: [{ type: 'solid', color: '#22C55E' }],
            dashPattern: [213.5, 213.5],
            dashOffset: 32,
            cap: 'round',
          },
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    normalizeRingTrackProgressGeometry(parent);

    const arc = (parent as PenNode & { children: PenNode[] }).children[1] as PenNode & {
      d: string;
      width: number;
      height: number;
      x?: number;
      y?: number;
    };

    expect(arc.width).toBe(80);
    expect(arc.height).toBe(80);
    expect(arc.x).toBe(5);
    expect(arc.y).toBe(5);
    expect(arc.d).toContain('A 40 40 0 1 1');
  });
});

describe('rewritePseudoRingFrames', () => {
  it('rewrites a pill-shaped faux ring into a stable square ring shell', () => {
    const pseudo = {
      id: 'ring',
      type: 'frame',
      name: 'Ring',
      width: 170,
      height: 100,
      cornerRadius: 50,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      stroke: {
        thickness: 12,
        fill: [{ type: 'solid', color: '#2D2D4A' }],
      },
      children: [
        {
          id: 'progress',
          type: 'frame',
          name: 'Progress Ring',
          width: 100,
          height: 100,
          cornerRadius: 50,
          fill: [],
          stroke: {
            thickness: 12,
            fill: [{ type: 'solid', color: '#22C55E' }],
          },
        },
        {
          id: 'inner',
          type: 'frame',
          name: 'Inner Circle',
          width: 70,
          height: 70,
          cornerRadius: 35,
          fill: [{ type: 'solid', color: '#1A1A1A' }],
          children: [
            {
              id: 'icon',
              type: 'icon_font',
              iconFontName: 'activity',
              width: 24,
              height: 24,
            },
          ],
        },
      ],
    } as unknown as PenNode;

    rewritePseudoRingFrames(pseudo);

    const root = pseudo as PenNode & {
      width?: number;
      height?: number;
      cornerRadius?: number;
      layout?: string;
      fill?: unknown[];
      children: PenNode[];
    };
    const progress = root.children[0] as PenNode & {
      width?: number;
      height?: number;
      x?: number;
      y?: number;
      stroke?: { thickness?: number };
    };
    const inner = root.children[1] as PenNode & {
      width?: number;
      height?: number;
      x?: number;
      y?: number;
      cornerRadius?: number;
    };

    expect(root.width).toBe(100);
    expect(root.height).toBe(100);
    expect(root.cornerRadius).toBe(50);
    expect(root.layout).toBe('none');
    expect(root.fill).toEqual([]);
    expect(progress.width).toBe(100);
    expect(progress.height).toBe(100);
    expect(progress.x).toBe(0);
    expect(progress.y).toBe(0);
    expect(progress.stroke?.thickness).toBeLessThan(12);
    expect(inner.x).toBe(15);
    expect(inner.y).toBe(15);
    expect(inner.cornerRadius).toBe(35);
  });
});

describe('stripRingFrameFills', () => {
  it('removes accidental white or neutral fills from stroked ring frames', () => {
    const ring = {
      id: 'activity-ring',
      type: 'frame',
      name: 'Outer Ring',
      width: 160,
      height: 160,
      cornerRadius: 80,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      stroke: { thickness: 12, fill: [{ type: 'solid', color: '#00D09C' }] },
      effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: '#0000001A' }],
      children: [text('value', '78%')],
    } as unknown as PenNode;

    stripRingFrameFills(ring);

    expect((ring as unknown as { fill?: unknown[] }).fill).toEqual([]);
    expect((ring as unknown as { effects?: unknown[] }).effects).toBeUndefined();
    expect((ring as unknown as { stroke?: unknown }).stroke).toBeTruthy();
  });

  it('does not touch ordinary filled circles without a stroke', () => {
    const avatar = {
      id: 'avatar-circle',
      type: 'frame',
      name: 'Avatar Circle',
      width: 48,
      height: 48,
      cornerRadius: 24,
      fill: [{ type: 'solid', color: '#00D09C' }],
      children: [],
    } as unknown as PenNode;

    stripRingFrameFills(avatar);

    expect((avatar as unknown as { fill?: unknown[] }).fill).toEqual([
      { type: 'solid', color: '#00D09C' },
    ]);
  });
});

// ---------------------------------------------------------------------------
// rewriteLlmAntiPatterns (orchestrator)
// ---------------------------------------------------------------------------

describe('rewriteLlmAntiPatterns', () => {
  it('runs both rewriters against the same tree in one pass', () => {
    const page: PenNode = {
      id: 'page',
      type: 'frame',
      layout: 'vertical',
      children: [
        // Broken ring composition
        {
          id: 'rings',
          type: 'frame',
          width: 100,
          height: 100,
          layout: 'none',
          children: [
            strokeEllipse('r1', 100, 100, '#00D09C'),
            strokeEllipse('r2', 60, 60, '#FF8A65'),
          ],
        } as unknown as PenNode,
        // Broken bar chart
        {
          id: 'chart',
          type: 'frame',
          layout: 'horizontal',
          children: [
            bar('mon', 40, '#000'),
            text('monl', 'Mon'),
            bar('tue', 40, '#000'),
            text('tuel', 'Tue'),
            bar('wed', 40, '#000'),
            text('wedl', 'Wed'),
          ],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    rewriteLlmAntiPatterns(page);

    const children = (page as unknown as { children: PenNode[] }).children;
    // Ring rewritten
    const rings = children[0] as PenNode & { layout?: string; children: PenNode[] };
    expect(rings.layout).toBe('horizontal');
    expect(rings.children).toHaveLength(1);
    // Chart rewritten
    const chart = children[1] as PenNode & { children: PenNode[] };
    expect(chart.children).toHaveLength(3);
    expect((chart.children[0] as PenNode & { layout?: string }).layout).toBe('vertical');
  });
});
