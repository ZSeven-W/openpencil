import { defineEventHandler, getQuery, setResponseHeaders } from 'h3'
import simpleIconsData from '@iconify-json/simple-icons/icons.json'
import lucideData from '@iconify-json/lucide/icons.json'

interface IconResult {
  d: string
  style: 'stroke' | 'fill'
  width: number
  height: number
  iconId: string
}

type IconifySet = {
  width?: number
  height?: number
  icons: Record<string, { body: string; width?: number; height?: number }>
}

const simpleIcons = simpleIconsData as unknown as IconifySet
const lucideIcons = lucideData as unknown as IconifySet

// In-memory cache: normalized name → result (null = confirmed miss)
const iconCache = new Map<string, IconResult | null>()

/**
 * GET /api/ai/icon?name=google
 *
 * Resolves icon names to SVG path data using locally bundled icon sets.
 * Search order: lucide → simple-icons (brand icons)
 * No external network requests — instant, offline-capable.
 */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })

  const { name } = getQuery(event) as { name?: string }
  if (!name || typeof name !== 'string') {
    return { icon: null, error: 'Missing required query parameter: name' }
  }

  const normalizedName = name.trim().toLowerCase()
  if (!normalizedName) {
    return { icon: null, error: 'Empty icon name' }
  }

  if (iconCache.has(normalizedName)) {
    return { icon: iconCache.get(normalizedName) ?? null }
  }

  const result = resolveIcon(normalizedName)
  iconCache.set(normalizedName, result)
  return { icon: result }
})

function resolveIcon(name: string): IconResult | null {
  const kebab = toKebabCase(name)
  const candidates = kebab !== name ? [name, kebab] : [name]

  // 1. Try simple-icons first (brand/product icons).
  //    simple-icons only contains brand logos, so a hit here is unambiguously
  //    a brand — no risk of shadowing UI icon names like "search" or "home".
  for (const n of candidates) {
    const result = lookupLocal(simpleIcons, 'simple-icons', n)
    if (result) return result
  }

  // 2. Try Lucide (UI icons)
  for (const n of candidates) {
    const result = lookupLocal(lucideIcons, 'lucide', n)
    if (result) return result
  }

  return null
}

function lookupLocal(
  set: IconifySet,
  collection: string,
  iconName: string,
): IconResult | null {
  const icon = set.icons[iconName]
  if (!icon) return null
  const w = icon.width ?? set.width ?? 24
  const h = icon.height ?? set.height ?? 24
  return parseIconBody(icon.body, w, h, `${collection}:${iconName}`)
}

/**
 * Parse the SVG `body` field from Iconify into path data.
 * Extracts `d` from `<path>` elements and detects stroke vs fill style.
 */
function parseIconBody(
  body: string,
  width: number,
  height: number,
  iconId: string,
): IconResult | null {
  const pathRegex = /<path\s[^>]*?\bd="([^"]+)"[^>]*?\/?>/gi
  const paths: string[] = []
  let hasStroke = false
  let hasFill = false
  let match: RegExpExecArray | null

  while ((match = pathRegex.exec(body)) !== null) {
    paths.push(match[1])
    const tag = match[0]
    if (/\bstroke=/.test(tag) || /\bstroke-width=/.test(tag) || /\bstroke-linecap=/.test(tag)) {
      hasStroke = true
    }
    if (/\bfill="(?!none)[^"]*"/.test(tag)) {
      hasFill = true
    }
    if (/\bfill="none"/.test(tag)) {
      hasStroke = true
    }
  }

  if (paths.length === 0) return null

  // Check body-level stroke/fill attributes
  if (/\bstroke="currentColor"/.test(body) || /\bstroke-linecap=/.test(body)) {
    hasStroke = true
  }
  if (/\bfill="currentColor"/.test(body) && !/\bfill="none"/.test(body)) {
    hasFill = true
  }

  const d = paths.join(' ')
  const style: 'stroke' | 'fill' = hasStroke && !hasFill ? 'stroke' : 'fill'

  return { d, style, width, height, iconId }
}

/**
 * Convert concatenated lowercase to kebab-case for icon name matching.
 * e.g. "arrowright" → "arrow-right", "chevrondown" → "chevron-down"
 */
function toKebabCase(name: string): string {
  const prefixes = [
    'arrow', 'chevron', 'circle', 'alert', 'help',
    'external', 'bar', 'message', 'log',
  ]
  for (const prefix of prefixes) {
    if (name.startsWith(prefix) && name.length > prefix.length) {
      return `${prefix}-${name.slice(prefix.length)}`
    }
  }
  return name
}
