import { describe, it, expect, beforeEach } from 'vitest';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import type { PenNode } from '@/types/pen';

describe('document-store mutations', () => {
  beforeEach(() => {
    useDocumentStore.getState().newDocument();
    useHistoryStore.getState().clear();
  });

  it('addNode should push history before mutating document', () => {
    const docBefore = useDocumentStore.getState().document;
    const testNode: PenNode = {
      id: 'test-1', type: 'rectangle', name: 'Test Rect',
      x: 0, y: 0, width: 100, height: 100,
    } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    const docAfter = useDocumentStore.getState().document;
    expect(docAfter).not.toBe(docBefore);
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().isDirty).toBe(true);
  });

  it('updateNode should push history and mark dirty', () => {
    const testNode = { id: 'test-2', type: 'rectangle', name: 'Test', x: 0, y: 0, width: 50, height: 50 } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    useHistoryStore.getState().clear();
    useDocumentStore.getState().updateNode('test-2', { x: 100 });
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().isDirty).toBe(true);
    const node = useDocumentStore.getState().getNodeById('test-2');
    expect(node?.x).toBe(100);
  });

  it('removeNode should push history', () => {
    const testNode = { id: 'test-3', type: 'rectangle', name: 'Test', x: 0, y: 0, width: 50, height: 50 } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    useHistoryStore.getState().clear();
    useDocumentStore.getState().removeNode('test-3');
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().getNodeById('test-3')).toBeUndefined();
  });
});
