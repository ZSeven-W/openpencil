import { useState, useEffect } from 'react'

export interface FontInfo {
  family: string
  source: 'bundled' | 'system'
}

/** Bundled font families (always available, vector rendering) */
const BUNDLED_FAMILIES = [
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

/** Common system fonts shown even when queryLocalFonts is not available */
const FALLBACK_SYSTEM_FONTS = [
  'Arial',
  'Helvetica',
  'Helvetica Neue',
  'Georgia',
  'Times New Roman',
  'Courier New',
  'Verdana',
  'Trebuchet MS',
  'Tahoma',
  'Impact',
  'Comic Sans MS',
]

/** Cached system font families to avoid re-querying */
let cachedSystemFonts: string[] | null = null
let fetchPromise: Promise<string[]> | null = null

async function querySystemFonts(): Promise<string[]> {
  if (cachedSystemFonts) return cachedSystemFonts

  if (fetchPromise) return fetchPromise

  fetchPromise = (async () => {
    try {
      // queryLocalFonts() is available in Chromium 103+ and Electron
      if ('queryLocalFonts' in window) {
        const fonts = await (window as unknown as { queryLocalFonts: () => Promise<Array<{ family: string }>> }).queryLocalFonts()
        const families = new Set<string>()
        for (const font of fonts) {
          families.add(font.family)
        }
        // Remove bundled fonts from system list to avoid duplicates
        const bundledSet = new Set(BUNDLED_FAMILIES.map(f => f.toLowerCase()))
        const systemFonts = [...families]
          .filter(f => !bundledSet.has(f.toLowerCase()))
          .sort((a, b) => a.localeCompare(b))
        cachedSystemFonts = systemFonts
        return systemFonts
      }
    } catch {
      // Permission denied or API not available
    }
    cachedSystemFonts = FALLBACK_SYSTEM_FONTS
    return FALLBACK_SYSTEM_FONTS
  })()

  return fetchPromise
}

/**
 * Hook to enumerate system fonts via the Local Font Access API.
 * Falls back to a common font list if the API is unavailable.
 */
export function useSystemFonts() {
  const [systemFonts, setSystemFonts] = useState<string[]>(cachedSystemFonts ?? [])
  const [loading, setLoading] = useState(!cachedSystemFonts)

  useEffect(() => {
    if (cachedSystemFonts) {
      setSystemFonts(cachedSystemFonts)
      setLoading(false)
      return
    }
    let cancelled = false
    querySystemFonts().then(fonts => {
      if (!cancelled) {
        setSystemFonts(fonts)
        setLoading(false)
      }
    })
    return () => { cancelled = true }
  }, [])

  const allFonts: FontInfo[] = [
    ...BUNDLED_FAMILIES.map(f => ({ family: f, source: 'bundled' as const })),
    ...systemFonts.map(f => ({ family: f, source: 'system' as const })),
  ]

  return { allFonts, systemFonts, bundledFonts: BUNDLED_FAMILIES, loading }
}
