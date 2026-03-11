import { describe, it, expect } from 'vitest'
import { CONTENT_TEMPLATES, getTemplateById } from './content-templates'

describe('CONTENT_TEMPLATES', () => {
  it('has 5 essential templates', () => {
    expect(CONTENT_TEMPLATES.length).toBe(5)
  })

  it('all templates have unique IDs', () => {
    const ids = CONTENT_TEMPLATES.map((t) => t.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('all templates create valid PenNode trees', () => {
    for (const template of CONTENT_TEMPLATES) {
      const node = template.create()
      expect(node.type).toBe('frame')
      expect(node.id).toBeTruthy()
      expect(node.reusable).toBe(true)
      expect(node.width).toBe('fill_container')
      expect(node.height).toBe('fill_container')
      expect(node.children).toBeDefined()
      expect(node.children!.length).toBeGreaterThan(0)
    }
  })

  it('generates unique IDs on each create() call', () => {
    const template = CONTENT_TEMPLATES[0]
    const node1 = template.create()
    const node2 = template.create()
    expect(node1.id).not.toBe(node2.id)
  })

  it('all text nodes use $variable refs for font properties', () => {
    for (const template of CONTENT_TEMPLATES) {
      const node = template.create()
      walkNodes(node.children ?? [], (child) => {
        if (child.type === 'text') {
          // fontFamily should be a $variable ref
          if (child.fontFamily) {
            expect(String(child.fontFamily).startsWith('$'), `${template.name}/${child.name}: fontFamily should be a $ref`).toBe(true)
          }
          // fill color should be a $variable ref
          if (Array.isArray(child.fill)) {
            for (const f of child.fill) {
              if (f.type === 'solid') {
                expect(f.color.startsWith('$'), `${template.name}/${child.name}: fill color should be a $ref`).toBe(true)
              }
            }
          }
        }
      })
    }
  })

  it('all frame backgrounds use $variable refs', () => {
    for (const template of CONTENT_TEMPLATES) {
      const node = template.create()
      if (Array.isArray(node.fill)) {
        for (const f of node.fill) {
          if (f.type === 'solid') {
            expect(f.color.startsWith('$'), `${template.name}: root fill color should be a $ref`).toBe(true)
          }
        }
      }
    }
  })
})

describe('getTemplateById', () => {
  it('returns template by ID', () => {
    const template = getTemplateById('tpl-title-intro')
    expect(template).toBeDefined()
    expect(template!.name).toBe('Title / Intro')
  })

  it('returns undefined for unknown ID', () => {
    expect(getTemplateById('nonexistent')).toBeUndefined()
  })
})

// Helper to walk all nodes
function walkNodes(nodes: unknown[], visitor: (node: Record<string, unknown>) => void): void {
  for (const node of nodes) {
    const n = node as Record<string, unknown>
    visitor(n)
    if (Array.isArray(n.children)) {
      walkNodes(n.children, visitor)
    }
  }
}
