import { describe, it, expect, beforeEach, vi } from 'vitest'
import type { VibeKit } from '@/types/vibekit'
import { VIBE_KIT_SCHEMA, VIBE_KIT_VARIABLE_NAMES } from './schema'

// Mock stores before importing applicator
const mockSetVariable = vi.fn()
const mockSetThemes = vi.fn()
const mockStartBatch = vi.fn()
const mockEndBatch = vi.fn()
const mockSetActiveKit = vi.fn()

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      document: { version: '1', children: [], variables: {}, themes: {} },
      setVariable: mockSetVariable,
      setThemes: mockSetThemes,
    }),
  },
}))

vi.mock('@/stores/history-store', () => ({
  useHistoryStore: {
    getState: () => ({
      startBatch: mockStartBatch,
      endBatch: mockEndBatch,
    }),
  },
}))

vi.mock('@/stores/vibekit-store', () => ({
  useVibeKitStore: {
    getState: () => ({
      setActiveKit: mockSetActiveKit,
    }),
  },
}))

import { applyKit, extractKitFromDocument } from './kit-applicator'

function createTestKit(overrides?: Partial<VibeKit>): VibeKit {
  return {
    id: 'test-kit-1',
    name: 'Test Kit',
    version: '1.0.0',
    variables: {
      'color-primary': { type: 'color', value: '#ff0000' },
      'font-heading': { type: 'string', value: 'Helvetica, sans-serif' },
      'space-md': { type: 'number', value: 20 },
    },
    themes: { 'Theme-1': ['Light', 'Dark'] },
    assets: {},
    metadata: { createdAt: '2026-01-01', generatedBy: 'manual' },
    ...overrides,
  }
}

describe('applyKit', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('wraps all mutations in a history batch', () => {
    const kit = createTestKit()
    applyKit(kit)

    expect(mockStartBatch).toHaveBeenCalledTimes(1)
    expect(mockEndBatch).toHaveBeenCalledTimes(1)

    // startBatch should be called before setVariable
    const startOrder = mockStartBatch.mock.invocationCallOrder[0]
    const endOrder = mockEndBatch.mock.invocationCallOrder[0]
    expect(startOrder).toBeLessThan(endOrder)
  })

  it('sets themes from the kit', () => {
    const kit = createTestKit()
    applyKit(kit)

    expect(mockSetThemes).toHaveBeenCalledWith({ 'Theme-1': ['Light', 'Dark'] })
  })

  it('sets all kit variables', () => {
    const kit = createTestKit()
    applyKit(kit)

    expect(mockSetVariable).toHaveBeenCalledWith('color-primary', { type: 'color', value: '#ff0000' })
    expect(mockSetVariable).toHaveBeenCalledWith('font-heading', { type: 'string', value: 'Helvetica, sans-serif' })
    expect(mockSetVariable).toHaveBeenCalledWith('space-md', { type: 'number', value: 20 })
  })

  it('fills missing schema variables with fallbacks', () => {
    const kit = createTestKit()
    applyKit(kit)

    // Should have been called for each kit variable + each missing schema variable
    const totalCalls = mockSetVariable.mock.calls.length
    expect(totalCalls).toBe(VIBE_KIT_VARIABLE_NAMES.length)

    // Check a fallback was applied for a variable NOT in the kit
    const colorBgCall = mockSetVariable.mock.calls.find(
      (c: unknown[]) => c[0] === 'color-bg',
    )
    expect(colorBgCall).toBeDefined()
    expect(colorBgCall![1]).toEqual({
      type: VIBE_KIT_SCHEMA['color-bg'].type,
      value: VIBE_KIT_SCHEMA['color-bg'].fallback,
    })
  })

  it('updates the active kit in the vibekit store', () => {
    const kit = createTestKit()
    applyKit(kit)

    expect(mockSetActiveKit).toHaveBeenCalledWith('test-kit-1')
  })

  it('skips setThemes when kit has no themes', () => {
    const kit = createTestKit({ themes: undefined })
    applyKit(kit)

    expect(mockSetThemes).not.toHaveBeenCalled()
  })
})

describe('extractKitFromDocument', () => {
  it('creates a kit with current document variables', () => {
    const kit = extractKitFromDocument('My Kit')

    expect(kit.name).toBe('My Kit')
    expect(kit.version).toBe('1.0.0')
    expect(kit.metadata.generatedBy).toBe('manual')
    expect(kit.id).toMatch(/^kit-/)
  })
})
