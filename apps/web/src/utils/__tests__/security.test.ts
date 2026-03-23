// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest'

// paper.js self-initializes a Canvas 2D context on import, which crashes in
// jsdom. Mock it before any transitive import can trigger the real module.
vi.mock('paper', () => ({
  default: {
    PaperScope: class {},
    setup: () => {},
    project: { activeLayer: { removeChildren: () => {} } },
  },
}))

import { parseSvgToNodes } from '@/utils/svg-parser'

// ---------------------------------------------------------------------------
// 1. SVG parser SKIP_TAGS — dangerous elements are stripped
// ---------------------------------------------------------------------------

describe('SVG parser SKIP_TAGS', () => {
  /** Recursively collect all node names in the tree */
  function collectNames(nodes: { name?: string; children?: any[] }[]): string[] {
    const names: string[] = []
    for (const n of nodes) {
      if (n.name) names.push(n.name)
      if (n.children) names.push(...collectNames(n.children))
    }
    return names
  }

  /** Recursively collect all node types */
  function collectTypes(nodes: { type?: string; children?: any[] }[]): string[] {
    const types: string[] = []
    for (const n of nodes) {
      if (n.type) types.push(n.type)
      if (n.children) types.push(...collectTypes(n.children))
    }
    return types
  }

  it('strips <script> tags from parsed SVG', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="red"/>
        <script>alert("xss")</script>
      </svg>`
    const nodes = parseSvgToNodes(svg)
    const names = collectNames(nodes).map(n => n.toLowerCase())
    expect(names).not.toContain('script')
    // The rect should still be present
    expect(nodes.length).toBeGreaterThan(0)
  })

  it('strips <foreignObject> tags from parsed SVG', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="blue"/>
        <foreignObject width="100" height="100">
          <div xmlns="http://www.w3.org/1999/xhtml">HTML content</div>
        </foreignObject>
      </svg>`
    const nodes = parseSvgToNodes(svg)
    const names = collectNames(nodes).map(n => n.toLowerCase())
    expect(names).not.toContain('foreignobject')
    expect(names).not.toContain('foreignObject')
  })

  it('strips <animate>, <animateMotion>, and <set> tags', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="green">
          <animate attributeName="x" from="0" to="100" dur="1s"/>
          <animateMotion path="M0,0 L100,100" dur="2s"/>
          <set attributeName="fill" to="red"/>
        </rect>
      </svg>`
    const nodes = parseSvgToNodes(svg)
    const types = collectTypes(nodes)
    // Only rectangle-type nodes should remain, no animation nodes
    for (const t of types) {
      expect(['rectangle', 'frame', 'path', 'ellipse', 'line', 'text', 'group']).toContain(t)
    }
  })

  it('preserves valid shape elements while stripping dangerous ones', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
        <rect x="0" y="0" width="50" height="50" fill="red"/>
        <circle cx="100" cy="100" r="30" fill="blue"/>
        <script>document.cookie</script>
        <foreignObject><body xmlns="http://www.w3.org/1999/xhtml"><iframe/></body></foreignObject>
      </svg>`
    const nodes = parseSvgToNodes(svg)
    // Should have a wrapping frame with 2 children (rect + circle)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].type).toBe('frame')
    expect('children' in nodes[0] && nodes[0].children).toHaveLength(2)
  })
})

// ---------------------------------------------------------------------------
// 2. SVG parser getAttr — ReDoS-safe regex escaping
// ---------------------------------------------------------------------------

describe('SVG parser getAttr ReDoS safety', () => {
  it('parses SVG with regex-special chars in style attribute without hanging', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" style="(x+x+)+y: red; fill: blue"/>
      </svg>`

    const start = performance.now()
    const nodes = parseSvgToNodes(svg)
    const elapsed = performance.now() - start

    // Must complete in under 100ms (would hang without escaping)
    expect(elapsed).toBeLessThan(100)
    expect(nodes.length).toBeGreaterThan(0)
  })

  it('handles style with brackets and special characters safely', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" style="fill: green; [weird]: val; opacity: 0.5"/>
      </svg>`

    const start = performance.now()
    const nodes = parseSvgToNodes(svg)
    const elapsed = performance.now() - start

    expect(elapsed).toBeLessThan(100)
    expect(nodes.length).toBeGreaterThan(0)
  })
})

// NOTE: Figma parser decompression limits (MAX_UNZIPPED_SIZE = 100MB,
// MAX_IMAGE_SIZE = 50MB) are internal constants in fig-parser.ts. They guard
// against zip bombs during .fig file decompression. Testing requires crafting
// valid .fig binaries with oversized payloads, and the WASM-based parser
// hangs vitest's jsdom environment, so these are verified via code review.
