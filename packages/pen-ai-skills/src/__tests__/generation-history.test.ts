import { describe, it, expect } from 'vitest'
import {
  createHistoryEntry,
  updateFeedback,
  getRecentEntries,
} from '../memory/generation-history'

describe('createHistoryEntry', () => {
  it('creates entry with required fields', () => {
    const entry = createHistoryEntry({
      documentPath: '/test.op',
      prompt: 'build a landing page',
      phase: 'generation',
      skillsUsed: ['schema', 'layout', 'landing-page'],
      nodeCount: 42,
      sectionTypes: ['hero', 'features'],
    })
    expect(entry.id).toBeTruthy()
    expect(entry.timestamp).toBeTruthy()
    expect(entry.input.prompt).toBe('build a landing page')
    expect(entry.output.nodeCount).toBe(42)
    expect(entry.feedback).toBeUndefined()
  })
})

describe('updateFeedback', () => {
  it('sets feedback on an entry', () => {
    const entry = createHistoryEntry({
      documentPath: '/test.op',
      prompt: 'test',
      phase: 'generation',
      skillsUsed: [],
      nodeCount: 1,
      sectionTypes: [],
    })
    const updated = updateFeedback(entry, 'accepted')
    expect(updated.feedback).toBe('accepted')
  })
})

describe('getRecentEntries', () => {
  it('returns last N entries', () => {
    const entries = Array.from({ length: 10 }, (_, i) =>
      createHistoryEntry({
        documentPath: '/test.op',
        prompt: `prompt ${i}`,
        phase: 'generation',
        skillsUsed: [],
        nodeCount: i,
        sectionTypes: [],
      })
    )
    const recent = getRecentEntries(entries, 3)
    expect(recent).toHaveLength(3)
    expect(recent[0].input.prompt).toBe('prompt 7')
  })

  it('filters by documentPath when provided', () => {
    const entries = [
      createHistoryEntry({ documentPath: '/a.op', prompt: 'a', phase: 'generation', skillsUsed: [], nodeCount: 1, sectionTypes: [] }),
      createHistoryEntry({ documentPath: '/b.op', prompt: 'b', phase: 'generation', skillsUsed: [], nodeCount: 1, sectionTypes: [] }),
      createHistoryEntry({ documentPath: '/a.op', prompt: 'c', phase: 'generation', skillsUsed: [], nodeCount: 1, sectionTypes: [] }),
    ]
    const recent = getRecentEntries(entries, 5, '/a.op')
    expect(recent).toHaveLength(2)
  })
})
