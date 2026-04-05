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
