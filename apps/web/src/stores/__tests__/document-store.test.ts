import { describe, expect, it } from 'vitest';

import { useCanvasStore } from '@/stores/canvas-store';
import { createEmptyDocument, useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import type { PenNode } from '@/types/pen';
import type { PenDocument } from '@/types/pen';

function resetStores(document: PenDocument) {
  useCanvasStore.setState({
    activePageId: 'page-1',
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
    },
  });

  useDocumentStore.setState({
    document,
    isDirty: false,
    fileHandle: null,
    fileName: null,
    filePath: null,
    saveDialogOpen: false,
  } as any);
}

describe('document-store moveNode', () => {
  it('replaces imported content without dropping cloud file metadata', () => {
    useDocumentStore.getState().loadCloudDocument({
      id: 'cloud-file-1',
      name: 'Untitled',
      revision: 7,
      thumbnailPath: null,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      document: createEmptyDocument(),
    });

    const importedDoc: PenDocument = {
      version: '1.0.0',
      name: 'Imported Figma',
      pages: [
        {
          id: 'figma-page-1',
          name: 'Figma Page',
          children: [
            {
              id: 'figma-frame-1',
              type: 'frame',
              name: 'Imported frame',
              width: 320,
              height: 240,
              children: [],
            },
          ],
        },
      ],
      children: [],
    };

    useDocumentStore.getState().replaceDocumentContent(importedDoc, 'imported-figma.op');

    const state = useDocumentStore.getState();
    expect(state.cloudFileId).toBe('cloud-file-1');
    expect(state.cloudRevision).toBe(7);
    expect(state.cloudSaveState).toBe('idle');
    expect(state.fileName).toBe('imported-figma.op');
    expect(state.fileHandle).toBeNull();
    expect(state.filePath).toBeNull();
    expect(state.isDirty).toBe(true);
    expect(state.document.name).toBe('Imported Figma');
    expect(useCanvasStore.getState().activePageId).toBe('figma-page-1');
  });

  it('addNode should push history before mutating document', () => {
    useDocumentStore.getState().newDocument();
    useHistoryStore.getState().clear();
    const docBefore = useDocumentStore.getState().document;
    const testNode: PenNode = {
      id: 'test-1',
      type: 'rectangle',
      name: 'Test Rect',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    const docAfter = useDocumentStore.getState().document;
    expect(docAfter).not.toBe(docBefore);
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().isDirty).toBe(true);
  });

  it('updateNode should push history and mark dirty', () => {
    useDocumentStore.getState().newDocument();
    const testNode = {
      id: 'test-2',
      type: 'rectangle',
      name: 'Test',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    useHistoryStore.getState().clear();
    useDocumentStore.getState().updateNode('test-2', { x: 100 });
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().isDirty).toBe(true);
    const node = useDocumentStore.getState().getNodeById('test-2');
    expect(node?.x).toBe(100);
  });

  it('removeNode should push history', () => {
    useDocumentStore.getState().newDocument();
    const testNode = {
      id: 'test-3',
      type: 'rectangle',
      name: 'Test',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    } as PenNode;
    useDocumentStore.getState().addNode(null, testNode);
    useHistoryStore.getState().clear();
    useDocumentStore.getState().removeNode('test-3');
    expect(useHistoryStore.getState().canUndo()).toBe(true);
    expect(useDocumentStore.getState().getNodeById('test-3')).toBeUndefined();
  });

  it('detaches moved nodes from stale references so reparenting cannot corrupt local coordinates', () => {
    const document: PenDocument = {
      version: '1.0.0',
      pages: [
        {
          id: 'page-1',
          name: 'Page 1',
          children: [
            {
              id: 'panel-1',
              type: 'frame',
              name: 'panel-1',
              x: 0,
              y: 0,
              width: 400,
              height: 200,
              children: [
                {
                  id: 'rect-1',
                  type: 'rectangle',
                  name: 'rect-1',
                  x: 100,
                  y: 100,
                  width: 80,
                  height: 60,
                },
              ],
            },
            {
              id: 'panel-10',
              type: 'frame',
              name: 'panel-10',
              x: 0,
              y: 2700,
              width: 400,
              height: 200,
              children: [],
            },
          ],
        },
      ],
      children: [],
    };

    resetStores(document);

    const staleNodeRef = useDocumentStore.getState().getNodeById('rect-1');
    expect(staleNodeRef?.x).toBe(100);
    expect(staleNodeRef?.y).toBe(100);

    useDocumentStore.getState().moveNode('rect-1', 'panel-10', 0);

    const movedNode = useDocumentStore.getState().getNodeById('rect-1');
    expect(movedNode?.x).toBe(100);
    expect(movedNode?.y).toBe(100);
    expect(useDocumentStore.getState().getParentOf('rect-1')?.id).toBe('panel-10');

    if (!staleNodeRef) {
      throw new Error('Expected stale node reference to exist');
    }
    staleNodeRef.x = 9999;
    staleNodeRef.y = 9999;

    const storedNodeAfterStaleMutation = useDocumentStore.getState().getNodeById('rect-1');
    expect(storedNodeAfterStaleMutation?.x).toBe(100);
    expect(storedNodeAfterStaleMutation?.y).toBe(100);
  });
});
