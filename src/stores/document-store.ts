import { create } from 'zustand'
import type { PenDocument, PenNode } from '@/types/pen'

function createEmptyDocument(): PenDocument {
  return {
    version: '1.0.0',
    children: [],
  }
}

function findNodeInTree(
  nodes: PenNode[],
  id: string,
): PenNode | undefined {
  for (const node of nodes) {
    if (node.id === id) return node
    if ('children' in node && node.children) {
      const found = findNodeInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

function findParentInTree(
  nodes: PenNode[],
  id: string,
): PenNode | undefined {
  for (const node of nodes) {
    if ('children' in node && node.children) {
      for (const child of node.children) {
        if (child.id === id) return node
      }
      const found = findParentInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

function removeNodeFromTree(nodes: PenNode[], id: string): PenNode[] {
  return nodes
    .filter((n) => n.id !== id)
    .map((n) => {
      if ('children' in n && n.children) {
        return { ...n, children: removeNodeFromTree(n.children, id) }
      }
      return n
    })
}

function updateNodeInTree(
  nodes: PenNode[],
  id: string,
  updates: Partial<PenNode>,
): PenNode[] {
  return nodes.map((n) => {
    if (n.id === id) {
      return { ...n, ...updates } as PenNode
    }
    if ('children' in n && n.children) {
      return {
        ...n,
        children: updateNodeInTree(n.children, id, updates),
      } as PenNode
    }
    return n
  })
}

function flattenNodes(nodes: PenNode[]): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
    result.push(node)
    if ('children' in node && node.children) {
      result.push(...flattenNodes(node.children))
    }
  }
  return result
}

function insertNodeInTree(
  nodes: PenNode[],
  parentId: string | null,
  node: PenNode,
  index?: number,
): PenNode[] {
  if (parentId === null) {
    const arr = [...nodes]
    if (index !== undefined) {
      arr.splice(index, 0, node)
    } else {
      arr.push(node)
    }
    return arr
  }

  return nodes.map((n) => {
    if (n.id === parentId && 'children' in n) {
      const children = [...(n.children ?? [])]
      if (index !== undefined) {
        children.splice(index, 0, node)
      } else {
        children.push(node)
      }
      return { ...n, children } as PenNode
    }
    if ('children' in n && n.children) {
      return {
        ...n,
        children: insertNodeInTree(n.children, parentId, node, index),
      } as PenNode
    }
    return n
  })
}

interface DocumentStoreState {
  document: PenDocument
  fileName: string | null
  isDirty: boolean

  addNode: (
    parentId: string | null,
    node: PenNode,
    index?: number,
  ) => void
  updateNode: (id: string, updates: Partial<PenNode>) => void
  removeNode: (id: string) => void
  moveNode: (
    id: string,
    newParentId: string | null,
    index: number,
  ) => void
  reorderNode: (id: string, direction: 'up' | 'down') => void
  getNodeById: (id: string) => PenNode | undefined
  getParentOf: (id: string) => PenNode | undefined
  getFlatNodes: () => PenNode[]

  loadDocument: (doc: PenDocument, fileName?: string) => void
  newDocument: () => void
  markClean: () => void
}

export const useDocumentStore = create<DocumentStoreState>(
  (set, get) => ({
    document: createEmptyDocument(),
    fileName: null,
    isDirty: false,

    addNode: (parentId, node, index) =>
      set((s) => ({
        document: {
          ...s.document,
          children: insertNodeInTree(
            s.document.children,
            parentId,
            node,
            index,
          ),
        },
        isDirty: true,
      })),

    updateNode: (id, updates) =>
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(
            s.document.children,
            id,
            updates,
          ),
        },
        isDirty: true,
      })),

    removeNode: (id) =>
      set((s) => ({
        document: {
          ...s.document,
          children: removeNodeFromTree(s.document.children, id),
        },
        isDirty: true,
      })),

    moveNode: (id, newParentId, index) => {
      const state = get()
      const node = findNodeInTree(state.document.children, id)
      if (!node) return
      const withoutNode = removeNodeFromTree(
        state.document.children,
        id,
      )
      const withNode = insertNodeInTree(
        withoutNode,
        newParentId,
        node,
        index,
      )
      set({
        document: { ...state.document, children: withNode },
        isDirty: true,
      })
    },

    reorderNode: (id, direction) => {
      const state = get()
      const parent = findParentInTree(state.document.children, id)
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : state.document.children
      const idx = siblings.findIndex((n) => n.id === id)
      if (idx === -1) return
      const newIdx =
        direction === 'up'
          ? Math.max(0, idx - 1)
          : Math.min(siblings.length - 1, idx + 1)
      if (newIdx === idx) return
      const newSiblings = [...siblings]
      const [removed] = newSiblings.splice(idx, 1)
      newSiblings.splice(newIdx, 0, removed)

      if (parent && 'children' in parent) {
        set((s) => ({
          document: {
            ...s.document,
            children: updateNodeInTree(
              s.document.children,
              parent.id,
              { children: newSiblings } as Partial<PenNode>,
            ),
          },
          isDirty: true,
        }))
      } else {
        set((s) => ({
          document: { ...s.document, children: newSiblings },
          isDirty: true,
        }))
      }
    },

    getNodeById: (id) =>
      findNodeInTree(get().document.children, id),

    getParentOf: (id) =>
      findParentInTree(get().document.children, id),

    getFlatNodes: () => flattenNodes(get().document.children),

    loadDocument: (doc, fileName) =>
      set({ document: doc, fileName: fileName ?? null, isDirty: false }),

    newDocument: () =>
      set({
        document: createEmptyDocument(),
        fileName: null,
        isDirty: false,
      }),

    markClean: () => set({ isDirty: false }),
  }),
)

export { createEmptyDocument }
export { nanoid as generateId } from 'nanoid'
