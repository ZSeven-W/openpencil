import type { TypefaceFontProvider, CanvasKit } from 'canvaskit-wasm'

/**
 * Bundled font files served from /fonts/ (no external CDN dependency).
 * Key = lowercase family name, values = local URLs.
 */
const BUNDLED_FONTS: Record<string, string[]> = {
  inter: [
    '/fonts/inter-400.woff2',
    '/fonts/inter-500.woff2',
    '/fonts/inter-600.woff2',
    '/fonts/inter-700.woff2',
    '/fonts/inter-ext-400.woff2',
    '/fonts/inter-ext-500.woff2',
    '/fonts/inter-ext-600.woff2',
    '/fonts/inter-ext-700.woff2',
  ],
  poppins: [
    '/fonts/poppins-400.woff2',
    '/fonts/poppins-500.woff2',
    '/fonts/poppins-600.woff2',
    '/fonts/poppins-700.woff2',
  ],
  roboto: [
    '/fonts/roboto-400.woff2',
    '/fonts/roboto-500.woff2',
    '/fonts/roboto-700.woff2',
  ],
  montserrat: [
    '/fonts/montserrat-400.woff2',
    '/fonts/montserrat-500.woff2',
    '/fonts/montserrat-600.woff2',
    '/fonts/montserrat-700.woff2',
  ],
  'open sans': [
    '/fonts/open-sans-400.woff2',
    '/fonts/open-sans-600.woff2',
    '/fonts/open-sans-700.woff2',
  ],
  lato: [
    '/fonts/lato-400.woff2',
    '/fonts/lato-700.woff2',
  ],
  raleway: [
    '/fonts/raleway-400.woff2',
    '/fonts/raleway-500.woff2',
    '/fonts/raleway-600.woff2',
    '/fonts/raleway-700.woff2',
  ],
  'dm sans': [
    '/fonts/dm-sans-400.woff2',
    '/fonts/dm-sans-500.woff2',
    '/fonts/dm-sans-700.woff2',
  ],
  'playfair display': [
    '/fonts/playfair-display-400.woff2',
    '/fonts/playfair-display-700.woff2',
  ],
  nunito: [
    '/fonts/nunito-400.woff2',
    '/fonts/nunito-600.woff2',
    '/fonts/nunito-700.woff2',
  ],
  'source sans 3': [
    '/fonts/source-sans-3-400.woff2',
    '/fonts/source-sans-3-600.woff2',
    '/fonts/source-sans-3-700.woff2',
  ],
  'source sans pro': [
    '/fonts/source-sans-3-400.woff2',
    '/fonts/source-sans-3-600.woff2',
    '/fonts/source-sans-3-700.woff2',
  ],
  'noto sans sc': [
    '/fonts/noto-sans-sc-400.woff2',
    '/fonts/noto-sans-sc-700.woff2',
    '/fonts/noto-sans-sc-latin-400.woff2',
    '/fonts/noto-sans-sc-latin-700.woff2',
  ],
}

/** List of all bundled font family names (for UI font picker) */
export const BUNDLED_FONT_FAMILIES = [
  'Inter',
  'Noto Sans SC',
  'Poppins',
  'Roboto',
  'Montserrat',
  'Open Sans',
  'Lato',
  'Raleway',
  'DM Sans',
  'Playfair Display',
  'Nunito',
  'Source Sans 3',
]

/**
 * Manages font loading for CanvasKit's Paragraph API (vector text rendering).
 *
 * Fonts are loaded from bundled /fonts/ directory first, falling back to
 * Google Fonts CDN. Once loaded, text is rendered as true vector glyphs.
 */
export class SkiaFontManager {
  private provider: TypefaceFontProvider
  /** Registered family names (lowercase) → true once loaded */
  private loadedFamilies = new Set<string>()
  /** Font families that failed to load — prevents repeated fetch attempts */
  private failedFamilies = new Set<string>()
  /** In-flight font fetch promises to avoid duplicate requests */
  private pendingFetches = new Map<string, Promise<boolean>>()

  constructor(ck: CanvasKit) {
    this.provider = ck.TypefaceFontProvider.Make()
  }

  getProvider(): TypefaceFontProvider {
    return this.provider
  }

  /** Check if a font family is ready for use */
  isFontReady(family: string): boolean {
    return this.loadedFamilies.has(family.toLowerCase())
  }

  /** Check if a font family is bundled (available offline) */
  isBundled(family: string): boolean {
    return family.toLowerCase() in BUNDLED_FONTS
  }

  /**
   * Build a font fallback chain for the Paragraph API.
   * Only includes fonts actually registered in the TypefaceFontProvider.
   * Extended subsets (e.g. "Inter Ext") are added for per-glyph fallback
   * so characters like ₦ (U+20A6) render correctly.
   */
  getFallbackChain(primaryFamily: string): string[] {
    const chain: string[] = []
    const lower = primaryFamily.toLowerCase()
    // Only add primary if it's actually registered
    if (this.loadedFamilies.has(lower)) {
      chain.push(primaryFamily)
    }
    // Add ext subset of primary family if available
    if (this.loadedFamilies.has(lower + ' ext')) {
      chain.push(primaryFamily + ' Ext')
    }
    // Add Noto Sans SC for CJK glyph fallback (bundled, works offline)
    if (lower !== 'noto sans sc' && this.loadedFamilies.has('noto sans sc')) {
      chain.push('Noto Sans SC')
    }
    // Add Inter + Inter Ext as final fallback for Latin glyphs
    if (lower !== 'inter') {
      if (this.loadedFamilies.has('inter')) chain.push('Inter')
      if (this.loadedFamilies.has('inter ext')) chain.push('Inter Ext')
    }
    // Must have at least one font
    if (chain.length === 0) chain.push('Inter')
    return chain
  }

  /**
   * Check if there's at least one loaded fallback font for the given primary family.
   * Used to decide whether vector rendering can proceed when the primary font is unavailable.
   */
  hasAnyFallback(primaryFamily: string): boolean {
    const key = primaryFamily.toLowerCase()
    if (key === 'inter' || key === 'noto sans sc') return false
    return this.loadedFamilies.has('inter') || this.loadedFamilies.has('noto sans sc')
  }

  /** Register a font from raw ArrayBuffer data */
  registerFont(data: ArrayBuffer, familyName: string): boolean {
    try {
      this.provider.registerFont(data, familyName)
      this.loadedFamilies.add(familyName.toLowerCase())
      console.log(`[FontManager] Registered "${familyName}" (${(data.byteLength / 1024).toFixed(1)}KB)`)
      return true
    } catch (e) {
      console.warn(`[FontManager] Failed to register "${familyName}":`, e)
      return false
    }
  }

  /**
   * Ensure a font family is loaded. Tries bundled fonts first, then Google Fonts.
   */
  async ensureFont(family: string, weights: number[] = [400, 500, 600, 700]): Promise<boolean> {
    const key = family.toLowerCase()
    if (this.loadedFamilies.has(key)) return true
    if (this.failedFamilies.has(key)) return false

    const existing = this.pendingFetches.get(key)
    if (existing) return existing

    const promise = this._loadFont(family, weights)
    this.pendingFetches.set(key, promise)
    const result = await promise
    this.pendingFetches.delete(key)
    if (!result) {
      this.failedFamilies.add(key)
      console.warn(`[FontManager] Font "${family}" unavailable, will not retry`)
    }
    return result
  }

  /**
   * Load multiple font families concurrently.
   */
  async ensureFonts(families: string[]): Promise<Set<string>> {
    const unique = [...new Set(families.map(f => f.trim()).filter(Boolean))]
    const results = await Promise.allSettled(
      unique.map(f => this.ensureFont(f))
    )
    const loaded = new Set<string>()
    results.forEach((r, i) => {
      if (r.status === 'fulfilled' && r.value) loaded.add(unique[i])
    })
    return loaded
  }

  private async _loadFont(family: string, weights: number[]): Promise<boolean> {
    // 1. Try bundled fonts first (no network dependency)
    const bundled = BUNDLED_FONTS[family.toLowerCase()]
    if (bundled) {
      const ok = await this._fetchLocalFonts(family, bundled)
      if (ok) return true
    }

    // 2. Skip Google Fonts for system/proprietary fonts that won't exist there
    if (isSystemFont(family)) {
      console.log(`[FontManager] "${family}" is a system font, skipping Google Fonts`)
      return false
    }

    // 3. Fall back to Google Fonts CDN
    return this._fetchGoogleFont(family, weights)
  }

  private async _fetchLocalFonts(family: string, urls: string[]): Promise<boolean> {
    try {
      const buffers = await Promise.all(
        urls.map(async (url) => {
          const resp = await fetch(url)
          if (!resp.ok) {
            console.warn(`[FontManager] Failed to fetch ${url}: ${resp.status}`)
            return null
          }
          return resp.arrayBuffer()
        })
      )
      let registered = 0
      for (let i = 0; i < buffers.length; i++) {
        const buf = buffers[i]
        if (!buf) continue
        // Register extended subset files (e.g. inter-ext-400.woff2) under a separate
        // family name so CanvasKit's Paragraph API can do per-glyph fallback.
        // Base Inter doesn't have ₦ (U+20A6) but latin-ext does.
        const regName = urls[i].includes('-ext-') ? family + ' Ext' : family
        if (this.registerFont(buf, regName)) registered++
      }
      console.log(`[FontManager] Local fonts for "${family}": ${registered}/${urls.length} registered`)
      return registered > 0
    } catch (e) {
      console.warn(`[FontManager] Local font fetch error for "${family}":`, e)
      return false
    }
  }

  /**
   * Fetch a font from Google Fonts CDN with China mirror fallback.
   * Tries Google Fonts first (3s timeout), then falls back to loli.net mirror
   * which is accessible in China where Google services are blocked.
   */
  private async _fetchGoogleFont(family: string, weights: number[]): Promise<boolean> {
    const weightStr = weights.join(';')
    const encodedFamily = encodeURIComponent(family)
    const query = `family=${encodedFamily}:wght@${weightStr}&display=swap`

    // Try Google Fonts first, then China-accessible mirrors
    const cdnConfigs = [
      {
        cssBase: 'https://fonts.googleapis.com/css2',
        fontUrlPattern: /url\((https:\/\/fonts\.gstatic\.com\/[^)]+\.woff2)\)/g,
      },
      {
        cssBase: 'https://fonts.font.im/css2',
        fontUrlPattern: /url\((https?:\/\/[^)]+\.woff2)\)/g,
      },
    ]

    for (const cdn of cdnConfigs) {
      try {
        const cssUrl = `${cdn.cssBase}?${query}`
        const cssResp = await fetchWithTimeout(cssUrl, 4000)
        if (!cssResp.ok) continue
        const css = await cssResp.text()

        const urls: string[] = []
        let match: RegExpExecArray | null
        while ((match = cdn.fontUrlPattern.exec(css)) !== null) {
          urls.push(match[1])
        }
        if (urls.length === 0) continue

        const fontBuffers = await Promise.all(
          urls.map(async (url) => {
            try {
              const resp = await fetchWithTimeout(url, 8000)
              return resp.ok ? resp.arrayBuffer() : null
            } catch { return null }
          })
        )

        let registered = 0
        for (const buf of fontBuffers) {
          if (buf && this.registerFont(buf, family)) registered++
        }
        if (registered > 0) return true
      } catch {
        // CDN failed, try next
      }
    }
    return false
  }

  dispose() {
    this.provider.delete()
    this.loadedFamilies.clear()
    this.failedFamilies.clear()
    this.pendingFetches.clear()
  }
}

/**
 * Known system/proprietary fonts that are NOT on Google Fonts.
 * Avoids pointless 400 requests and CORS errors.
 */
const SYSTEM_FONT_PATTERNS = [
  // Apple
  'pingfang', 'sf pro', 'sf mono', 'sf compact', 'helvetica neue', 'helvetica',
  'apple sd gothic', 'hiragino',
  // Microsoft
  'microsoft yahei', 'segoe ui', 'consolas', 'arial',
  // CJK
  'noto sans cjk', 'youshebiaotihei', 'simhei', 'simsun', 'fangsong', 'kaiti',
  // Proprietary / non-Google
  'd-din', 'din pro', 'din-pro', 'avenir', 'futura', 'proxima nova', 'gotham',
  'brandon grotesque', 'aktiv grotesk', 'circular',
]

function isSystemFont(family: string): boolean {
  const lower = family.toLowerCase()
  return SYSTEM_FONT_PATTERNS.some(p => lower.includes(p))
}

/** Fetch with timeout — rejects if response doesn't arrive within `ms`. */
function fetchWithTimeout(url: string, ms: number): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), ms)
  return fetch(url, { signal: controller.signal }).finally(() => clearTimeout(timer))
}
