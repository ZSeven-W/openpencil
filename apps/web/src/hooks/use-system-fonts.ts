import { useState, useEffect, useCallback } from 'react';

export interface FontInfo {
  family: string;
  source: 'bundled' | 'system';
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
];

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
];

/** Permission state for the Local Font Access API */
export type FontPermissionState = 'prompt' | 'granted' | 'denied' | 'unavailable';

/** Cached system font families to avoid re-querying */
let cachedSystemFonts: string[] | null = null;
let cachedPermissionState: FontPermissionState | null = null;
let fetchPromise: Promise<{ fonts: string[]; permission: FontPermissionState }> | null = null;

async function querySystemFonts(): Promise<{ fonts: string[]; permission: FontPermissionState }> {
  if (cachedSystemFonts && cachedPermissionState) {
    return { fonts: cachedSystemFonts, permission: cachedPermissionState };
  }

  if (fetchPromise) return fetchPromise;

  fetchPromise = (async () => {
    try {
      // queryLocalFonts() is available in Chromium 103+ and Electron
      if ('queryLocalFonts' in window) {
        const fonts = await (
          window as unknown as { queryLocalFonts: () => Promise<Array<{ family: string }>> }
        ).queryLocalFonts();
        const families = new Set<string>();
        for (const font of fonts) {
          families.add(font.family);
        }
        // Remove bundled fonts from system list to avoid duplicates
        const bundledSet = new Set(BUNDLED_FAMILIES.map((f) => f.toLowerCase()));
        const systemFonts = [...families]
          .filter((f) => !bundledSet.has(f.toLowerCase()))
          .sort((a, b) => a.localeCompare(b));
        cachedSystemFonts = systemFonts;
        cachedPermissionState = 'granted';
        return { fonts: systemFonts, permission: 'granted' as FontPermissionState };
      }
    } catch (e: unknown) {
      // Check if it was a permission denial
      if (e instanceof DOMException && e.name === 'NotAllowedError') {
        cachedPermissionState = 'denied';
        cachedSystemFonts = FALLBACK_SYSTEM_FONTS;
        return { fonts: FALLBACK_SYSTEM_FONTS, permission: 'denied' };
      }
      // Other error — API may be unavailable
      console.warn(
        '[useSystemFonts] queryLocalFonts failed:',
        e instanceof Error ? e.message : String(e),
      );
    }

    // API not available or failed
    if (!cachedPermissionState) {
      cachedPermissionState =
        typeof window !== 'undefined' && 'queryLocalFonts' in window ? 'denied' : 'unavailable';
    }
    cachedSystemFonts = FALLBACK_SYSTEM_FONTS;
    return { fonts: FALLBACK_SYSTEM_FONTS, permission: cachedPermissionState };
  })();

  return fetchPromise;
}

/**
 * Request local font access permission from the user.
 * Must be called from a user gesture context (click handler).
 * Resets cached state and re-queries fonts.
 */
async function requestFontAccess(): Promise<{ fonts: string[]; permission: FontPermissionState }> {
  // Reset cache to force re-query
  cachedSystemFonts = null;
  cachedPermissionState = null;
  fetchPromise = null;
  return querySystemFonts();
}

/**
 * Hook to enumerate system fonts via the Local Font Access API.
 * Falls back to a common font list if the API is unavailable.
 */
export function useSystemFonts() {
  const [systemFonts, setSystemFonts] = useState<string[]>(cachedSystemFonts ?? []);
  const [loading, setLoading] = useState(false);
  const [permissionState, setPermissionState] = useState<FontPermissionState>(
    cachedPermissionState ?? 'prompt',
  );

  // Only read from module-level cache on mount — do NOT call queryLocalFonts()
  // here because it requires a user gesture context. Fonts are populated via
  // requestAccess() when the user opens the font picker dropdown.
  useEffect(() => {
    if (cachedSystemFonts && cachedPermissionState) {
      setSystemFonts(cachedSystemFonts);
      setPermissionState(cachedPermissionState);
    }
  }, []);

  const requestAccess = useCallback(async () => {
    setLoading(true);
    const { fonts, permission } = await requestFontAccess();
    setSystemFonts(fonts);
    setPermissionState(permission);
    setLoading(false);
  }, []);

  const allFonts: FontInfo[] = [
    ...BUNDLED_FAMILIES.map((f) => ({ family: f, source: 'bundled' as const })),
    ...systemFonts.map((f) => ({ family: f, source: 'system' as const })),
  ];

  return {
    allFonts,
    systemFonts,
    bundledFonts: BUNDLED_FAMILIES,
    loading,
    permissionState,
    requestAccess,
  };
}
