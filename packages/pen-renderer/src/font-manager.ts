import type { TypefaceFontProvider, CanvasKit } from 'canvaskit-wasm'

export interface FontManagerOptions {
  /** Base URL for bundled font files. Default: '/fonts/' */
  fontBasePath?: string
  /** Custom Google Fonts CSS endpoint. Default: 'https://fonts.googleapis.com/css2' */
  googleFontsCssUrl?: string
}

/**
 * Bundled font files (relative paths, prepended with fontBasePath at load time).
 * Key = lowercase family name, values = relative file names.
 */
const BUNDLED_FONTS: Record<string, string[]> = {
  inter: [
    'inter-400.woff2',
    'inter-500.woff2',
    'inter-600.woff2',
    'inter-700.woff2',
    'inter-ext-400.woff2',
    'inter-ext-500.woff2',
    'inter-ext-600.woff2',
    'inter-ext-700.woff2',
  ],
  poppins: [
    'poppins-400.woff2',
    'poppins-500.woff2',
    'poppins-600.woff2',
    'poppins-700.woff2',
  ],
  roboto: [
    'roboto-400.woff2',
    'roboto-500.woff2',
    'roboto-700.woff2',
  ],
  montserrat: [
    'montserrat-400.woff2',
    'montserrat-500.woff2',
    'montserrat-600.woff2',
    'montserrat-700.woff2',
  ],
  'open sans': [
    'open-sans-400.woff2',
    'open-sans-600.woff2',
    'open-sans-700.woff2',
  ],
  lato: [
    'lato-400.woff2',
    'lato-700.woff2',
  ],
  raleway: [
    'raleway-400.woff2',
    'raleway-500.woff2',
    'raleway-600.woff2',
    'raleway-700.woff2',
  ],
  'dm sans': [
    'dm-sans-400.woff2',
    'dm-sans-500.woff2',
    'dm-sans-700.woff2',
  ],
  'playfair display': [
    'playfair-display-400.woff2',
    'playfair-display-700.woff2',
  ],
  nunito: [
    'nunito-400.woff2',
    'nunito-600.woff2',
    'nunito-700.woff2',
  ],
  'source sans 3': [
    'source-sans-3-400.woff2',
    'source-sans-3-600.woff2',
    'source-sans-3-700.woff2',
  ],
  'source sans pro': [
    'source-sans-3-400.woff2',
    'source-sans-3-600.woff2',
    'source-sans-3-700.woff2',
  ],
  'noto sans sc': [
    'noto-sans-sc-400.woff2',
    'noto-sans-sc-700.woff2',
    'noto-sans-sc-latin-400.woff2',
    'noto-sans-sc-latin-700.woff2',
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
 * Fonts are loaded from a configurable base path first, falling back to
 * Google Fonts CDN. Once loaded, text is rendered as true vector glyphs.
 */
export class SkiaFontManager {
  private provider: TypefaceFontProvider
  private fontBasePath: string
  private googleFontsCssUrl: string
  /** Registered family names (lowercase) -> true once loaded */
  private loadedFamilies = new Set<string>()
  /** Font families that failed to load */
  private failedFamilies = new Set<string>()
  /** System fonts that render via bitmap */
  private systemFontFamilies = new Set<string>()
  /** In-flight font fetch promises to avoid duplicate requests */
  private pendingFetches = new Map<string, Promise<boolean>>()

  constructor(ck: CanvasKit, options?: FontManagerOptions) {
    this.provider = ck.TypefaceFontProvider.Make()
    this.fontBasePath = options?.fontBasePath ?? '/fonts/'
    // Ensure trailing slash
    if (!this.fontBasePath.endsWith('/')) this.fontBasePath += '/'
    this.googleFontsCssUrl = options?.googleFontsCssUrl ?? 'https://fonts.googleapis.com/css2'
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

  /** Check if a font is a system font that should use bitmap rendering */
  isSystemFont(family: string): boolean {
    return this.systemFontFamilies.has(family.toLowerCase()) || isSystemFont(family)
  }

  /**
   * Build a font fallback chain for the Paragraph API.
   * Only includes fonts actually registered in the TypefaceFontProvider.
   */
  getFallbackChain(primaryFamily: string): string[] {
    const chain: string[] = []
    const lower = primaryFamily.toLowerCase()
    if (this.loadedFamilies.has(lower)) {
      chain.push(primaryFamily)
    }
    if (this.loadedFamilies.has(lower + ' ext')) {
      chain.push(primaryFamily + ' Ext')
    }
    if (lower !== 'noto sans sc' && this.loadedFamilies.has('noto sans sc')) {
      chain.push('Noto Sans SC')
    }
    if (lower !== 'inter') {
      if (this.loadedFamilies.has('inter')) chain.push('Inter')
      if (this.loadedFamilies.has('inter ext')) chain.push('Inter Ext')
    }
    if (chain.length === 0) chain.push('Inter')
    return chain
  }

  /**
   * Check if there's at least one loaded fallback font for the given primary family.
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
    if (this.systemFontFamilies.has(key)) return false

    const existing = this.pendingFetches.get(key)
    if (existing) return existing

    const promise = this._loadFont(family, weights)
    this.pendingFetches.set(key, promise)
    const result = await promise
    this.pendingFetches.delete(key)
    if (!result) {
      if (isSystemFont(family)) {
        this.systemFontFamilies.add(key)
      } else {
        this.failedFamilies.add(key)
      }
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
      const urls = bundled.map(f => `${this.fontBasePath}${f}`)
      const ok = await this._fetchLocalFonts(family, urls, bundled)
      if (ok) return true
    }

    // 2. Skip Google Fonts for system/proprietary fonts
    if (isSystemFont(family)) {
      return false
    }

    // 3. Fall back to Google Fonts CDN
    return this._fetchGoogleFont(family, weights)
  }

  private async _fetchLocalFonts(family: string, urls: string[], relPaths: string[]): Promise<boolean> {
    try {
      const buffers = await Promise.all(
        urls.map(async (url) => {
          const resp = await fetch(url)
          if (!resp.ok) return null
          return resp.arrayBuffer()
        })
      )
      let registered = 0
      for (let i = 0; i < buffers.length; i++) {
        const buf = buffers[i]
        if (!buf) continue
        const regName = relPaths[i].includes('-ext-') ? family + ' Ext' : family
        if (this.registerFont(buf, regName)) registered++
      }
      return registered > 0
    } catch {
      return false
    }
  }

  /**
   * Fetch a font from Google Fonts CDN with China mirror fallback.
   */
  private async _fetchGoogleFont(family: string, weights: number[]): Promise<boolean> {
    const weightStr = weights.join(';')
    const encodedFamily = encodeURIComponent(family)
    const query = `family=${encodedFamily}:wght@${weightStr}&display=swap`

    const cdnConfigs = [
      {
        cssBase: this.googleFontsCssUrl,
        fontUrlPattern: /url\((https?:\/\/[^)]+\.woff2)\)/g,
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
    this.systemFontFamilies.clear()
    this.pendingFetches.clear()
  }
}

// ---------------------------------------------------------------------------
// System font detection (browser-only)
// ---------------------------------------------------------------------------

const localFontCache = new Map<string, boolean>()

function isFontLocallyAvailable(family: string): boolean {
  const key = family.toLowerCase()
  const cached = localFontCache.get(key)
  if (cached !== undefined) return cached

  if (typeof document === 'undefined') return false
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')
  if (!ctx) return false

  const testStr = 'mmmmmmmmmmlli1|'
  ctx.font = '72px monospace'
  const monoWidth = ctx.measureText(testStr).width
  ctx.font = '72px serif'
  const serifWidth = ctx.measureText(testStr).width
  ctx.font = `72px "${family}", monospace`
  const testMonoWidth = ctx.measureText(testStr).width
  ctx.font = `72px "${family}", serif`
  const testSerifWidth = ctx.measureText(testStr).width

  const available = testMonoWidth !== monoWidth && testSerifWidth !== serifWidth
  localFontCache.set(key, available)
  return available
}

const NON_GOOGLE_FONT_PATTERNS = [
  /^microsoft/i, /^ms /i, /^segoe/i, /^simhei/i, /^simsun/i,
  /^kaiti/i, /^fangsong/i, /^youyuan/i, /^lishu/i, /^dengxian/i,
  /^sf /i, /^sf-/i, /^apple/i, /^pingfang/i, /^hiragino/i,
  /^helvetica/i, /^menlo/i, /^monaco/i, /^lucida grande/i,
  /^avenir/i, /^\.apple/i,
  /^d-din/i, /^din[ -]/i, /^din$/i, /^proxima/i, /^gotham/i,
  /^futura/i, /^akzidenz/i, /^univers/i, /^frutiger/i,
  /^youshebiaotihei/i, /^youshebiaoti/i,
  /^fz/i, /^alibaba/i, /^huawen/i, /^stk/i, /^st[hf]/i,
  /^source han /i, /^noto sans cjk/i, /^noto serif cjk/i,
  /^yu gothic/i, /^yu mincho/i, /^meiryo/i, /^ms gothic/i, /^ms mincho/i,
  /^system-ui/i, /^-apple-system/i, /^blinkmacsystemfont/i,
  /^arial/i, /^times new roman/i, /^courier new/i, /^georgia/i,
  /^verdana/i, /^tahoma/i, /^trebuchet/i, /^impact/i,
  /^comic sans/i, /^consolas/i, /^calibri/i, /^cambria/i,
]

function isKnownNonGoogleFont(family: string): boolean {
  return NON_GOOGLE_FONT_PATTERNS.some(p => p.test(family.trim()))
}

function isSystemFont(family: string): boolean {
  return isFontLocallyAvailable(family) || isKnownNonGoogleFont(family)
}

function fetchWithTimeout(url: string, ms: number): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), ms)
  return fetch(url, { signal: controller.signal }).finally(() => clearTimeout(timer))
}
