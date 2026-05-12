import { describe, it, expect } from 'vitest';
import { FRAMEWORKS } from '../codegen';
import type { ChunkContract, NodeSnapshot, ResolvedDepContract } from '../codegen';

describe('codegen types', () => {
  it('FRAMEWORKS contains all 9 frameworks', () => {
    expect(FRAMEWORKS).toHaveLength(9);
    expect(FRAMEWORKS).toContain('react');
    expect(FRAMEWORKS).toContain('flutter');
    expect(FRAMEWORKS).toContain('uniapp');
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

  it('ChunkContract can include generated output file paths', () => {
    const contract: ChunkContract = {
      chunkId: 'chunk-1',
      componentName: 'HomePage',
      exportedProps: [],
      slots: [],
      cssClasses: [],
      cssVariables: [],
      imports: [],
      outputFiles: ['pages/index/index.vue'],
    };

    expect(contract.outputFiles).toEqual(['pages/index/index.vue']);
  });
});
