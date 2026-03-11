import { describe, it, expect, vi, beforeEach } from 'vitest'

// Import effects to populate the registries
import '../../animation/effects/index'

import {
  handleListEffects,
  handleListAnimatableProperties,
  handleAddClip,
  handleRemoveClip,
  handleSetComposition,
} from './animation'
import type { PenDocument } from '../../types/pen'

// ---------------------------------------------------------------------------
// Mock document-manager
// ---------------------------------------------------------------------------

let mockDoc: PenDocument

vi.mock('@/mcp/document-manager', () => ({
  resolveDocPath: (p?: string) => p ?? 'test.op',
  openDocument: async () => structuredClone(mockDoc),
  saveDocument: async (_path: string, doc: PenDocument) => {
    mockDoc = doc
  },
}))

function createTestDoc(): PenDocument {
  return {
    version: '1.0.0',
    children: [
      {
        id: 'node-1',
        type: 'frame' as const,
        name: 'Test Frame',
        x: 0,
        y: 0,
        width: 400,
        height: 300,
        children: [
          {
            id: 'text-1',
            type: 'text' as const,
            name: 'Hello',
            x: 10,
            y: 10,
            content: 'Hello',
            fontSize: 16,
          },
        ],
      },
    ],
  }
}

beforeEach(() => {
  mockDoc = createTestDoc()
})

// ---------------------------------------------------------------------------
// list_effects
// ---------------------------------------------------------------------------

describe('list_effects', () => {
  it('returns all effects', () => {
    const result = handleListEffects({})
    expect(result.effects.length).toBeGreaterThan(0)
    for (const e of result.effects) {
      expect(e).toHaveProperty('id')
      expect(e).toHaveProperty('name')
      expect(e).toHaveProperty('category')
      expect(e).toHaveProperty('properties')
      expect(e).toHaveProperty('defaultDuration')
      expect(e).toHaveProperty('parameters')
    }
  })

  it('filters by category', () => {
    const result = handleListEffects({ category: 'enter' })
    expect(result.effects.length).toBeGreaterThan(0)
    for (const e of result.effects) {
      expect(e.category).toBe('enter')
    }
  })

  it('returns empty for nonexistent category', () => {
    const result = handleListEffects({ category: 'nonexistent' })
    expect(result.effects).toHaveLength(0)
  })
})

// ---------------------------------------------------------------------------
// list_animatable_properties
// ---------------------------------------------------------------------------

describe('list_animatable_properties', () => {
  it('returns all properties', () => {
    const result = handleListAnimatableProperties({})
    expect(result.properties.length).toBeGreaterThan(0)
    for (const p of result.properties) {
      expect(p).toHaveProperty('key')
      expect(p).toHaveProperty('type')
      expect(p).toHaveProperty('default')
    }
  })

  it('filters by nodeType', () => {
    const textOnly = handleListAnimatableProperties({ nodeType: 'text' })
    // text should include universal props + text-specific ones
    expect(textOnly.properties.length).toBeGreaterThan(0)
    // Should exclude no properties that restrict to other types
    for (const p of textOnly.properties) {
      if (p.nodeTypes) {
        expect(p.nodeTypes).toContain('text')
      }
    }
    // frame should NOT include text-only properties like fontSize
    const frameOnly = handleListAnimatableProperties({ nodeType: 'frame' })
    const textSpecificKeys = textOnly.properties
      .filter((p) => p.nodeTypes && !p.nodeTypes.includes('frame'))
      .map((p) => p.key)
    for (const key of textSpecificKeys) {
      expect(frameOnly.properties.find((p) => p.key === key)).toBeUndefined()
    }
  })
})

// ---------------------------------------------------------------------------
// add_clip
// ---------------------------------------------------------------------------

describe('add_clip', () => {
  it('adds clip with effectId', async () => {
    const result = await handleAddClip({
      nodeId: 'node-1',
      effectId: 'fade-in',
      startTime: 0,
      duration: 500,
    })

    expect(result.clip).toBeDefined()
    expect(result.clip.kind).toBe('animation')
    expect(result.clip.effectId).toBe('fade-in')
    expect(result.clip.keyframes.length).toBeGreaterThan(0)
    expect(result.clip.id).toBeTruthy()

    // Verify persisted to doc
    const node = mockDoc.children[0]
    expect(node.clips).toHaveLength(1)
    expect(node.clips![0].id).toBe(result.clip.id)
  })

  it('adds clip with raw keyframes', async () => {
    const keyframes = [
      { id: 'kf-1', offset: 0, properties: { opacity: 0 }, easing: 'ease' as const },
      { id: 'kf-2', offset: 1, properties: { opacity: 1 }, easing: 'ease' as const },
    ]
    const result = await handleAddClip({
      nodeId: 'node-1',
      startTime: 100,
      duration: 300,
      keyframes,
    })

    expect(result.clip.keyframes).toEqual(keyframes)
    expect(result.clip.startTime).toBe(100)
    expect(result.clip.duration).toBe(300)
  })

  it('throws when neither effectId nor keyframes provided', async () => {
    await expect(
      handleAddClip({ nodeId: 'node-1', startTime: 0, duration: 500 }),
    ).rejects.toThrow('Either effectId or keyframes must be provided')
  })

  it('throws for unknown node', async () => {
    await expect(
      handleAddClip({ nodeId: 'nonexistent', effectId: 'fade-in', startTime: 0, duration: 500 }),
    ).rejects.toThrow('Node not found')
  })

  it('throws for unknown effect', async () => {
    await expect(
      handleAddClip({ nodeId: 'node-1', effectId: 'bad-effect', startTime: 0, duration: 500 }),
    ).rejects.toThrow('Effect not found')
  })
})

// ---------------------------------------------------------------------------
// remove_clip
// ---------------------------------------------------------------------------

describe('remove_clip', () => {
  it('removes a clip from a node', async () => {
    // First add a clip
    const { clip } = await handleAddClip({
      nodeId: 'node-1',
      effectId: 'fade-in',
      startTime: 0,
      duration: 500,
    })

    // Remove it
    const result = await handleRemoveClip({
      nodeId: 'node-1',
      clipId: clip.id,
    })

    expect(result.ok).toBe(true)
    // clips array should be cleaned up
    expect(mockDoc.children[0].clips).toBeUndefined()
  })

  it('throws for unknown clip', async () => {
    // Add a clip first so the node has clips
    await handleAddClip({
      nodeId: 'node-1',
      effectId: 'fade-in',
      startTime: 0,
      duration: 500,
    })

    await expect(
      handleRemoveClip({ nodeId: 'node-1', clipId: 'bad-id' }),
    ).rejects.toThrow('Clip not found')
  })
})

// ---------------------------------------------------------------------------
// set_composition
// ---------------------------------------------------------------------------

describe('set_composition', () => {
  it('sets composition settings', async () => {
    const result = await handleSetComposition({
      duration: 10000,
      fps: 60,
    })

    expect(result.composition.duration).toBe(10000)
    expect(result.composition.fps).toBe(60)
    expect(mockDoc.composition).toEqual({ duration: 10000, fps: 60 })
  })

  it('partially updates composition', async () => {
    // Set initial
    await handleSetComposition({ duration: 5000, fps: 30 })

    // Update just fps
    const result = await handleSetComposition({ fps: 60 })

    expect(result.composition.duration).toBe(5000)
    expect(result.composition.fps).toBe(60)
  })

  it('uses defaults when no composition exists', async () => {
    const result = await handleSetComposition({ fps: 24 })

    expect(result.composition.duration).toBe(5000)
    expect(result.composition.fps).toBe(24)
  })
})
