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
}

/** List of all bundled font family names (for UI font picker) */
export const BUNDLED_FONT_FAMILIES = [
  'Inter',
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

    const existing = this.pendingFetches.get(key)
    if (existing) return existing

    const promise = this._loadFont(family, weights)
    this.pendingFetches.set(key, promise)
    const result = await promise
    this.pendingFetches.delete(key)
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

    // 2. Fall back to Google Fonts CDN
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
      for (const buf of buffers) {
        if (buf && this.registerFont(buf, family)) registered++
      }
      console.log(`[FontManager] Local fonts for "${family}": ${registered}/${urls.length} registered`)
      return registered > 0
    } catch (e) {
      console.warn(`[FontManager] Local font fetch error for "${family}":`, e)
      return false
    }
  }

  private async _fetchGoogleFont(family: string, weights: number[]): Promise<boolean> {
    try {
      const weightStr = weights.join(';')
      const cssUrl = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(family)}:wght@${weightStr}&display=swap`

      const cssResp = await fetch(cssUrl)
      if (!cssResp.ok) return false
      const css = await cssResp.text()

      const urlRegex = /url\((https:\/\/fonts\.gstatic\.com\/[^)]+\.woff2)\)/g
      const urls: string[] = []
      let match: RegExpExecArray | null
      while ((match = urlRegex.exec(css)) !== null) {
        urls.push(match[1])
      }

      if (urls.length === 0) return false

      const fontBuffers = await Promise.all(
        urls.map(async (url) => {
          const resp = await fetch(url)
          return resp.ok ? resp.arrayBuffer() : null
        })
      )

      let registered = 0
      for (const buf of fontBuffers) {
        if (buf && this.registerFont(buf, family)) registered++
      }
      return registered > 0
    } catch {
      return false
    }
  }

  dispose() {
    this.provider.delete()
    this.loadedFamilies.clear()
    this.pendingFetches.clear()
  }
}
