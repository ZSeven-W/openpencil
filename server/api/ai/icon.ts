import { defineEventHandler, getQuery, setResponseHeaders } from 'h3'

interface IconResult {
  d: string
  style: 'stroke' | 'fill'
  width: number
  height: number
}

// In-memory cache: icon name → result (null = confirmed miss)
const iconCache = new Map<string, IconResult | null>()

// Iconify collection search order
const COLLECTIONS = ['lucide', 'simple-icons', 'mdi'] as const

/**
 * GET /api/ai/icon?name=google
 *
 * Server-side Iconify proxy with in-memory cache.
 * Searches collections in order: lucide → simple-icons → mdi,
 * then falls back to Iconify search API.
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

  // Check cache first (includes cached misses)
  if (iconCache.has(normalizedName)) {
    return { icon: iconCache.get(normalizedName) ?? null }
  }

  try {
    const result = await resolveIcon(normalizedName)
    iconCache.set(normalizedName, result)
    return { icon: result }
  } catch {
    // Cache misses to avoid repeated failures
    iconCache.set(normalizedName, null)
    return { icon: null }
  }
})

async function resolveIcon(name: string): Promise<IconResult | null> {
  // 1. Try direct lookup in each collection
  for (const collection of COLLECTIONS) {
    const result = await tryDirectLookup(collection, name)
    if (result) return result
  }

  // 2. Try kebab-case variant (e.g. "arrowright" → "arrow-right")
  const kebab = toKebabCase(name)
  if (kebab !== name) {
    for (const collection of COLLECTIONS) {
      const result = await tryDirectLookup(collection, kebab)
      if (result) return result
    }
  }

  // 3. Fall back to Iconify search API
  return trySearchApi(name)
}

async function tryDirectLookup(
  collection: string,
  iconName: string,
): Promise<IconResult | null> {
  const url = `https://api.iconify.design/${collection}/${iconName}.json`
  try {
    const res = await fetch(url)
    if (!res.ok) return null
    const data = (await res.json()) as { body?: string; width?: number; height?: number }
    if (!data.body) return null
    return parseIconBody(data.body, data.width ?? 24, data.height ?? 24)
  } catch {
    return null
  }
}

async function trySearchApi(query: string): Promise<IconResult | null> {
  const url = `https://api.iconify.design/search?query=${encodeURIComponent(query)}&limit=3`
  try {
    const res = await fetch(url)
    if (!res.ok) return null
    const data = (await res.json()) as { icons?: string[] }
    if (!data.icons || data.icons.length === 0) return null

    // Each result is "collection:name" format
    for (const fullName of data.icons) {
      const [collection, iconName] = fullName.split(':')
      if (!collection || !iconName) continue
      const result = await tryDirectLookup(collection, iconName)
      if (result) return result
    }
    return null
  } catch {
    return null
  }
}

/**
 * Parse the SVG `body` field from Iconify into path data.
 * Extracts `d` from `<path>` elements and detects stroke vs fill style.
 */
function parseIconBody(
  body: string,
  width: number,
  height: number,
): IconResult | null {
  // Extract all path d attributes
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

  if (paths.length === 0) {
    // Try extracting from <circle>, <rect>, <line> by returning the raw body
    // for basic shapes — but we only support <path> for now
    return null
  }

  // Also check body-level stroke/fill attributes
  if (/\bstroke="currentColor"/.test(body) || /\bstroke-linecap=/.test(body)) {
    hasStroke = true
  }
  if (/\bfill="currentColor"/.test(body) && !/\bfill="none"/.test(body)) {
    hasFill = true
  }

  const d = paths.join(' ')
  const style: 'stroke' | 'fill' = hasStroke && !hasFill ? 'stroke' : 'fill'

  return { d, style, width, height }
}

/**
 * Convert concatenated lowercase to kebab-case.
 * e.g. "arrowright" → "arrow-right", "chevrondown" → "chevron-down"
 */
function toKebabCase(name: string): string {
  // Common prefixes in icon naming
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
