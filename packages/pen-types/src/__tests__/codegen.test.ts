import { describe, it, expect } from 'vitest';
import { FRAMEWORKS } from '../codegen';
import type { NodeSnapshot, ResolvedDepContract } from '../codegen';

describe('codegen types', () => {
  it('FRAMEWORKS contains all 8 frameworks', () => {
    expect(FRAMEWORKS).toHaveLength(8);
    expect(FRAMEWORKS).toContain('react');
    expect(FRAMEWORKS).toContain('flutter');
  });

  it('NodeSnapshot allows truncated children', () => {
    const snapshot: NodeSnapshot = {
      id: 'n1',
      type: 'frame',
      name: 'Test',
      children: '...',
    } as NodeSnapshot;
    expect(snapshot.children).toBe('...');
  });

  it('NodeSnapshot allows nested snapshots', () => {
    const snapshot: NodeSnapshot = {
      id: 'n1',
      type: 'frame',
      name: 'Parent',
      children: [{ id: 'n2', type: 'rectangle', name: 'Child' } as NodeSnapshot],
    } as NodeSnapshot;
    expect(Array.isArray(snapshot.children)).toBe(true);
  });

  it('ResolvedDepContract allows null', () => {
    const resolved: ResolvedDepContract = null;
    expect(resolved).toBeNull();
  });
});
