import { describe, it, expect } from 'vitest';
import { hexLuminance, hasFill, resolveTreePostPass } from '../role-resolver';
import type { PenNode } from '@zseven-w/pen-types';

describe('hexLuminance', () => {
  it('returns 0 for black', () => {
    expect(hexLuminance('#000000')).toBeCloseTo(0, 2);
  });

  it('returns 1 for white', () => {
    expect(hexLuminance('#FFFFFF')).toBeCloseTo(1, 2);
  });

  it('returns ~0.5 for mid-gray', () => {
    const lum = hexLuminance('#808080');
    expect(lum).toBeGreaterThan(0.2);
    expect(lum).toBeLessThan(0.6);
  });

  it('handles lowercase hex', () => {
    expect(hexLuminance('#ffffff')).toBeCloseTo(1, 2);
  });

  it('handles 8-digit hex (with alpha)', () => {
    expect(hexLuminance('#000000FF')).toBeCloseTo(0, 2);
  });

  it('returns < 0.5 for dark blue (#2563EB)', () => {
    expect(hexLuminance('#2563EB')).toBeLessThan(0.5);
  });

  it('returns > 0.5 for light gray (#F8FAFC)', () => {
    expect(hexLuminance('#F8FAFC')).toBeGreaterThan(0.5);
  });
});

describe('hasFill', () => {
  it('returns false for node without fill', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100, fill: [] } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns true for node with solid fill', () => {
    const node = {
      id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });
});

describe('resolveTreePostPass — button foreground contrast', () => {
  it('sets white text on dark button', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 120, height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'txt', type: 'text', name: 'Label', x: 0, y: 0, width: 80, height: 20, content: 'Sign In' } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets dark text on light button', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 120, height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#DBEAFE' }],
      children: [
        { id: 'txt', type: 'text', name: 'Label', x: 0, y: 0, width: 80, height: 20, content: 'Sign In' } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#0F172A' }]);
  });

  it('does not overwrite explicit text fill', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 120, height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'txt', type: 'text', name: 'Label', x: 0, y: 0, width: 80, height: 20, content: 'Sign In',
          fill: [{ type: 'solid', color: '#FDE047' }] } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FDE047' }]);
  });

  it('sets fill on icon_font child in dark button', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 44, height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#1E293B' }],
      children: [
        { id: 'ico', type: 'icon_font', name: 'Icon', x: 0, y: 0, width: 24, height: 24 } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const ico = (root as any).children[0].children[0];
    expect(ico.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets stroke.fill on stroke-style path in dark button', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 44, height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'p', type: 'path', name: 'Arrow', x: 0, y: 0, width: 24, height: 24,
          stroke: { thickness: 2 } } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.stroke.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets fill on unstyled path (no stroke, no fill) in dark button', () => {
    const button: PenNode = {
      id: 'btn', type: 'frame', name: 'Button', x: 0, y: 0, width: 44, height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'p', type: 'path', name: 'Arrow', x: 0, y: 0, width: 24, height: 24 } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 375, height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });
});

describe('resolveTreePostPass — section background alternation', () => {
  it('alternates fills on 3+ consecutive unfilled sections', () => {
    const children = [
      { id: 's0', type: 'frame' as const, name: 'Hero', x: 0, y: 0, width: 1200, height: 400, role: 'hero', layout: 'vertical' as const, children: [] },
      { id: 's1', type: 'frame' as const, name: 'Features', x: 0, y: 0, width: 1200, height: 400, role: 'section', layout: 'vertical' as const, children: [] },
      { id: 's2', type: 'frame' as const, name: 'CTA', x: 0, y: 0, width: 1200, height: 400, role: 'cta-section', layout: 'vertical' as const, children: [] },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 2400,
      layout: 'vertical', children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((children[1] as any).fill).toEqual([{ type: 'solid', color: '#F8FAFC' }]);
    expect((children[2] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('only alternates within contiguous runs — non-section children break the run', () => {
    const children = [
      { id: 's0', type: 'frame' as const, name: 'Hero', x: 0, y: 0, width: 1200, height: 400, role: 'hero', layout: 'vertical' as const, children: [] },
      { id: 's1', type: 'frame' as const, name: 'Features', x: 0, y: 0, width: 1200, height: 400, role: 'section', layout: 'vertical' as const, children: [] },
      { id: 'card', type: 'frame' as const, name: 'Card', x: 0, y: 0, width: 300, height: 200, role: 'card', children: [] },
      { id: 's2', type: 'frame' as const, name: 'Footer', x: 0, y: 0, width: 1200, height: 400, role: 'footer', layout: 'vertical' as const, children: [] },
      { id: 's3', type: 'frame' as const, name: 'Section2', x: 0, y: 0, width: 1200, height: 400, role: 'section', layout: 'vertical' as const, children: [] },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 3000,
      layout: 'vertical', children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
    expect((children[3] as any).fill).toBeUndefined();
    expect((children[4] as any).fill).toBeUndefined();
  });

  it('skips sections with existing fills', () => {
    const children = [
      { id: 's0', type: 'frame' as const, name: 'Hero', x: 0, y: 0, width: 1200, height: 400, role: 'hero', layout: 'vertical' as const, fill: [{ type: 'solid', color: '#1E293B' }], children: [] },
      { id: 's1', type: 'frame' as const, name: 'Features', x: 0, y: 0, width: 1200, height: 400, role: 'section', layout: 'vertical' as const, children: [] },
      { id: 's2', type: 'frame' as const, name: 'Footer', x: 0, y: 0, width: 1200, height: 400, role: 'footer', layout: 'vertical' as const, children: [] },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 2400,
      layout: 'vertical', children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#1E293B' }]);
    expect((children[1] as any).fill).toBeUndefined();
  });

  it('does nothing with fewer than 3 consecutive sections', () => {
    const children = [
      { id: 's0', type: 'frame' as const, name: 'Hero', x: 0, y: 0, width: 1200, height: 400, role: 'hero', layout: 'vertical' as const, children: [] },
      { id: 's1', type: 'frame' as const, name: 'Footer', x: 0, y: 0, width: 1200, height: 400, role: 'footer', layout: 'vertical' as const, children: [] },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 1200,
      layout: 'vertical', children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
  });
});

describe('resolveTreePostPass — orphan container contrast', () => {
  it('adds fill + shadow to untagged rounded frame when parent has no fill', () => {
    const card: PenNode = {
      id: 'card', type: 'frame', name: 'Card', x: 0, y: 0, width: 300, height: 200,
      cornerRadius: 12,
      children: [
        { id: 'txt', type: 'text', name: 'Title', x: 0, y: 0, width: 200, height: 20, content: 'Hello' } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((card as any).effects).toHaveLength(2);
    expect((card as any).effects[0].type).toBe('shadow');
  });

  it('does not apply to structural roles like section', () => {
    const section: PenNode = {
      id: 'sec', type: 'frame', name: 'Section', x: 0, y: 0, width: 1200, height: 400,
      role: 'section', cornerRadius: 12,
      children: [
        { id: 'txt', type: 'text', name: 'Title', x: 0, y: 0, width: 200, height: 20, content: 'Hello' } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 800,
      children: [section],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((section as any).fill).toBeUndefined();
  });

  it('does not apply when parent has fill', () => {
    const card: PenNode = {
      id: 'card', type: 'frame', name: 'Card', x: 0, y: 0, width: 300, height: 200,
      cornerRadius: 12,
      children: [
        { id: 'txt', type: 'text', name: 'Title', x: 0, y: 0, width: 200, height: 20, content: 'Hello' } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 800,
      fill: [{ type: 'solid', color: '#F8FAFC' }],
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toBeUndefined();
  });

  it('does not apply to empty frames', () => {
    const empty: PenNode = {
      id: 'e', type: 'frame', name: 'Empty', x: 0, y: 0, width: 300, height: 200,
      cornerRadius: 12, children: [],
    } as PenNode;
    const root: PenNode = {
      id: 'root', type: 'frame', name: 'Root', x: 0, y: 0, width: 1200, height: 800,
      children: [empty],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((empty as any).fill).toBeUndefined();
  });
});
