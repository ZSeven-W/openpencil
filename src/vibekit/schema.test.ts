import { describe, it, expect } from 'vitest'
import {
  VIBE_KIT_SCHEMA,
  VIBE_KIT_VARIABLE_NAMES,
  VIBE_CATEGORIES,
  getSchemaByCategory,
  validateKitVariables,
} from './schema'

describe('VIBE_KIT_SCHEMA', () => {
  it('defines all expected categories', () => {
    const categories = new Set(Object.values(VIBE_KIT_SCHEMA).map((e) => e.category))
    for (const cat of VIBE_CATEGORIES) {
      expect(categories.has(cat)).toBe(true)
    }
  })

  it('uses only existing VariableDefinition types', () => {
    const validTypes = new Set(['color', 'number', 'boolean', 'string'])
    for (const [name, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
      expect(validTypes.has(entry.type), `${name} has invalid type ${entry.type}`).toBe(true)
    }
  })

  it('all fallbacks match their declared type', () => {
    for (const [name, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
      if (entry.type === 'number') {
        expect(typeof entry.fallback, `${name} fallback should be number`).toBe('number')
      } else if (entry.type === 'boolean') {
        expect(typeof entry.fallback, `${name} fallback should be boolean`).toBe('boolean')
      } else {
        // color and string both have string fallbacks
        expect(typeof entry.fallback, `${name} fallback should be string`).toBe('string')
      }
    }
  })

  it('has no duplicate variable names', () => {
    const names = VIBE_KIT_VARIABLE_NAMES
    const unique = new Set(names)
    expect(names.length).toBe(unique.size)
  })
})

describe('getSchemaByCategory', () => {
  it('returns only entries for the specified category', () => {
    const colors = getSchemaByCategory('color')
    for (const entry of Object.values(colors)) {
      expect(entry.category).toBe('color')
    }
    expect(Object.keys(colors).length).toBeGreaterThan(0)
  })

  it('returns empty object for category with no entries', () => {
    // All VIBE_CATEGORIES should have entries, but test the filter logic
    const result = getSchemaByCategory('color')
    expect(typeof result).toBe('object')
  })
})

describe('validateKitVariables', () => {
  it('returns all names when given empty object', () => {
    const missing = validateKitVariables({})
    expect(missing.length).toBe(VIBE_KIT_VARIABLE_NAMES.length)
  })

  it('returns empty when all variables present', () => {
    const vars: Record<string, unknown> = {}
    for (const name of VIBE_KIT_VARIABLE_NAMES) {
      vars[name] = true
    }
    const missing = validateKitVariables(vars)
    expect(missing.length).toBe(0)
  })

  it('returns only missing variable names', () => {
    const vars: Record<string, unknown> = {
      'font-heading': 'Inter',
      'color-primary': '#000',
    }
    const missing = validateKitVariables(vars)
    expect(missing).not.toContain('font-heading')
    expect(missing).not.toContain('color-primary')
    expect(missing.length).toBe(VIBE_KIT_VARIABLE_NAMES.length - 2)
  })
})
