import { describe, it, expect, vi, beforeEach } from 'vitest';
import { HistoryManager } from '../core/history-manager';
import type { PenDocument } from '@zseven-w/pen-types';

function makeDoc(name: string): PenDocument {
  return { version: '1.0.0', name, children: [] };
}

describe('HistoryManager', () => {
  let hm: HistoryManager;
  let onChange: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onChange = vi.fn();
    hm = new HistoryManager({ maxStates: 5, onChange });
  });

  it('should start with empty stacks', () => {
    expect(hm.canUndo).toBe(false);
    expect(hm.canRedo).toBe(false);
  });

  it('push + undo should restore previous document', () => {
    const doc1 = makeDoc('v1');
    const doc2 = makeDoc('v2');
    hm.push(doc1);
    const restored = hm.undo(doc2);
    expect(restored).not.toBeNull();
    expect(restored!.name).toBe('v1');
    expect(hm.canUndo).toBe(false);
    expect(hm.canRedo).toBe(true);
  });

  it('undo + redo should restore the document', () => {
    const doc1 = makeDoc('v1');
    const doc2 = makeDoc('v2');
    hm.push(doc1);
    hm.undo(doc2);
    const restored = hm.redo(doc1);
    expect(restored).not.toBeNull();
    expect(restored!.name).toBe('v2');
  });

  it('push should clear redo stack', () => {
    const doc1 = makeDoc('v1');
    const doc2 = makeDoc('v2');
    const doc3 = makeDoc('v3');
    hm.push(doc1);
    hm.undo(doc2);
    expect(hm.canRedo).toBe(true);
    hm.push(doc3);
    expect(hm.canRedo).toBe(false);
  });

  it('should respect maxStates', () => {
    for (let i = 0; i < 10; i++) {
      // Force past debounce by resetting internal timer
      (hm as any).lastPushTime = 0;
      hm.push(makeDoc(`v${i}`));
    }
    let undoCount = 0;
    let current = makeDoc('latest');
    while (hm.canUndo) {
      current = hm.undo(current)!;
      undoCount++;
    }
    // maxStates is 5, so at most 5 undo steps
    expect(undoCount).toBeLessThanOrEqual(5);
  });

  it('should debounce rapid pushes', () => {
    const doc1 = makeDoc('v1');
    const doc2 = makeDoc('v2');
    const doc3 = makeDoc('v3');
    hm.push(doc1);
    // Second push within debounce window should be skipped
    hm.push(doc2);
    hm.push(doc3);
    // Only one undo step (doc1)
    const restored = hm.undo(makeDoc('current'));
    expect(restored!.name).toBe('v1');
    expect(hm.canUndo).toBe(false);
  });

  it('batch should merge all pushes into one undo step', () => {
    const baseDoc = makeDoc('base');
    hm.startBatch(baseDoc);
    hm.push(makeDoc('during1'));
    hm.push(makeDoc('during2'));
    hm.endBatch(makeDoc('after'));
    expect(hm.canUndo).toBe(true);
    const restored = hm.undo(makeDoc('after'));
    expect(restored!.name).toBe('base');
    expect(hm.canUndo).toBe(false);
  });

  it('nested batches should only commit on outermost end', () => {
    const baseDoc = makeDoc('base');
    hm.startBatch(baseDoc);
    hm.startBatch(baseDoc); // nested
    hm.push(makeDoc('inner'));
    hm.endBatch(); // close inner (no-op)
    expect(hm.canUndo).toBe(false); // still in outer batch
    hm.endBatch(makeDoc('final'));
    expect(hm.canUndo).toBe(true);
  });

  it('clear should reset both stacks', () => {
    hm.push(makeDoc('v1'));
    (hm as any).lastPushTime = 0;
    hm.push(makeDoc('v2'));
    hm.clear();
    expect(hm.canUndo).toBe(false);
    expect(hm.canRedo).toBe(false);
  });

  it('should fire onChange callback', () => {
    hm.push(makeDoc('v1'));
    expect(onChange).toHaveBeenCalledWith({ canUndo: true, canRedo: false });
  });
});
