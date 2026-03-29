import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDocumentStore } from '@/stores/document-store';
import { subscribeToActivePageChildren } from '../canvas-document-sync';

describe('subscribeToActivePageChildren', () => {
  beforeEach(() => {
    useDocumentStore.getState().newDocument();
  });

  it('should call onSync when active page children reference changes', () => {
    const onSync = vi.fn();
    const unsub = subscribeToActivePageChildren(onSync);

    const testNode = {
      id: 'test-sync-1',
      type: 'rectangle' as const,
      name: 'Test Rect',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    };
    useDocumentStore.getState().addNode(null, testNode as any);
    expect(onSync).toHaveBeenCalledTimes(1);

    unsub();
  });

  it('should NOT call onSync when non-children state changes', () => {
    const onSync = vi.fn();
    const unsub = subscribeToActivePageChildren(onSync);

    useDocumentStore.setState({ fileName: 'test.pen' });
    expect(onSync).not.toHaveBeenCalled();

    useDocumentStore.setState({ isDirty: true });
    expect(onSync).not.toHaveBeenCalled();

    unsub();
  });

  it('should fire again on second mutation', () => {
    const onSync = vi.fn();
    const unsub = subscribeToActivePageChildren(onSync);

    const node1 = {
      id: 'n1',
      type: 'rectangle' as const,
      name: 'N1',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    };
    const node2 = {
      id: 'n2',
      type: 'rectangle' as const,
      name: 'N2',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    };
    useDocumentStore.getState().addNode(null, node1 as any);
    useDocumentStore.getState().addNode(null, node2 as any);
    expect(onSync).toHaveBeenCalledTimes(2);

    unsub();
  });

  it('should stop firing after unsubscribe', () => {
    const onSync = vi.fn();
    const unsub = subscribeToActivePageChildren(onSync);
    unsub();

    const node = {
      id: 'n3',
      type: 'rectangle' as const,
      name: 'N3',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    };
    useDocumentStore.getState().addNode(null, node as any);
    expect(onSync).not.toHaveBeenCalled();
  });
});
