import { describe, expect, it } from 'bun:test'

import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { PenDocument } from '@/types/pen'

function resetStores(document: PenDocument) {
  useCanvasStore.setState({
    activePageId: 'page-1',
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
    },
  })

  useDocumentStore.setState({
    document,
    isDirty: false,
    fileHandle: null,
    fileName: null,
    filePath: null,
    saveDialogOpen: false,
  } as any)
}

describe('document-store moveNode', () => {
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
    }

    resetStores(document)

    const staleNodeRef = useDocumentStore.getState().getNodeById('rect-1')
    expect(staleNodeRef?.x).toBe(100)
    expect(staleNodeRef?.y).toBe(100)

    useDocumentStore.getState().moveNode('rect-1', 'panel-10', 0)

    const movedNode = useDocumentStore.getState().getNodeById('rect-1')
    expect(movedNode?.x).toBe(100)
    expect(movedNode?.y).toBe(100)
    expect(useDocumentStore.getState().getParentOf('rect-1')?.id).toBe('panel-10')

    if (!staleNodeRef) {
      throw new Error('Expected stale node reference to exist')
    }
    staleNodeRef.x = 9999
    staleNodeRef.y = 9999

    const storedNodeAfterStaleMutation = useDocumentStore.getState().getNodeById('rect-1')
    expect(storedNodeAfterStaleMutation?.x).toBe(100)
    expect(storedNodeAfterStaleMutation?.y).toBe(100)
  })
})
