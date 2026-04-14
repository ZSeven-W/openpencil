import { describe, expect, it } from 'vitest';
import type { PenNode } from '@/types/pen';
import { shouldAutoReparentOnDragOutsideParent } from './drag-reparent-policy';

const node = (type: PenNode['type']): PenNode => ({ id: type, type }) as PenNode;

describe('shouldAutoReparentOnDragOutsideParent', () => {
  it('keeps frame and shape-style nodes parented while dragging', () => {
    expect(shouldAutoReparentOnDragOutsideParent(node('frame'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('group'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('rectangle'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('ellipse'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('line'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('polygon'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('path'))).toBe(false);
    expect(shouldAutoReparentOnDragOutsideParent(node('ref'))).toBe(false);
  });

  it('still allows leaf content to detach with the legacy behavior', () => {
    expect(shouldAutoReparentOnDragOutsideParent(node('text'))).toBe(true);
    expect(shouldAutoReparentOnDragOutsideParent(node('image'))).toBe(true);
    expect(shouldAutoReparentOnDragOutsideParent(node('icon_font'))).toBe(true);
  });

  it('falls back safely when node data is unavailable', () => {
    expect(shouldAutoReparentOnDragOutsideParent(undefined)).toBe(true);
  });
});
