import { describe, it, expect } from 'vitest'
import type { PenNode, PenDocument } from '@zseven-w/pen-types'
import { generateReactCode, generateReactFromDocument } from '../react-generator'
import { generateHTMLCode, generateHTMLFromDocument } from '../html-generator'
import { generateCSSVariables, variableNameToCSS } from '../css-variables-generator'

const simpleFrame: PenNode = {
  id: 'f1',
  type: 'frame',
  name: 'Card',
  x: 0, y: 0,
  width: 300, height: 200,
  fill: [{ type: 'solid', color: '#ffffff' }],
  cornerRadius: 8,
  children: [
    {
      id: 't1',
      type: 'text',
      content: 'Hello World',
      fontSize: 16,
      fontWeight: 600,
      x: 16, y: 16,
    },
  ],
}

const docWithVars: PenDocument = {
  version: '1.0.0',
  variables: {
    'primary': { type: 'color', value: '#3b82f6' },
    'spacing': { type: 'number', value: 16 },
  },
  themes: { 'Theme-1': ['Light', 'Dark'] },
  children: [simpleFrame],
}

describe('codegen', () => {
  describe('variableNameToCSS', () => {
    it('converts variable name to CSS custom property name', () => {
      expect(variableNameToCSS('primary-color')).toBe('--primary-color')
    })
  })

  describe('generateReactCode', () => {
    it('generates React/Tailwind code for nodes', () => {
      const code = generateReactCode([simpleFrame])
      expect(code).toContain('Card')
      expect(code).toContain('Hello World')
    })
  })

  describe('generateReactFromDocument', () => {
    it('generates from a document', () => {
      const code = generateReactFromDocument(docWithVars)
      expect(code).toContain('Hello World')
    })
  })

  describe('generateHTMLCode', () => {
    it('generates HTML and CSS', () => {
      const { html, css } = generateHTMLCode([simpleFrame])
      expect(html).toContain('Hello World')
      expect(html).toContain('card')
      expect(css).toBeTruthy()
    })
  })

  describe('generateHTMLFromDocument', () => {
    it('generates from a document with CSS variables', () => {
      const { html, css } = generateHTMLFromDocument(docWithVars)
      expect(html).toContain('Hello World')
      expect(css).toBeTruthy()
    })
  })

  describe('generateCSSVariables', () => {
    it('generates CSS variables from document', () => {
      const css = generateCSSVariables(docWithVars)
      expect(css).toContain('--primary')
      expect(css).toContain('#3b82f6')
    })

    it('returns empty for document without variables', () => {
      const doc: PenDocument = { version: '1.0.0', children: [] }
      const css = generateCSSVariables(doc)
      expect(css).toContain('No design variables')
    })
  })
})
