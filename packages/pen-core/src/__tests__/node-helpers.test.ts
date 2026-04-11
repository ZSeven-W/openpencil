import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { isBadgeOverlayNode } from '../node-helpers';

describe('isBadgeOverlayNode', () => {
  it('returns true for badge role', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'badge' };
    expect(isBadgeOverlayNode(node)).toBe(true);
  });

  it('returns true for pill role', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'pill' };
    expect(isBadgeOverlayNode(node)).toBe(true);
  });

  it('returns true for name containing "badge"', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Notification Badge' };
    expect(isBadgeOverlayNode(node)).toBe(true);
  });

  it('returns true for name containing "overlay"', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Image Overlay' };
    expect(isBadgeOverlayNode(node)).toBe(true);
  });

  it('returns false for regular nodes', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Button' };
    expect(isBadgeOverlayNode(node)).toBe(false);
  });
});
