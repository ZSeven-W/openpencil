import { create } from 'zustand'
import { nanoid } from 'nanoid'
import type { PenDocument, PenNode, GroupNode, RefNode } from '@/types/pen'
import type { VariableDefinition } from '@/types/variables'
import { useHistoryStore } from '@/stores/history-store'
import { getDefaultTheme } from '@/variables/resolve-variables'
import { replaceVariableRefsInTree } from '@/variables/replace-refs'

export const DEFAULT_FRAME_ID = 'root-frame'

function createEmptyDocument(): PenDocument {
  return {
    version: '1.0.0',
    children: [
      {
        id: DEFAULT_FRAME_ID,
        type: 'frame',
        name: 'Frame',
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        children: [],
      },
    ],
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

/** Resolve the bounding box of a node, falling back to its referenced component for RefNodes. */
function getNodeBounds(
  node: PenNode,
  allNodes: PenNode[],
): { x: number; y: number; w: number; h: number } {
  const x = node.x ?? 0
  const y = node.y ?? 0
  let w = ('width' in node && typeof node.width === 'number') ? node.width : 0
  let h = ('height' in node && typeof node.height === 'number') ? node.height : 0
  if (node.type === 'ref' && !w) {
    const refComp = findNodeInTree(allNodes, (node as RefNode).ref)
    if (refComp) {
      w = ('width' in refComp && typeof refComp.width === 'number') ? refComp.width : 100
      h = ('height' in refComp && typeof refComp.height === 'number') ? refComp.height : 100
    }
  }
  return { x, y, w: w || 100, h: h || 100 }
}

/**
 * Find a clear X position to the right of `sourceX + sourceW` that doesn't
 * overlap any sibling (excluding `excludeId`) on the same vertical band.
 */
function findClearX(
  sourceX: number,
  sourceW: number,
  proposedY: number,
  proposedH: number,
  siblings: PenNode[],
  excludeId: string,
  allNodes: PenNode[],
  gap = 20,
): number {
  const proposedW = sourceW
  let proposedX = sourceX + sourceW + gap

  const siblingBounds: { x: number; y: number; w: number; h: number }[] = []
  for (const sib of siblings) {
    if (sib.id === excludeId) continue
    const b = getNodeBounds(sib, allNodes)
    if (b.w > 0 && b.h > 0) siblingBounds.push(b)
  }

  let maxAttempts = 100
  while (maxAttempts-- > 0) {
    const hasOverlap = siblingBounds.some((b) => {
      const overlapX = proposedX < b.x + b.w && proposedX + proposedW > b.x
      const overlapY = proposedY < b.y + b.h && proposedY + proposedH > b.y
      return overlapX && overlapY
    })
    if (!hasOverlap) break
    let maxRight = proposedX
    for (const b of siblingBounds) {
      const overlapX = proposedX < b.x + b.w && proposedX + proposedW > b.x
      const overlapY = proposedY < b.y + b.h && proposedY + proposedH > b.y
      if (overlapX && overlapY && b.x + b.w > maxRight) {
        maxRight = b.x + b.w
      }
    }
    proposedX = maxRight + gap
  }

  return proposedX
}

function isDescendantOf(
  nodes: PenNode[],
  nodeId: string,
  ancestorId: string,
): boolean {
  const ancestor = findNodeInTree(nodes, ancestorId)
  if (!ancestor || !('children' in ancestor) || !ancestor.children) return false
  for (const child of ancestor.children) {
    if (child.id === nodeId) return true
    if (isDescendantOf([child], nodeId, child.id)) return true
  }
  return false
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
    if (n.id === parentId) {
      const children = 'children' in n && n.children ? [...n.children] : []
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

/** Recursively scale all children's relative positions and sizes. */
function scaleChildrenInPlace(
  children: PenNode[],
  scaleX: number,
  scaleY: number,
): PenNode[] {
  return children.map((child) => {
    const updated: Record<string, unknown> = { ...child }
    if (child.x !== undefined) updated.x = child.x * scaleX
    if (child.y !== undefined) updated.y = child.y * scaleY
    if ('width' in child && typeof child.width === 'number') {
      updated.width = child.width * scaleX
    }
    if ('height' in child && typeof child.height === 'number') {
      updated.height = child.height * scaleY
    }
    if ('children' in child && child.children) {
      updated.children = scaleChildrenInPlace(child.children, scaleX, scaleY)
    }
    return updated as unknown as PenNode
  })
}

/** Recursively rotate all children's relative positions and angles. */
function rotateChildrenInPlace(
  children: PenNode[],
  angleDeltaDeg: number,
): PenNode[] {
  const rad = (angleDeltaDeg * Math.PI) / 180
  const cos = Math.cos(rad)
  const sin = Math.sin(rad)
  return children.map((child) => {
    const x = child.x ?? 0
    const y = child.y ?? 0
    const updated: Record<string, unknown> = { ...child }
    updated.x = x * cos - y * sin
    updated.y = x * sin + y * cos
    updated.rotation = ((child.rotation ?? 0) + angleDeltaDeg) % 360
    if ('children' in child && child.children) {
      updated.children = rotateChildrenInPlace(child.children, angleDeltaDeg)
    }
    return updated as unknown as PenNode
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
  scaleDescendantsInStore: (
    parentId: string,
    scaleX: number,
    scaleY: number,
  ) => void
  rotateDescendantsInStore: (
    parentId: string,
    angleDeltaDeg: number,
  ) => void
  getNodeById: (id: string) => PenNode | undefined
  getParentOf: (id: string) => PenNode | undefined
  getFlatNodes: () => PenNode[]
  isDescendantOf: (nodeId: string, ancestorId: string) => boolean

  // Component management
  makeReusable: (nodeId: string) => void
  detachComponent: (nodeId: string) => string | undefined

  // Variable management
  setVariable: (name: string, definition: VariableDefinition) => void
  removeVariable: (name: string) => void
  renameVariable: (oldName: string, newName: string) => void
  setThemes: (themes: Record<string, string[]>) => void

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

      // Duplicating a reusable component creates an instance (RefNode)
      if ('reusable' in node && node.reusable === true) {
        const bounds = getNodeBounds(node, state.document.children)
        const parent = findParentInTree(state.document.children, id)
        const parentId = parent ? parent.id : null
        const siblings = parent
          ? ('children' in parent ? parent.children ?? [] : [])
          : state.document.children
        const idx = siblings.findIndex((n) => n.id === id)

        const clearX = findClearX(
          bounds.x, bounds.w, bounds.y, bounds.h,
          siblings, id, state.document.children,
        )

        const refNode: RefNode = {
          id: nanoid(),
          type: 'ref',
          ref: node.id,
          name: node.name ?? node.type,
          x: clearX,
          y: bounds.y,
        }

        useHistoryStore.getState().pushState(state.document)
        set((s) => ({
          document: {
            ...s.document,
            children: insertNodeInTree(
              s.document.children,
              parentId,
              refNode as PenNode,
              idx + 1,
            ),
          },
          isDirty: true,
        }))
        return refNode.id
      }

      // Regular duplication for non-reusable nodes
      const cloneWithNewIds = (n: PenNode): PenNode => {
        const cloned = { ...n, id: nanoid() } as PenNode
        if ('children' in cloned && cloned.children) {
          cloned.children = cloned.children.map(cloneWithNewIds)
        }
        return cloned
      }

      const clone = cloneWithNewIds(node)
      clone.name = (clone.name ?? clone.type) + ' copy'

      const parent = findParentInTree(state.document.children, id)
      const parentId = parent ? parent.id : null
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : state.document.children
      const idx = siblings.findIndex((n) => n.id === id)

      const bounds = getNodeBounds(node, state.document.children)
      clone.x = findClearX(
        bounds.x, bounds.w, bounds.y, bounds.h,
        siblings, id, state.document.children,
      )
      clone.y = bounds.y

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

    scaleDescendantsInStore: (parentId, scaleX, scaleY) => {
      if (scaleX === 1 && scaleY === 1) return
      const state = get()
      const parent = findNodeInTree(state.document.children, parentId)
      if (!parent || !('children' in parent) || !parent.children) return

      const scaledChildren = scaleChildrenInPlace(
        parent.children,
        scaleX,
        scaleY,
      )
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(s.document.children, parentId, {
            children: scaledChildren,
          } as Partial<PenNode>),
        },
        isDirty: true,
      }))
    },

    rotateDescendantsInStore: (parentId, angleDeltaDeg) => {
      if (angleDeltaDeg === 0) return
      const state = get()
      const parent = findNodeInTree(state.document.children, parentId)
      if (!parent || !('children' in parent) || !parent.children) return

      const rotatedChildren = rotateChildrenInPlace(
        parent.children,
        angleDeltaDeg,
      )
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(s.document.children, parentId, {
            children: rotatedChildren,
          } as Partial<PenNode>),
        },
        isDirty: true,
      }))
    },

    getNodeById: (id) =>
      findNodeInTree(get().document.children, id),

    getParentOf: (id) =>
      findParentInTree(get().document.children, id),

    getFlatNodes: () => flattenNodes(get().document.children),
    isDescendantOf: (nodeId, ancestorId) =>
      isDescendantOf(get().document.children, nodeId, ancestorId),

    // --- Component management ---

    makeReusable: (nodeId) => {
      const state = get()
      const node = findNodeInTree(state.document.children, nodeId)
      if (!node) return
      // Only container types (frame, group, rectangle) can be made reusable
      if (node.type !== 'frame' && node.type !== 'group' && node.type !== 'rectangle') return
      if ('reusable' in node && node.reusable) return
      useHistoryStore.getState().pushState(state.document)
      set((s) => ({
        document: {
          ...s.document,
          children: updateNodeInTree(s.document.children, nodeId, {
            reusable: true,
          } as Partial<PenNode>),
        },
        isDirty: true,
      }))
    },

    detachComponent: (nodeId) => {
      const state = get()
      const node = findNodeInTree(state.document.children, nodeId)
      if (!node) return

      // Case 1: Detach a reusable component (remove reusable flag)
      if ('reusable' in node && node.reusable) {
        useHistoryStore.getState().pushState(state.document)
        set((s) => ({
          document: {
            ...s.document,
            children: updateNodeInTree(s.document.children, nodeId, {
              reusable: undefined,
            } as Partial<PenNode>),
          },
          isDirty: true,
        }))
        return nodeId
      }

      // Case 2: Detach an instance (RefNode → independent node tree)
      if (node.type === 'ref') {
        const component = findNodeInTree(state.document.children, node.ref)
        if (!component) return

        useHistoryStore.getState().pushState(state.document)

        // Apply overrides to a copy of the component before cloning IDs
        const source = structuredClone(component)
        // Apply top-level visual overrides (fill, stroke, etc.)
        const topOverrides = node.descendants?.[node.ref]
        if (topOverrides) {
          Object.assign(source, topOverrides)
        }
        // Apply child-level overrides
        if (node.descendants && 'children' in source && source.children) {
          source.children = source.children.map((child: PenNode) => {
            const override = node.descendants?.[child.id]
            return override ? ({ ...child, ...override } as PenNode) : child
          })
        }

        // Clone with new IDs
        const cloneWithNewIds = (n: PenNode): PenNode => {
          const cloned = { ...n, id: nanoid() } as PenNode
          if ('children' in cloned && cloned.children) {
            cloned.children = cloned.children.map(cloneWithNewIds)
          }
          return cloned
        }
        const detached = cloneWithNewIds(source)
        // Apply all direct instance properties (position, size, meta)
        const detachedRecord = detached as unknown as Record<string, unknown>
        for (const [key, val] of Object.entries(node)) {
          if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children' || key === 'id') continue
          if (val !== undefined) {
            detachedRecord[key] = val
          }
        }
        if (!detached.name) detached.name = source.name
        delete detachedRecord.reusable

        // Replace the RefNode with the detached tree
        const parent = findParentInTree(state.document.children, nodeId)
        const parentId = parent ? parent.id : null
        const siblings = parent
          ? ('children' in parent ? parent.children ?? [] : [])
          : state.document.children
        const idx = siblings.findIndex((n) => n.id === nodeId)

        let newChildren = removeNodeFromTree(state.document.children, nodeId)
        newChildren = insertNodeInTree(
          newChildren,
          parentId,
          detached,
          idx >= 0 ? idx : undefined,
        )

        set({
          document: { ...state.document, children: newChildren },
          isDirty: true,
        })
        return detached.id
      }
    },

    // --- Variable management ---

    setVariable: (name, definition) => {
      useHistoryStore.getState().pushState(get().document)
      set((s) => ({
        document: {
          ...s.document,
          variables: { ...(s.document.variables ?? {}), [name]: definition },
        },
        isDirty: true,
      }))
    },

    removeVariable: (name) => {
      const state = get()
      const vars = state.document.variables
      if (!vars || !(name in vars)) return
      useHistoryStore.getState().pushState(state.document)
      const { [name]: _removed, ...rest } = vars
      const activeTheme = getDefaultTheme(state.document.themes)
      const newChildren = replaceVariableRefsInTree(
        state.document.children,
        name,
        null,
        vars,
        activeTheme,
      )
      set({
        document: {
          ...state.document,
          variables: Object.keys(rest).length > 0 ? rest : undefined,
          children: newChildren,
        },
        isDirty: true,
      })
    },

    renameVariable: (oldName, newName) => {
      if (oldName === newName) return
      const state = get()
      const vars = state.document.variables
      if (!vars || !(oldName in vars)) return
      useHistoryStore.getState().pushState(state.document)
      const def = vars[oldName]
      const { [oldName]: _removed, ...rest } = vars
      const newVars = { ...rest, [newName]: def }
      const activeTheme = getDefaultTheme(state.document.themes)
      const newChildren = replaceVariableRefsInTree(
        state.document.children,
        oldName,
        newName,
        vars,
        activeTheme,
      )
      set({
        document: {
          ...state.document,
          variables: newVars,
          children: newChildren,
        },
        isDirty: true,
      })
    },

    setThemes: (themes) => {
      useHistoryStore.getState().pushState(get().document)
      set((s) => ({
        document: { ...s.document, themes },
        isDirty: true,
      }))
    },

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
