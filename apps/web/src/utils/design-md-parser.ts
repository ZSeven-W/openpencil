import type { DesignMdSpec, DesignMdColor, DesignMdTypography } from '@/types/design-md'
import type { PenDocument } from '@/types/pen'
import type { VariableDefinition } from '@/types/variables'

// ---------------------------------------------------------------------------
// Section header patterns (fuzzy matching)
// ---------------------------------------------------------------------------

const SECTION_PATTERNS: Record<string, RegExp> = {
  visualTheme: /visual\s*theme|atmosphere|mood/i,
  colorPalette: /colou?r\s*(palette|roles?|system)?/i,
  typography: /typography|type\s*rules?|fonts?/i,
  componentStyles: /component\s*(styl|pattern)|button|card|input/i,
  layoutPrinciples: /layout\s*(principle|rule|system)?|grid|whitespace/i,
  generationNotes: /generation\s*notes?|design\s*system\s*notes?|stitch|prompting/i,
}

interface ParsedSection {
  key: string
  title: string
  content: string
}

// ---------------------------------------------------------------------------
// Parse markdown into sections
// ---------------------------------------------------------------------------

function splitSections(markdown: string): { projectName?: string; sections: ParsedSection[] } {
  const lines = markdown.split('\n')
  let projectName: string | undefined

  // Extract project name from H1
  const h1Match = markdown.match(/^#\s+(?:Design\s+System:\s*)?(.+)$/m)
  if (h1Match) projectName = h1Match[1].trim()

  const sections: ParsedSection[] = []
  let currentTitle = ''
  let currentLines: string[] = []
  let currentKey = ''

  for (const line of lines) {
    const h2Match = line.match(/^##\s+(?:\d+\.\s*)?(.+)$/)
    if (h2Match) {
      // Flush previous section
      if (currentKey) {
        sections.push({ key: currentKey, title: currentTitle, content: currentLines.join('\n').trim() })
      }
      currentTitle = h2Match[1].trim()
      currentLines = []
      currentKey = matchSectionKey(currentTitle)
    } else {
      currentLines.push(line)
    }
  }

  // Flush last section
  if (currentKey) {
    sections.push({ key: currentKey, title: currentTitle, content: currentLines.join('\n').trim() })
  }

  return { projectName, sections }
}

function matchSectionKey(title: string): string {
  for (const [key, pattern] of Object.entries(SECTION_PATTERNS)) {
    if (pattern.test(title)) return key
  }
  // Fallback: try to match common section names loosely
  const lower = title.toLowerCase()
  if (lower.includes('theme') || lower.includes('vibe') || lower.includes('aesthetic')) return 'visualTheme'
  if (lower.includes('color') || lower.includes('palette') || lower.includes('colour')) return 'colorPalette'
  if (lower.includes('type') || lower.includes('font') || lower.includes('typo')) return 'typography'
  if (lower.includes('component') || lower.includes('style') || lower.includes('element')) return 'componentStyles'
  if (lower.includes('layout') || lower.includes('spacing') || lower.includes('grid')) return 'layoutPrinciples'
  if (lower.includes('note') || lower.includes('generation') || lower.includes('usage')) return 'generationNotes'
  return 'unknown'
}

// ---------------------------------------------------------------------------
// Color extraction
// ---------------------------------------------------------------------------

const HEX_COLOR_RE = /#([0-9A-Fa-f]{6})\b/

function parseColors(content: string): DesignMdColor[] {
  const colors: DesignMdColor[] = []
  const lines = content.split('\n')

  for (const line of lines) {
    // Skip JSON/code artifacts
    if (line.trimStart().startsWith('{') || line.trimStart().startsWith('`')) continue

    const hexMatch = line.match(HEX_COLOR_RE)
    if (!hexMatch) continue

    const hex = `#${hexMatch[1].toUpperCase()}`

    // Try pattern: **Name** (hex) — role  OR  - **Name** (#hex) – role
    const namedMatch = line.match(/\*\*([^*]+)\*\*\s*\(?#?[0-9A-Fa-f]{6}\)?\s*[–—-]\s*(.+)/)
    if (namedMatch) {
      colors.push({ name: namedMatch[1].trim(), hex, role: namedMatch[2].trim() })
      continue
    }

    // Try pattern: - Name (#hex) — role  OR  Name (#hex): role
    const dashMatch = line.match(/[-*]\s*(.+?)\s*\(#?[0-9A-Fa-f]{6}\)\s*[–—:-]\s*(.+)/)
    if (dashMatch) {
      colors.push({ name: dashMatch[1].trim(), hex, role: dashMatch[2].trim() })
      continue
    }

    // Fallback: just grab the hex and surrounding text
    const before = line.substring(0, hexMatch.index).replace(/[-*#\s]+$/, '').trim()
    const after = line.substring((hexMatch.index ?? 0) + hexMatch[0].length).replace(/^[)\s–—:-]+/, '').trim()
    colors.push({
      name: before || hex,
      hex,
      role: after || '',
    })
  }

  return colors
}

// ---------------------------------------------------------------------------
// Typography extraction
// ---------------------------------------------------------------------------

function parseTypography(content: string): DesignMdTypography {
  const typo: DesignMdTypography = {}

  // Font family
  const fontMatch = content.match(/(?:font\s*family|primary\s*font)[:\s]*\*?\*?([^*\n]+)/i)
  if (fontMatch) typo.fontFamily = fontMatch[1].trim()

  // Try to extract heading/body descriptions
  const headingMatch = content.match(/(?:heading|display|h1)[^:]*:\s*([^\n]+)/i)
  if (headingMatch) typo.headings = headingMatch[1].trim()

  const bodyMatch = content.match(/(?:body\s*text|paragraph)[^:]*:\s*([^\n]+)/i)
  if (bodyMatch) typo.body = bodyMatch[1].trim()

  // Keep full content as scale description
  typo.scale = content

  return typo
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Parse a design.md markdown string into a structured DesignMdSpec. */
export function parseDesignMd(markdown: string): DesignMdSpec {
  const { projectName, sections } = splitSections(markdown)

  const spec: DesignMdSpec = { raw: markdown, projectName }

  for (const section of sections) {
    switch (section.key) {
      case 'visualTheme':
        spec.visualTheme = section.content
        break
      case 'colorPalette':
        spec.colorPalette = parseColors(section.content)
        break
      case 'typography':
        spec.typography = parseTypography(section.content)
        break
      case 'componentStyles':
        spec.componentStyles = section.content
        break
      case 'layoutPrinciples':
        spec.layoutPrinciples = section.content
        break
      case 'generationNotes':
        spec.generationNotes = section.content
        break
      case 'unknown':
        // Append to componentStyles as catch-all
        spec.componentStyles = spec.componentStyles
          ? spec.componentStyles + '\n\n' + section.content
          : section.content
        break
    }
  }

  // Fallback: if no sections were parsed at all, try to extract colors
  // and typography from the entire markdown
  if (!spec.colorPalette || spec.colorPalette.length === 0) {
    const colors = parseColors(markdown)
    if (colors.length > 0) spec.colorPalette = colors
  }
  if (!spec.typography) {
    const typo = parseTypography(markdown)
    if (typo.fontFamily) spec.typography = typo
  }
  // If still nothing parsed, store the whole text as visual theme
  if (!spec.visualTheme && !spec.colorPalette && !spec.componentStyles && sections.length === 0) {
    spec.visualTheme = markdown.trim()
  }

  return spec
}

/** Generate a design.md markdown string from a DesignMdSpec. */
export function generateDesignMd(spec: DesignMdSpec): string {
  // If raw was set and nothing was structurally changed, return as-is
  if (spec.raw) return spec.raw

  const lines: string[] = []

  lines.push(`# Design System: ${spec.projectName ?? 'Untitled'}`)
  lines.push('')

  if (spec.visualTheme) {
    lines.push('## 1. Visual Theme & Atmosphere')
    lines.push(spec.visualTheme)
    lines.push('')
  }

  if (spec.colorPalette?.length) {
    lines.push('## 2. Color Palette & Roles')
    lines.push('')
    for (const c of spec.colorPalette) {
      lines.push(`- **${c.name}** (${c.hex}) — ${c.role}`)
    }
    lines.push('')
  }

  if (spec.typography) {
    lines.push('## 3. Typography Rules')
    if (spec.typography.fontFamily) {
      lines.push(`**Primary Font Family:** ${spec.typography.fontFamily}`)
    }
    if (spec.typography.scale) {
      lines.push(spec.typography.scale)
    }
    lines.push('')
  }

  if (spec.componentStyles) {
    lines.push('## 4. Component Stylings')
    lines.push(spec.componentStyles)
    lines.push('')
  }

  if (spec.layoutPrinciples) {
    lines.push('## 5. Layout Principles')
    lines.push(spec.layoutPrinciples)
    lines.push('')
  }

  if (spec.generationNotes) {
    lines.push('## 6. Design System Notes')
    lines.push(spec.generationNotes)
    lines.push('')
  }

  return lines.join('\n')
}

/** Auto-extract a DesignMdSpec from an existing PenDocument. */
export function extractDesignMdFromDocument(doc: PenDocument): DesignMdSpec {
  const colors: DesignMdColor[] = []

  // Extract colors from document variables
  if (doc.variables) {
    for (const [name, def] of Object.entries(doc.variables)) {
      if (def.type !== 'color') continue
      const value = typeof def.value === 'string' ? def.value
        : Array.isArray(def.value) ? String(def.value[0]?.value ?? '')
        : String(def.value)
      if (/^#[0-9A-Fa-f]{6,8}$/.test(value)) {
        colors.push({
          name: name.replace(/^[$]/, ''),
          hex: value.substring(0, 7).toUpperCase(),
          role: `Design variable $${name}`,
        })
      }
    }
  }

  // Collect font families from text nodes
  const fonts = new Set<string>()
  const collectFonts = (nodes: { fontFamily?: string; children?: unknown[] }[]) => {
    for (const n of nodes) {
      if ('fontFamily' in n && typeof n.fontFamily === 'string') fonts.add(n.fontFamily)
      if ('children' in n && Array.isArray(n.children)) collectFonts(n.children as { fontFamily?: string; children?: unknown[] }[])
    }
  }
  collectFonts(doc.children as { fontFamily?: string; children?: unknown[] }[])
  if (doc.pages) {
    for (const page of doc.pages) {
      collectFonts(page.children as { fontFamily?: string; children?: unknown[] }[])
    }
  }

  const typography: DesignMdTypography = {}
  if (fonts.size > 0) typography.fontFamily = [...fonts].join(', ')

  const spec: DesignMdSpec = {
    raw: '',
    projectName: doc.name ?? 'Untitled',
    colorPalette: colors.length > 0 ? colors : undefined,
    typography: fonts.size > 0 ? typography : undefined,
  }

  // Generate the markdown and set as raw
  spec.raw = generateDesignMd(spec)

  return spec
}

/** Convert design.md colors to document variables. */
export function designMdColorsToVariables(
  colors: DesignMdColor[],
): Record<string, VariableDefinition> {
  const vars: Record<string, VariableDefinition> = {}
  for (const color of colors) {
    const key = color.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
    vars[key] = { type: 'color', value: color.hex }
  }
  return vars
}
