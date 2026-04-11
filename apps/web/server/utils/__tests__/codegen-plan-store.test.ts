import { describe, it, expect } from 'vitest';
import {
  createPlan,
  getPlan,
  submitChunkResult,
  assemblePlan,
  cleanPlan,
} from '../codegen-plan-store';
import type { CodePlanFromAI, ChunkResult, PenNode } from '@zseven-w/pen-types';

const mockNodes: PenNode[] = [
  { id: 'n1', type: 'frame', name: 'Hero', x: 0, y: 0, width: 800, height: 600 } as PenNode,
  { id: 'n2', type: 'rectangle', name: 'Card', x: 0, y: 0, width: 200, height: 150 } as PenNode,
];

const validPlan: CodePlanFromAI = {
  chunks: [
    {
      id: 'c1',
      name: 'Hero',
      nodeIds: ['n1'],
      role: 'section',
      suggestedComponentName: 'Hero',
      dependencies: [],
      exposedSlots: [],
    },
    {
      id: 'c2',
      name: 'Card',
      nodeIds: ['n2'],
      role: 'component',
      suggestedComponentName: 'Card',
      dependencies: ['c1'],
    },
  ],
  sharedStyles: [],
  rootLayout: { direction: 'vertical', gap: 16, responsive: true },
};

describe('codegen-plan-store', () => {
  it('createPlan returns planId and sorted executionPlan', () => {
    const result = createPlan(validPlan, mockNodes);
    expect(result.planId).toBeTruthy();
    expect(result.executionPlan).toHaveLength(2);
    expect(result.executionPlan[0].id).toBe('c1');
    expect(result.executionPlan[1].id).toBe('c2');
    expect(result.warnings).toHaveLength(0);
    cleanPlan(result.planId);
  });

  it('createPlan rejects duplicate chunkIds', () => {
    const badPlan = {
      ...validPlan,
      chunks: [{ ...validPlan.chunks[0] }, { ...validPlan.chunks[1], id: 'c1' }],
    };
    expect(() => createPlan(badPlan, mockNodes)).toThrow('Duplicate chunkId: c1');
  });

  it('createPlan rejects empty nodeIds', () => {
    const badPlan = {
      ...validPlan,
      chunks: [{ ...validPlan.chunks[0], nodeIds: [] }],
    };
    expect(() => createPlan(badPlan, mockNodes)).toThrow('has no nodeIds');
  });

  it('createPlan rejects unknown dependency', () => {
    const badPlan = {
      ...validPlan,
      chunks: [{ ...validPlan.chunks[0], dependencies: ['unknown'] }],
    };
    expect(() => createPlan(badPlan, mockNodes)).toThrow('depends on unknown chunk');
  });

  it('createPlan rejects circular dependency', () => {
    const badPlan: CodePlanFromAI = {
      ...validPlan,
      chunks: [
        { ...validPlan.chunks[0], dependencies: ['c2'] },
        { ...validPlan.chunks[1], dependencies: ['c1'] },
      ],
    };
    expect(() => createPlan(badPlan, mockNodes)).toThrow('Circular dependency');
  });

  it('createPlan rejects missing nodeId in document', () => {
    const badPlan: CodePlanFromAI = {
      ...validPlan,
      chunks: [{ ...validPlan.chunks[0], nodeIds: ['missing'] }],
    };
    expect(() => createPlan(badPlan, [])).toThrow('not found in document');
  });

  it('createPlan warns on shared nodeIds', () => {
    const sharedPlan: CodePlanFromAI = {
      ...validPlan,
      chunks: [
        { ...validPlan.chunks[0], nodeIds: ['n1'] },
        { ...validPlan.chunks[1], nodeIds: ['n1'], dependencies: [] },
      ],
    };
    const result = createPlan(sharedPlan, mockNodes);
    expect(result.warnings.length).toBeGreaterThan(0);
    expect(result.warnings[0]).toContain('claimed by chunks');
    cleanPlan(result.planId);
  });

  it('submitChunkResult stores result and returns next chunk', () => {
    const { planId } = createPlan(validPlan, mockNodes);
    const chunkResult: ChunkResult = {
      chunkId: 'c1',
      code: 'function Hero() {}',
      contract: {
        chunkId: 'c1',
        componentName: 'Hero',
        exportedProps: [],
        slots: [],
        cssClasses: [],
        cssVariables: [],
        imports: [],
      },
    };
    const submitResult = submitChunkResult(planId, chunkResult);
    expect(submitResult.validation.valid).toBe(true);
    expect(submitResult.nextChunk).toBeDefined();
    expect(submitResult.nextChunk!.id).toBe('c2');
    cleanPlan(planId);
  });

  it('assemblePlan returns all results and clears cache', () => {
    const { planId } = createPlan(validPlan, mockNodes);
    const cr1: ChunkResult = {
      chunkId: 'c1',
      code: 'function Hero() {}',
      contract: {
        chunkId: 'c1',
        componentName: 'Hero',
        exportedProps: [],
        slots: [],
        cssClasses: [],
        cssVariables: [],
        imports: [],
      },
    };
    const cr2: ChunkResult = {
      chunkId: 'c2',
      code: 'function Card() {}',
      contract: {
        chunkId: 'c2',
        componentName: 'Card',
        exportedProps: [],
        slots: [],
        cssClasses: [],
        cssVariables: [],
        imports: [],
      },
    };
    submitChunkResult(planId, cr1);
    submitChunkResult(planId, cr2);

    const assembled = assemblePlan(planId, 'react');
    expect(assembled.chunks).toHaveLength(2);
    expect(assembled.degraded).toBe(false);

    // Terminal operation — plan should be cleared
    expect(getPlan(planId)).toBeUndefined();
  });

  it('cleanPlan deletes existing plan', () => {
    const { planId } = createPlan(validPlan, mockNodes);
    expect(cleanPlan(planId)).toEqual({ ok: true, deleted: true });
    expect(getPlan(planId)).toBeUndefined();
  });

  it('cleanPlan returns deleted=false for unknown plan', () => {
    expect(cleanPlan('nonexistent')).toEqual({ ok: true, deleted: false });
  });
});
