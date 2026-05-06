import { useState, useEffect, useCallback } from 'react';

export interface FontInfo {
  family: string;
  source: 'bundled' | 'system';
}

/** Bundled 字体系列（始终可用，矢量渲染） */
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

/** 即使 queryLocalFonts 不可用，也会显示 Common 系统字体 */
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

/** Permission 的 Local 状态 Font Access API */
export type FontPermissionState = 'prompt' | 'granted' | 'denied' | 'unavailable';

/** Cached 系统字体系列以避免重新查询 */
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
      // queryLocalFonts() 可用于 Chromium 103+ 和 Electron
      if ('queryLocalFonts' in window) {
        const fonts = await (
          window as unknown as { queryLocalFonts: () => Promise<Array<{ family: string }>> }
        ).queryLocalFonts();
        const families = new Set<string>();
        for (const font of fonts) {
          families.add(font.family);
        }
        // Remove 从系统列表中捆绑字体以避免重复
        const bundledSet = new Set(BUNDLED_FAMILIES.map((f) => f.toLowerCase()));
        const systemFonts = [...families]
          .filter((f) => !bundledSet.has(f.toLowerCase()))
          .sort((a, b) => a.localeCompare(b));
        cachedSystemFonts = systemFonts;
        cachedPermissionState = 'granted';
        return { fonts: systemFonts, permission: 'granted' as FontPermissionState };
      }
    } catch (e: unknown) {
      // Check 如果是权限拒绝
      if (e instanceof DOMException && e.name === 'NotAllowedError') {
        cachedPermissionState = 'denied';
        cachedSystemFonts = FALLBACK_SYSTEM_FONTS;
        return { fonts: FALLBACK_SYSTEM_FONTS, permission: 'denied' };
      }
      // Other 错误 — API 可能不可用
      console.warn(
        '[useSystemFonts] queryLocalFonts failed:',
        e instanceof Error ? e.message : String(e),
      );
    }

    // API 不可用或失败
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
 * Request
 * 用户的本地字体访问权限。 Must 从用户手势上下文（单击处理程序）中调用。 Resets
 * 缓存状态并重新查询字体。
 */
async function requestFontAccess(): Promise<{ fonts: string[]; permission: FontPermissionState }> {
  // Reset 缓存强制重新查询
  cachedSystemFonts = null;
  cachedPermissionState = null;
  fetchPromise = null;
  return querySystemFonts();
}

/**
 * Hook 通过
 * Local Font Access API 枚举系统字体。如果 API 不可用，则 Falls 返回通用字体列表。
 */
export function useSystemFonts() {
  const [systemFonts, setSystemFonts] = useState<string[]>(cachedSystemFonts ?? []);
  const [loading, setLoading] = useState(false);
  const [permissionState, setPermissionState] = useState<FontPermissionState>(
    cachedPermissionState ?? 'prompt',
  );

  // Only 在挂载时从模块级缓存读取 — NOT 在此调用 queryLocalFonts()
  // 因为它需要用户手势上下文。当用户打开字体选择器下拉列表时，Fonts 通过 requestAccess() 填充。
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
