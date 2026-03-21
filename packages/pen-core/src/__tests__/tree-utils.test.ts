import { describe, it, expect } from 'vitest'
import type { PenNode, PenDocument } from '@zseven-w/pen-types'
import {
  createEmptyDocument,
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  flattenNodes,
  insertNodeInTree,
  isDescendantOf,
  getActivePageChildren,
  setActivePageChildren,
  migrateToPages,
  DEFAULT_FRAME_ID,
  DEFAULT_PAGE_ID,
} from '../tree-utils'

const frame = (id: string, children: PenNode[] = []): PenNode => ({
  id, type: 'frame', name: id, x: 0, y: 0, width: 100, height: 100,
  fill: [{ type: 'solid', color: '#fff' }], children,
})

const rect = (id: string): PenNode => ({
  id, type: 'rectangle', x: 0, y: 0, width: 50, height: 50,
})

describe('tree-utils', () => {
  describe('createEmptyDocument', () => {
    it('creates a document with a default page and root frame', () => {
      const doc = createEmptyDocument()
      expect(doc.version).toBe('1.0.0')
      expect(doc.pages).toHaveLength(1)
      expect(doc.pages![0].id).toBe(DEFAULT_PAGE_ID)
      expect(doc.pages![0].children).toHaveLength(1)
      expect(doc.pages![0].children[0].id).toBe(DEFAULT_FRAME_ID)
    })
  })

  describe('findNodeInTree', () => {
    it('finds a node by id at root level', () => {
      const nodes = [rect('a'), rect('b')]
      expect(findNodeInTree(nodes, 'b')?.id).toBe('b')
    })

    it('finds a nested node', () => {
      const nodes = [frame('parent', [rect('child')])]
      expect(findNodeInTree(nodes, 'child')?.id).toBe('child')
    })

    it('returns undefined for missing node', () => {
      expect(findNodeInTree([rect('a')], 'missing')).toBeUndefined()
    })
  })

  describe('findParentInTree', () => {
    it('finds the parent of a child node', () => {
      const nodes = [frame('parent', [rect('child')])]
      expect(findParentInTree(nodes, 'child')?.id).toBe('parent')
    })

    it('returns undefined for root nodes', () => {
      expect(findParentInTree([rect('root')], 'root')).toBeUndefined()
    })
  })

  describe('removeNodeFromTree', () => {
    it('removes a root node', () => {
      const result = removeNodeFromTree([rect('a'), rect('b')], 'a')
      expect(result).toHaveLength(1)
      expect(result[0].id).toBe('b')
    })

    it('removes a nested node', () => {
      const nodes = [frame('parent', [rect('child1'), rect('child2')])]
      const result = removeNodeFromTree(nodes, 'child1')
      const parent = result[0] as PenNode & { children: PenNode[] }
      expect(parent.children).toHaveLength(1)
      expect(parent.children[0].id).toBe('child2')
    })
  })

  describe('updateNodeInTree', () => {
    it('updates a node by id', () => {
      const nodes = [rect('a')]
      const result = updateNodeInTree(nodes, 'a', { name: 'updated' })
      expect(result[0].name).toBe('updated')
    })

    it('updates a nested node', () => {
      const nodes = [frame('parent', [rect('child')])]
      const result = updateNodeInTree(nodes, 'child', { name: 'updated' })
      const parent = result[0] as PenNode & { children: PenNode[] }
      expect(parent.children[0].name).toBe('updated')
    })
  })

  describe('flattenNodes', () => {
    it('flattens a nested tree', () => {
      const nodes = [frame('a', [rect('b'), frame('c', [rect('d')])])]
      const flat = flattenNodes(nodes)
      expect(flat.map(n => n.id)).toEqual(['a', 'b', 'c', 'd'])
    })
  })

  describe('insertNodeInTree', () => {
    it('inserts at root level', () => {
      const result = insertNodeInTree([rect('a')], null, rect('b'))
      expect(result).toHaveLength(2)
      expect(result[1].id).toBe('b')
    })

    it('inserts into a parent', () => {
      const nodes = [frame('parent', [rect('existing')])]
      const result = insertNodeInTree(nodes, 'parent', rect('new'))
      const parent = result[0] as PenNode & { children: PenNode[] }
      expect(parent.children).toHaveLength(2)
      expect(parent.children[1].id).toBe('new')
    })

    it('inserts at a specific index', () => {
      const nodes = [frame('parent', [rect('a'), rect('c')])]
      const result = insertNodeInTree(nodes, 'parent', rect('b'), 1)
      const parent = result[0] as PenNode & { children: PenNode[] }
      expect(parent.children.map(n => n.id)).toEqual(['a', 'b', 'c'])
    })
  })

  describe('isDescendantOf', () => {
    it('returns true for a descendant', () => {
      const nodes = [frame('a', [frame('b', [rect('c')])])]
      expect(isDescendantOf(nodes, 'c', 'a')).toBe(true)
    })

    it('returns false for non-descendant', () => {
      const nodes = [frame('a', [rect('b')]), rect('c')]
      expect(isDescendantOf(nodes, 'c', 'a')).toBe(false)
    })
  })

  describe('page helpers', () => {
    it('getActivePageChildren returns page children', () => {
      const doc = createEmptyDocument()
      const children = getActivePageChildren(doc, DEFAULT_PAGE_ID)
      expect(children).toHaveLength(1)
      expect(children[0].id).toBe(DEFAULT_FRAME_ID)
    })

    it('setActivePageChildren replaces page children', () => {
      const doc = createEmptyDocument()
      const newChildren = [rect('new')]
      const updated = setActivePageChildren(doc, DEFAULT_PAGE_ID, newChildren)
      expect(updated.pages![0].children).toHaveLength(1)
      expect(updated.pages![0].children[0].id).toBe('new')
    })

    it('migrateToPages wraps legacy doc', () => {
      const legacy: PenDocument = { version: '1.0.0', children: [rect('a')] }
      const migrated = migrateToPages(legacy)
      expect(migrated.pages).toHaveLength(1)
      expect(migrated.pages![0].children[0].id).toBe('a')
      expect(migrated.children).toEqual([])
    })

    it('migrateToPages preserves existing pages', () => {
      const doc = createEmptyDocument()
      expect(migrateToPages(doc)).toBe(doc)
    })
  })
})
