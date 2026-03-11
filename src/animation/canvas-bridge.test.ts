import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { Canvas, FabricObject } from 'fabric'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AnimatableValue } from '@/types/animation'
import {
  applyAnimatedFrame,
  captureNodeState,
  recalcCoordsForAnimatedObjects,
  restoreNodeStates,
  buildFabricObjectMap,
  clearFabricObjectMap,
  findFabricObject,
  isPlaybackActive,
  markCursorUpdate,
  getCursorUpdateCount,
  lockObjectInteraction,
  unlockObjectInteraction,
} from '@/animation/canvas-bridge'

// Import bindings to ensure they're registered
import '@/animation/canvas-property-bindings'

function createMockFabricObject(
  overrides: Partial<FabricObjectWithPenId> = {},
): FabricObjectWithPenId {
  return {
    penNodeId: 'node-1',
    left: 100,
    top: 200,
    scaleX: 1,
    scaleY: 1,
    angle: 0,
    opacity: 1,
    fill: '#ff0000',
    stroke: '#000000',
    strokeWidth: 1,
    dirty: false,
    selectable: true,
    evented: true,
    setCoords: vi.fn(),
    ...overrides,
  } as unknown as FabricObjectWithPenId
}

function createMockCanvas(
  objects: FabricObjectWithPenId[] = [],
): Canvas {
  return {
    getObjects: vi.fn(() => objects),
  } as unknown as Canvas
}

beforeEach(() => {
  clearFabricObjectMap()
})

// --- v2: applyAnimatedFrame ---

describe('applyAnimatedFrame', () => {
  it('applies transform values via bindings', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, { x: 50, y: 75, opacity: 0.5 })

    expect(obj.left).toBe(50)
    expect(obj.top).toBe(75)
    expect(obj.opacity).toBe(0.5)
  })

  it('does not set dirty for transform-only properties', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, { x: 10, y: 20, rotation: 45 })

    expect(obj.dirty).toBe(false)
  })

  it('sets dirty when cache-invalidating properties are applied', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, {
      'fill.color': '#00ff00' as AnimatableValue,
    })

    expect(obj.fill).toBe('#00ff00')
    expect(obj.dirty).toBe(true)
  })

  it('sets dirty when mixing transform and visual properties', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, {
      x: 10,
      'stroke.color': '#0000ff' as AnimatableValue,
    })

    expect(obj.left).toBe(10)
    expect(obj.stroke).toBe('#0000ff')
    expect(obj.dirty).toBe(true)
  })

  it('skips unknown binding keys', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, {
      x: 50,
      unknownProp: 999,
    })

    expect(obj.left).toBe(50)
  })

  it('does not call setCoords', () => {
    const obj = createMockFabricObject()
    applyAnimatedFrame(obj as FabricObject, { x: 50, y: 75 })

    expect(obj.setCoords).not.toHaveBeenCalled()
  })
})

// --- v2: captureNodeState ---

describe('captureNodeState', () => {
  it('captures all registered properties', () => {
    const obj = createMockFabricObject({
      left: 42,
      top: 84,
      scaleX: 2,
      scaleY: 3,
      angle: 90,
      opacity: 0.7,
    } as Partial<FabricObjectWithPenId>)

    const state = captureNodeState(obj as FabricObject)

    expect(state.x).toBe(42)
    expect(state.y).toBe(84)
    expect(state.scaleX).toBe(2)
    expect(state.scaleY).toBe(3)
    expect(state.rotation).toBe(90)
    expect(state.opacity).toBe(0.7)
  })

  it('captures visual properties', () => {
    const obj = createMockFabricObject({
      fill: '#abcdef',
      stroke: '#123456',
      strokeWidth: 3,
    } as Partial<FabricObjectWithPenId>)

    const state = captureNodeState(obj as FabricObject)

    expect(state['fill.color']).toBe('#abcdef')
    expect(state['stroke.color']).toBe('#123456')
    expect(state.strokeWidth).toBe(3)
  })
})

// --- v2: recalcCoordsForAnimatedObjects ---

describe('recalcCoordsForAnimatedObjects', () => {
  it('calls setCoords on all cached objects', () => {
    const obj1 = createMockFabricObject({ penNodeId: 'a' })
    const obj2 = createMockFabricObject({ penNodeId: 'b' })
    const canvas = createMockCanvas([obj1, obj2])

    buildFabricObjectMap(canvas)
    recalcCoordsForAnimatedObjects()

    expect(obj1.setCoords).toHaveBeenCalledOnce()
    expect(obj2.setCoords).toHaveBeenCalledOnce()
  })

  it('does nothing when map is empty', () => {
    recalcCoordsForAnimatedObjects()
  })
})

// --- v2: restoreNodeStates ---

describe('restoreNodeStates', () => {
  it('applies saved values and calls setCoords per object', () => {
    const obj = createMockFabricObject({ penNodeId: 'node-1' })
    const canvas = createMockCanvas([obj])
    buildFabricObjectMap(canvas)

    const saved = new Map<string, Record<string, AnimatableValue>>()
    saved.set('node-1', { x: 300, y: 400, opacity: 0.3 })

    restoreNodeStates(canvas, saved)

    expect(obj.left).toBe(300)
    expect(obj.top).toBe(400)
    expect(obj.opacity).toBe(0.3)
    expect(obj.setCoords).toHaveBeenCalledOnce()
  })

  it('skips unknown node IDs', () => {
    const canvas = createMockCanvas([])
    buildFabricObjectMap(canvas)

    const saved = new Map<string, Record<string, AnimatableValue>>()
    saved.set('nonexistent', { x: 100 })

    restoreNodeStates(canvas, saved)
  })
})

// --- Shared utilities ---

describe('cursor guard', () => {
  it('marks and reads count', () => {
    const before = getCursorUpdateCount()
    markCursorUpdate()
    expect(getCursorUpdateCount()).toBe(before + 1)
  })
})

describe('isPlaybackActive', () => {
  it('returns false when no engine is playing', () => {
    expect(isPlaybackActive()).toBe(false)
  })
})

describe('buildFabricObjectMap / findFabricObject', () => {
  it('builds map and looks up by nodeId', () => {
    const obj = createMockFabricObject({ penNodeId: 'abc' })
    const canvas = createMockCanvas([obj])

    buildFabricObjectMap(canvas)
    const found = findFabricObject(canvas, 'abc')

    expect(found).toBe(obj)
  })

  it('falls back to linear scan when map is empty', () => {
    const obj = createMockFabricObject({ penNodeId: 'xyz' })
    const canvas = createMockCanvas([obj])

    const found = findFabricObject(canvas, 'xyz')
    expect(found).toBe(obj)
  })

  it('returns null for missing objects', () => {
    const canvas = createMockCanvas([])
    expect(findFabricObject(canvas, 'nope')).toBeNull()
  })
})

describe('lockObjectInteraction / unlockObjectInteraction', () => {
  it('locks and unlocks object interaction', () => {
    const obj = createMockFabricObject({ penNodeId: 'node-1' })
    const canvas = createMockCanvas([obj])

    lockObjectInteraction(canvas, 'node-1')
    expect(obj.selectable).toBe(false)
    expect(obj.evented).toBe(false)

    unlockObjectInteraction(canvas, 'node-1')
    expect(obj.selectable).toBe(true)
    expect(obj.evented).toBe(true)
  })
})
