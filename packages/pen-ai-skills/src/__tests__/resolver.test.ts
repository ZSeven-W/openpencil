import { describe, it, expect } from 'vitest'
import { matchTrigger, filterByIntent } from '../engine/resolver'
import type { SkillRegistryEntry } from '../engine/types'

const skill = (
  name: string,
  trigger: null | { keywords: string[] } | { flags: string[] },
  priority = 50
): SkillRegistryEntry => ({
  meta: { name, description: '', phase: ['generation'], trigger, priority, budget: 2000, category: 'domain' },
  content: `content of ${name}`,
})

describe('matchTrigger', () => {
  it('null trigger always matches', () => {
    expect(matchTrigger(null, 'any message', {})).toBe(true)
  })

  it('keyword trigger matches case-insensitively', () => {
    expect(matchTrigger({ keywords: ['landing'] }, 'Build a Landing Page', {})).toBe(true)
    expect(matchTrigger({ keywords: ['landing'] }, 'Build a dashboard', {})).toBe(false)
  })

  it('keyword trigger matches if any keyword matches', () => {
    expect(matchTrigger({ keywords: ['dashboard', 'table'] }, 'Create a table view', {})).toBe(true)
  })

  it('flag trigger matches when all flags are true', () => {
    expect(matchTrigger({ flags: ['hasVariables'] }, '', { hasVariables: true })).toBe(true)
    expect(matchTrigger({ flags: ['hasVariables'] }, '', { hasVariables: false })).toBe(false)
    expect(matchTrigger({ flags: ['hasVariables'] }, '', {})).toBe(false)
  })

  it('flag trigger requires ALL flags to be true', () => {
    expect(
      matchTrigger({ flags: ['hasVariables', 'hasDesignMd'] }, '', {
        hasVariables: true,
        hasDesignMd: false,
      })
    ).toBe(false)
  })
})

describe('filterByIntent', () => {
  it('includes always-on skills', () => {
    const skills = [skill('base', null), skill('landing', { keywords: ['landing'] })]
    const result = filterByIntent(skills, 'build a dashboard', {})
    expect(result.map(s => s.meta.name)).toEqual(['base'])
  })

  it('includes keyword-matched skills', () => {
    const skills = [skill('base', null), skill('landing', { keywords: ['landing'] })]
    const result = filterByIntent(skills, 'build a landing page', {})
    expect(result.map(s => s.meta.name)).toEqual(['base', 'landing'])
  })

  it('sorts by priority', () => {
    const skills = [
      skill('b', null, 50),
      skill('a', null, 10),
      skill('c', null, 30),
    ]
    const result = filterByIntent(skills, '', {})
    expect(result.map(s => s.meta.name)).toEqual(['a', 'c', 'b'])
  })
})
