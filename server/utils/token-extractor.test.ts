import { describe, it, expect } from 'vitest'
import { extractTokensFromCSS, extractTokensFromHTML, mapTokensToVibeKit } from './token-extractor'

describe('extractTokensFromCSS', () => {
  it('extracts CSS custom properties', () => {
    const css = `
      :root {
        --primary: #3b82f6;
        --secondary: #8b5cf6;
        --bg: #ffffff;
      }
    `
    const tokens = extractTokensFromCSS(css)
    expect(tokens.colors['primary']).toBe('#3b82f6')
    expect(tokens.colors['secondary']).toBe('#8b5cf6')
    expect(tokens.colors['bg']).toBe('#ffffff')
  })

  it('extracts font-family declarations', () => {
    const css = `
      body { font-family: Inter, sans-serif; }
      h1 { font-family: "Playfair Display", serif; }
    `
    const tokens = extractTokensFromCSS(css)
    expect(tokens.fonts.length).toBeGreaterThanOrEqual(1)
    expect(tokens.fonts.some((f) => f.includes('Inter') || f.includes('Playfair'))).toBe(true)
  })

  it('extracts font sizes in px', () => {
    const css = `
      h1 { font-size: 48px; }
      p { font-size: 16px; }
    `
    const tokens = extractTokensFromCSS(css)
    expect(tokens.fontSizes).toContain(48)
    expect(tokens.fontSizes).toContain(16)
  })

  it('extracts spacing values', () => {
    const css = `
      .container { padding: 24px; margin: 16px; gap: 8px; }
    `
    const tokens = extractTokensFromCSS(css)
    expect(tokens.spacing).toContain(24)
    expect(tokens.spacing).toContain(16)
    expect(tokens.spacing).toContain(8)
  })

  it('extracts border-radius values', () => {
    const css = `
      .card { border-radius: 12px; }
    `
    const tokens = extractTokensFromCSS(css)
    expect(tokens.borderRadius).toContain(12)
  })
})

describe('extractTokensFromHTML', () => {
  it('extracts from inline style blocks', () => {
    const html = `
      <html>
        <head>
          <style>
            :root { --primary: #ff0000; }
            body { font-family: Georgia, serif; }
          </style>
        </head>
        <body></body>
      </html>
    `
    const tokens = extractTokensFromHTML(html)
    expect(tokens.colors['primary']).toBe('#ff0000')
    expect(tokens.fonts.some((f) => f.includes('Georgia'))).toBe(true)
  })

  it('extracts from style attributes', () => {
    const html = `
      <div style="color: #333; font-size: 14px; padding: 20px;"></div>
    `
    const tokens = extractTokensFromHTML(html)
    expect(tokens.fontSizes).toContain(14)
    expect(tokens.spacing).toContain(20)
  })
})

describe('mapTokensToVibeKit', () => {
  it('maps colors to semantic variable names', () => {
    const tokens = extractTokensFromCSS(`
      :root {
        --primary: #2563eb;
        --text: #111827;
        --bg: #ffffff;
      }
    `)
    const vars = mapTokensToVibeKit(tokens, 'https://example.com')
    // Should have mapped at least some color variables
    const colorKeys = Object.keys(vars).filter((k) => k.startsWith('color-'))
    expect(colorKeys.length).toBeGreaterThan(0)
  })

  it('maps fonts to font variables', () => {
    const tokens = extractTokensFromCSS(`
      body { font-family: Inter, sans-serif; }
      h1 { font-family: Poppins, sans-serif; }
    `)
    const vars = mapTokensToVibeKit(tokens, 'https://example.com')
    const fontKeys = Object.keys(vars).filter((k) => k.startsWith('font-'))
    expect(fontKeys.length).toBeGreaterThan(0)
  })

  it('returns VariableDefinition objects with correct types', () => {
    const tokens = extractTokensFromCSS(`
      :root { --primary: #ff0000; }
      body { font-family: Arial; font-size: 16px; }
    `)
    const vars = mapTokensToVibeKit(tokens, 'https://example.com')
    for (const [, def] of Object.entries(vars)) {
      expect(def).toBeDefined()
      expect(['color', 'number', 'string', 'boolean']).toContain(def!.type)
      expect(def!.value).toBeDefined()
    }
  })
})
