import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { isOverlayNode } from '../node-helpers';

describe('isOverlayNode', () => {
  it('returns true for overlay role', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'overlay' };
    expect(isOverlayNode(node)).toBe(true);
  });

  it('returns false for badge role (inline component, not overlay)', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'badge' };
    expect(isOverlayNode(node)).toBe(false);
  });

  it('returns false for pill role (inline component, not overlay)', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'pill' };
    expect(isOverlayNode(node)).toBe(false);
  });

  it('returns false for tag role (inline component, not overlay)', () => {
    const node: PenNode = { id: '1', type: 'rectangle', role: 'tag' };
    expect(isOverlayNode(node)).toBe(false);
  });

  it('returns false for name containing "badge" without explicit role', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Notification Badge' };
    expect(isOverlayNode(node)).toBe(false);
  });

  it('returns false for name containing "overlay" without explicit role', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Image Overlay' };
    expect(isOverlayNode(node)).toBe(false);
  });

  it('returns false for regular nodes', () => {
    const node: PenNode = { id: '1', type: 'rectangle', name: 'Button' };
    expect(isOverlayNode(node)).toBe(false);
  });
});
