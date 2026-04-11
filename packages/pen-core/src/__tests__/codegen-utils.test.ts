import { describe, it, expect } from 'vitest';
import { sanitizeName, nodeTreeToSummary } from '../index';
import type { PenNode } from '@zseven-w/pen-types';

describe('sanitizeName', () => {
  it('converts kebab-case to PascalCase', () => {
    expect(sanitizeName('hero-section')).toBe('HeroSection');
  });

  it('strips non-alphanumeric characters', () => {
    expect(sanitizeName('Card #1 (main)')).toBe('Card1Main');
  });

  it('handles empty string', () => {
    expect(sanitizeName('')).toBe('');
  });
});

describe('nodeTreeToSummary', () => {
  it('renders single node', () => {
    const nodes: PenNode[] = [
      { id: 'n1', type: 'frame', name: 'Hero', width: 800, height: 600 } as PenNode,
    ];
    const result = nodeTreeToSummary(nodes);
    expect(result).toContain('[n1]');
    expect(result).toContain('frame');
    expect(result).toContain('"Hero"');
    expect(result).toContain('800x600');
  });

  it('renders nested children with indentation', () => {
    const nodes: PenNode[] = [
      {
        id: 'p',
        type: 'frame',
        name: 'Parent',
        width: 100,
        height: 100,
        children: [{ id: 'c', type: 'rectangle', name: 'Child', width: 50, height: 50 } as PenNode],
      } as unknown as PenNode,
    ];
    const result = nodeTreeToSummary(nodes);
    expect(result).toContain('[1 children]');
    expect(result).toContain('  - [c]');
  });

  it('returns empty string for empty array', () => {
    expect(nodeTreeToSummary([])).toBe('');
  });
});
