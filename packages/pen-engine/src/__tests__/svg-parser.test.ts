import { describe, it, expect } from 'vitest';
import { parseSvgToNodes } from '../core/svg-parser';

describe('parseSvgToNodes (isomorphic)', () => {
  it('should parse a simple rectangle SVG', () => {
    const svg = `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
      <rect x="10" y="10" width="80" height="80" fill="#FF0000" />
    </svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBeGreaterThan(0);
    const rect = nodes[0];
    expect(rect.type).toBe('rectangle');
  });

  it('should parse a circle as ellipse', () => {
    const svg = `<svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="#00FF00" /></svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBeGreaterThan(0);
    expect(nodes[0].type).toBe('ellipse');
  });

  it('should parse a path element', () => {
    const svg = `<svg viewBox="0 0 100 100"><path d="M10 10 L90 10 L90 90 Z" fill="#0000FF" /></svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBeGreaterThan(0);
    expect(nodes[0].type).toBe('path');
  });

  it('should wrap multiple elements in a frame', () => {
    const svg = `<svg viewBox="0 0 200 200">
      <rect x="0" y="0" width="100" height="100" fill="#F00" />
      <rect x="100" y="100" width="100" height="100" fill="#00F" />
    </svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBe(1);
    expect(nodes[0].type).toBe('frame');
    expect((nodes[0] as any).children.length).toBe(2);
  });

  it('should skip defs/style/metadata tags', () => {
    const svg = `<svg viewBox="0 0 100 100">
      <defs><linearGradient id="g"/></defs>
      <style>.x{}</style>
      <rect x="0" y="0" width="100" height="100" fill="#F00" />
    </svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBe(1);
    expect(nodes[0].type).toBe('rectangle');
  });

  it('should return empty for invalid SVG', () => {
    expect(parseSvgToNodes('')).toEqual([]);
    expect(parseSvgToNodes('<div></div>')).toEqual([]);
  });

  it('should scale output to maxDim', () => {
    const svg = `<svg viewBox="0 0 1000 1000"><rect x="0" y="0" width="1000" height="1000" fill="#F00" /></svg>`;
    const nodes = parseSvgToNodes(svg, 200);
    const node = nodes[0] as any;
    // Dimensions should be scaled down
    expect(node.width ?? node.children?.[0]?.width).toBeLessThanOrEqual(200);
  });

  it('should handle g elements with style inheritance', () => {
    const svg = `<svg viewBox="0 0 100 100">
      <g fill="#FF0000">
        <rect x="0" y="0" width="50" height="50" />
        <rect x="50" y="50" width="50" height="50" />
      </g>
    </svg>`;
    const nodes = parseSvgToNodes(svg);
    expect(nodes.length).toBeGreaterThan(0);
  });
});
