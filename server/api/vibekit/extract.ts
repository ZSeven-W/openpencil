/**
 * POST /api/vibekit/extract
 *
 * Fetches a URL, extracts CSS tokens (colors, fonts, spacing),
 * and maps them to VibeKit-compatible variable definitions.
 */
import { defineEventHandler, readBody, setResponseHeaders, createError } from 'h3'
import {
  extractTokensFromHTML,
  extractTokensFromCSS,
  mapTokensToVibeKit,
} from '../../utils/token-extractor'
import type { VariableDefinition } from '../../../src/types/variables'
import type { ExtractedTokens } from '../../utils/token-extractor'

interface CacheEntry {
  variables: Partial<Record<string, VariableDefinition>>
  tokens: ExtractedTokens
  timestamp: number
}

const CACHE_TTL = 60 * 60 * 1000 // 1 hour
const cache = new Map<string, CacheEntry>()

const LINK_CSS_RE = /<link[^>]+rel=["']stylesheet["'][^>]+href=["']([^"']+)["']/gi

/** Resolve a potentially relative URL against a base */
function resolveUrl(href: string, base: string): string {
  try {
    return new URL(href, base).href
  } catch {
    return href
  }
}

/** Fetch linked CSS stylesheets referenced in HTML */
async function fetchLinkedCSS(html: string, baseUrl: string): Promise<string> {
  const hrefs: string[] = []
  let match: RegExpExecArray | null
  const re = new RegExp(LINK_CSS_RE.source, 'gi')
  while ((match = re.exec(html)) !== null) {
    hrefs.push(resolveUrl(match[1], baseUrl))
  }

  // Fetch up to 5 stylesheets in parallel, ignore failures
  const sheets = await Promise.allSettled(
    hrefs.slice(0, 5).map(async (href) => {
      const res = await fetch(href, { signal: AbortSignal.timeout(5000) })
      if (!res.ok) return ''
      return res.text()
    }),
  )

  return sheets
    .filter((r): r is PromiseFulfilledResult<string> => r.status === 'fulfilled')
    .map((r) => r.value)
    .join('\n')
}

export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })

  const body = (await readBody(event)) as { url?: string } | null
  const url = body?.url
  if (!url || typeof url !== 'string') {
    throw createError({ statusCode: 400, statusMessage: 'Missing required field: url' })
  }

  // Check cache
  const cached = cache.get(url)
  if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
    return { variables: cached.variables, tokens: cached.tokens, cached: true }
  }

  // Fetch the page
  let html: string
  try {
    const res = await fetch(url, {
      signal: AbortSignal.timeout(10000),
      headers: { 'User-Agent': 'OpenPencil-VibeKit/1.0' },
    })
    if (!res.ok) {
      throw createError({ statusCode: 502, statusMessage: `Failed to fetch URL: ${res.status}` })
    }
    html = await res.text()
  } catch (err: unknown) {
    if (err && typeof err === 'object' && 'statusCode' in err) throw err
    throw createError({ statusCode: 502, statusMessage: `Fetch error: ${(err as Error).message}` })
  }

  // Extract tokens from inline HTML styles
  const htmlTokens = extractTokensFromHTML(html)

  // Fetch and extract tokens from linked CSS
  const linkedCSS = await fetchLinkedCSS(html, url)
  const cssTokens = linkedCSS ? extractTokensFromCSS(linkedCSS) : null

  // Merge tokens
  const tokens: ExtractedTokens = cssTokens
    ? {
        colors: { ...htmlTokens.colors, ...cssTokens.colors },
        fonts: Array.from(new Set([...htmlTokens.fonts, ...cssTokens.fonts])),
        spacing: Array.from(new Set([...htmlTokens.spacing, ...cssTokens.spacing])).sort((a, b) => a - b),
        borderRadius: Array.from(new Set([...htmlTokens.borderRadius, ...cssTokens.borderRadius])).sort((a, b) => a - b),
        fontSizes: Array.from(new Set([...htmlTokens.fontSizes, ...cssTokens.fontSizes])).sort((a, b) => a - b),
      }
    : htmlTokens

  // Map to variable definitions
  const variables = mapTokensToVibeKit(tokens, url)

  // Cache result
  cache.set(url, { variables, tokens, timestamp: Date.now() })

  return { variables, tokens, cached: false }
})
