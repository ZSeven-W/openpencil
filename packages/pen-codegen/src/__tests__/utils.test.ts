import { describe, it, expect } from 'vitest'
import { varOrLiteral, sanitizeName, nodeTreeToSummary } from '../utils'
import type { PenNode } from '@zseven-w/pen-types'

describe('varOrLiteral', () => {
  it('returns CSS var() for variable references', () => {
    expect(varOrLiteral('$primary-color')).toBe('var(--primary-color)')
  })

  it('returns raw value for non-variable strings', () => {
    expect(varOrLiteral('#ff0000')).toBe('#ff0000')
  })

  it('handles variable names with spaces', () => {
    expect(varOrLiteral('$Primary Color')).toBe('var(--primary-color)')
  })
})

describe('sanitizeName', () => {
  it('converts kebab-case to PascalCase', () => {
    expect(sanitizeName('hero-section')).toBe('HeroSection')
  })

  it('converts space-separated to PascalCase', () => {
    expect(sanitizeName('my cool component')).toBe('MyCoolComponent')
  })

  it('handles single word', () => {
    expect(sanitizeName('navbar')).toBe('Navbar')
  })

  it('strips invalid characters', () => {
    expect(sanitizeName('hello@world#123')).toBe('Helloworld123')
  })
})

describe('nodeTreeToSummary', () => {
  it('produces lightweight summary with node IDs', () => {
    const nodes: PenNode[] = [
      {
        id: '1', type: 'frame', name: 'Header',
        x: 0, y: 0, width: 1200, height: 80,
        children: [
          { id: '1-1', type: 'text', name: 'Title', x: 0, y: 0, width: 200, height: 24 } as PenNode,
        ],
      } as PenNode,
    ]
    const summary = nodeTreeToSummary(nodes)
    expect(summary).toContain('[1]')
    expect(summary).toContain('Header')
    expect(summary).toContain('frame')
    expect(summary).toContain('1200')
    expect(summary).toContain('[1-1]')
    expect(summary).toContain('Title')
    expect(summary.length).toBeLessThan(500)
  })

  it('returns empty string for empty array', () => {
    expect(nodeTreeToSummary([])).toBe('')
  })
})
