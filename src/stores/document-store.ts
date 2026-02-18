import { create } from 'zustand'
import { nanoid } from 'nanoid'
import type { PenDocument, PenNode, GroupNode } from '@/types/pen'
import { useHistoryStore } from '@/stores/history-store'

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
  /** Native file handle for save-in-place (File System Access API). */
  fileHandle: FileSystemFileHandle | null
  /** Whether the "save as" dialog is open (fallback for browsers without FS API). */
  saveDialogOpen: boolean

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
  toggleVisibility: (id: string) => void
  toggleLock: (id: string) => void
  duplicateNode: (id: string) => string | null
  groupNodes: (nodeIds: string[]) => string | null
  ungroupNode: (groupId: string) => void
  getNodeById: (id: string) => PenNode | undefined
  getParentOf: (id: string) => PenNode | undefined
  getFlatNodes: () => PenNode[]

  applyHistoryState: (doc: PenDocument) => void
  loadDocument: (
    doc: PenDocument,
    fileName?: string,
    fileHandle?: FileSystemFileHandle | null,
  ) => void
  newDocument: () => void
  markClean: () => void
  setFileHandle: (handle: FileSystemFileHandle | null) => void
  setSaveDialogOpen: (open: boolean) => void
}

export const useDocumentStore = create<DocumentStoreState>(
  (set, get) => ({
    document: createEmptyDocument(),
    fileName: null,
    isDirty: false,
    fileHandle: null,
    saveDialogOpen: false,

    addNode: (parentId, node, index) => {
      useHistoryStore.getState().pushState(get().document)
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
      }))
    },

    updateNode: (id, updates) => {
      useHistoryStore.getState().pushState(get().document)
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
      }))
    },

    removeNode: (id) => {
      useHistoryStore.getState().pushState(get().document)
      set((s) => ({
        document: {
          ...s.document,
          children: removeNodeFromTree(s.document.children, id),
        },
        isDirty: true,
      }))
    },

    moveNode: (id, newParentId, index) => {
      const state = get()
      const node = findNodeInTree(state.document.children, id)
      if (!node) return
      useHistoryStore.getState().pushState(state.document)
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
      useHistoryStore.getState().pushState(state.document)
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

    toggleVisibility: (id) => {
      const node = findNodeInTree(get().document.children, id)
      if (!node) return
      useHistoryStore.getState().pushState(get().document)
      const currentVisible = node.visible !== false
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(s.document.children, id, {
            visible: !currentVisible,
          } as Partial<PenNode>),
        },
        isDirty: true,
      }))
    },

    toggleLock: (id) => {
      const node = findNodeInTree(get().document.children, id)
      if (!node) return
      useHistoryStore.getState().pushState(get().document)
      const currentLocked = node.locked === true
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(s.document.children, id, {
            locked: !currentLocked,
          } as Partial<PenNode>),
        },
        isDirty: true,
      }))
    },

    duplicateNode: (id) => {
      const state = get()
      const node = findNodeInTree(state.document.children, id)
      if (!node) return null

      const cloneWithNewIds = (n: PenNode): PenNode => {
        const cloned = { ...n, id: nanoid() } as PenNode
        if ('children' in cloned && cloned.children) {
          cloned.children = cloned.children.map(cloneWithNewIds)
        }
        return cloned
      }

      const clone = cloneWithNewIds(node)
      clone.name = (clone.name ?? clone.type) + ' copy'
      if (clone.x !== undefined) clone.x += 10
      if (clone.y !== undefined) clone.y += 10

      const parent = findParentInTree(state.document.children, id)
      const parentId = parent ? parent.id : null
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : state.document.children
      const idx = siblings.findIndex((n) => n.id === id)

      useHistoryStore.getState().pushState(state.document)
      set((s) => ({
        document: {
          ...s.document,
          children: insertNodeInTree(
            s.document.children,
            parentId,
            clone,
            idx + 1,
          ),
        },
        isDirty: true,
      }))
      return clone.id
    },

    groupNodes: (nodeIds) => {
      if (nodeIds.length < 2) return null
      const state = get()
      const nodes = nodeIds
        .map((id) => findNodeInTree(state.document.children, id))
        .filter(Boolean) as PenNode[]
      if (nodes.length < 2) return null

      // Compute bounding box
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
      for (const n of nodes) {
        const nx = n.x ?? 0
        const ny = n.y ?? 0
        const nw = 'width' in n && typeof n.width === 'number' ? n.width : 0
        const nh = 'height' in n && typeof n.height === 'number' ? n.height : 0
        minX = Math.min(minX, nx)
        minY = Math.min(minY, ny)
        maxX = Math.max(maxX, nx + nw)
        maxY = Math.max(maxY, ny + nh)
      }

      // Make children relative to group
      const groupChildren = nodes.map((n) => ({
        ...n,
        x: (n.x ?? 0) - minX,
        y: (n.y ?? 0) - minY,
      })) as PenNode[]

      const groupId = nanoid()
      const group: GroupNode = {
        id: groupId,
        type: 'group',
        name: 'Group',
        x: minX,
        y: minY,
        width: maxX - minX,
        height: maxY - minY,
        children: groupChildren,
      }

      // Find insertion position (position of first selected node)
      const firstParent = findParentInTree(state.document.children, nodeIds[0])
      const parentId = firstParent ? firstParent.id : null
      const siblings = firstParent
        ? ('children' in firstParent ? firstParent.children ?? [] : [])
        : state.document.children
      const firstIdx = siblings.findIndex((n) => nodeIds.includes(n.id))

      useHistoryStore.getState().pushState(state.document)

      // Remove all selected nodes
      let newChildren = state.document.children
      for (const id of nodeIds) {
        newChildren = removeNodeFromTree(newChildren, id)
      }

      // Insert group at first node's position
      newChildren = insertNodeInTree(newChildren, parentId, group, firstIdx)

      set({
        document: { ...state.document, children: newChildren },
        isDirty: true,
      })
      return groupId
    },

    ungroupNode: (groupId) => {
      const state = get()
      const group = findNodeInTree(state.document.children, groupId)
      if (!group || group.type !== 'group') return
      if (!('children' in group) || !group.children) return

      const parent = findParentInTree(state.document.children, groupId)
      const parentId = parent ? parent.id : null
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : state.document.children
      const groupIdx = siblings.findIndex((n) => n.id === groupId)

      // Adjust children coordinates to parent space
      const groupX = group.x ?? 0
      const groupY = group.y ?? 0
      const adjustedChildren = group.children.map((child) => ({
        ...child,
        x: (child.x ?? 0) + groupX,
        y: (child.y ?? 0) + groupY,
      })) as PenNode[]

      useHistoryStore.getState().pushState(state.document)

      // Remove group
      let newChildren = removeNodeFromTree(state.document.children, groupId)

      // Insert children at group's position (in reverse to maintain order)
      for (let i = adjustedChildren.length - 1; i >= 0; i--) {
        newChildren = insertNodeInTree(
          newChildren,
          parentId,
          adjustedChildren[i],
          groupIdx,
        )
      }

      set({
        document: { ...state.document, children: newChildren },
        isDirty: true,
      })
    },

    getNodeById: (id) =>
      findNodeInTree(get().document.children, id),

    getParentOf: (id) =>
      findParentInTree(get().document.children, id),

    getFlatNodes: () => flattenNodes(get().document.children),

    applyHistoryState: (doc) =>
      set({ document: doc, isDirty: true }),

    loadDocument: (doc, fileName, fileHandle) => {
      useHistoryStore.getState().clear()
      set({
        document: doc,
        fileName: fileName ?? null,
        fileHandle: fileHandle ?? null,
        isDirty: false,
      })
    },

    newDocument: () => {
      useHistoryStore.getState().clear()
      set({
        document: createEmptyDocument(),
        fileName: null,
        fileHandle: null,
        isDirty: false,
      })
    },

    markClean: () => set({ isDirty: false }),
    setFileHandle: (fileHandle) => set({ fileHandle }),
    setSaveDialogOpen: (saveDialogOpen) => set({ saveDialogOpen }),
  }),
)

export { createEmptyDocument }
export { nanoid as generateId } from 'nanoid'
export { findNodeInTree }
