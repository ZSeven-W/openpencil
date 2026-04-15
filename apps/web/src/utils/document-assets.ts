const EXTERNAL_ASSET_RE = /^(?:data:|https?:|blob:)/i;
const FILE_URL_RE = /^file:\/\//i;
const HTTP_PROTOCOL_RE = /^https?:$/i;
const LOCAL_IMAGE_EXT_RE = /\.(?:png|jpe?g|gif|webp|bmp|svg|avif)$/i;

export interface RuntimeAssetSource {
  sourcePath: string | null;
  runtimeUrl: string | null;
  isLocal: boolean;
  unresolved: boolean;
}

export function isLocalAssetPath(assetPath: string | null | undefined): boolean {
  if (!assetPath) return false;
  return !EXTERNAL_ASSET_RE.test(assetPath.trim());
}

export function resolveRuntimeAssetSource(
  assetPath: string | null | undefined,
  documentPath: string | null | undefined,
): RuntimeAssetSource {
  if (!assetPath) {
    return {
      sourcePath: null,
      runtimeUrl: null,
      isLocal: false,
      unresolved: false,
    };
  }

  if (!isLocalAssetPath(assetPath)) {
    return {
      sourcePath: null,
      runtimeUrl: assetPath,
      isLocal: false,
      unresolved: false,
    };
  }

  if (FILE_URL_RE.test(assetPath)) {
    const sourcePath = normalizePath(decodeFileUrl(assetPath));
    return createLocalRuntimeAssetSource(sourcePath);
  }

  if (isAbsolutePath(assetPath)) {
    const sourcePath = normalizePath(assetPath);
    return createLocalRuntimeAssetSource(sourcePath);
  }

  if (!documentPath) {
    return {
      sourcePath: null,
      runtimeUrl: null,
      isLocal: true,
      unresolved: true,
    };
  }

  const sourcePath = resolveDocumentAssetPath(documentPath, assetPath);
  return createLocalRuntimeAssetSource(sourcePath);
}

export function resolveDocumentAssetPath(documentPath: string, assetPath: string): string {
  if (FILE_URL_RE.test(assetPath)) {
    return normalizePath(decodeFileUrl(assetPath));
  }
  if (isAbsolutePath(assetPath)) {
    return normalizePath(assetPath);
  }

  const baseDir = getDirname(documentPath);
  return joinNormalizedPath(baseDir, assetPath);
}

export function toStoredAssetPath(
  assetPath: string,
  documentPath: string | null | undefined,
): string {
  if (!assetPath) return assetPath;
  if (!isLocalAssetPath(assetPath)) return assetPath;

  const normalizedAssetPath = FILE_URL_RE.test(assetPath)
    ? normalizePath(decodeFileUrl(assetPath))
    : normalizePath(assetPath);

  if (!isAbsolutePath(normalizedAssetPath) || !documentPath) {
    return normalizedAssetPath;
  }

  const relative = toRelativePath(getDirname(documentPath), normalizedAssetPath);
  return relative ?? normalizedAssetPath;
}

export function toFileUrl(filePath: string): string {
  const normalized = normalizePath(filePath);
  if (normalized.startsWith('//')) {
    return `file:${encodeURI(normalized)}`;
  }
  if (/^[A-Za-z]:\//.test(normalized)) {
    return `file:///${encodeURI(normalized)}`;
  }
  if (normalized.startsWith('/')) {
    return `file://${encodeURI(normalized)}`;
  }
  return encodeURI(normalized);
}

export function toRuntimeLocalAssetUrl(filePath: string): string {
  return toLocalAssetBridgeUrl(filePath) ?? toFileUrl(filePath);
}

export function toLocalAssetBridgeUrl(filePath: string): string | null {
  const runtimeOrigin = getRuntimeOrigin();
  if (!runtimeOrigin) return null;
  const normalized = normalizePath(filePath);
  return `${runtimeOrigin}/api/local-asset?path=${encodeURIComponent(normalized)}`;
}

function toRelativePath(baseDir: string, targetPath: string): string | null {
  const normalizedBase = normalizePath(baseDir);
  const normalizedTarget = normalizePath(targetPath);
  const basePrefix = extractPathPrefix(normalizedBase);
  const targetPrefix = extractPathPrefix(normalizedTarget);

  if (basePrefix.toLowerCase() !== targetPrefix.toLowerCase()) {
    return null;
  }

  const baseParts = stripPrefix(normalizedBase, basePrefix).split('/').filter(Boolean);
  const targetParts = stripPrefix(normalizedTarget, targetPrefix).split('/').filter(Boolean);

  let shared = 0;
  while (
    shared < baseParts.length &&
    shared < targetParts.length &&
    baseParts[shared].toLowerCase() === targetParts[shared].toLowerCase()
  ) {
    shared += 1;
  }

  const up: string[] = Array.from({ length: baseParts.length - shared }, () => '..');
  const down = targetParts.slice(shared);
  const relative = [...up, ...down].join('/');
  return relative || '.';
}

function decodeFileUrl(url: string): string {
  const decoded = decodeURIComponent(url.replace(/^file:\/\/+/i, ''));
  return decoded.replace(/^\/([A-Za-z]:)/, '$1');
}

function getRuntimeOrigin(): string | null {
  if (typeof window === 'undefined' || !window.location) return null;
  if (!HTTP_PROTOCOL_RE.test(window.location.protocol)) return null;
  const origin = window.location.origin?.trim();
  return origin && origin !== 'null' ? origin.replace(/\/+$/, '') : null;
}

function isAbsolutePath(value: string): boolean {
  return /^[A-Za-z]:[\\/]/.test(value) || value.startsWith('\\\\') || value.startsWith('/');
}

function normalizePath(value: string): string {
  const path = value.replace(/\\/g, '/');
  return path.startsWith('//')
    ? `//${path.slice(2).replace(/\/+/g, '/')}`
    : path.replace(/\/+/g, '/');
}

function createLocalRuntimeAssetSource(sourcePath: string): RuntimeAssetSource {
  return {
    sourcePath,
    runtimeUrl: canBridgeLocalImagePath(sourcePath) ? toRuntimeLocalAssetUrl(sourcePath) : null,
    isLocal: true,
    unresolved: false,
  };
}

function hasSupportedLocalImageExtension(filePath: string): boolean {
  const normalized = normalizePath(filePath).split(/[?#]/, 1)[0];
  return LOCAL_IMAGE_EXT_RE.test(normalized);
}

function canBridgeLocalImagePath(filePath: string): boolean {
  if (hasSupportedLocalImageExtension(filePath)) return true;
  return !hasExplicitFileExtension(filePath);
}

function hasExplicitFileExtension(filePath: string): boolean {
  const normalized = normalizePath(filePath).split(/[?#]/, 1)[0];
  const segments = normalized.split('/');
  const lastSegment = segments[segments.length - 1] ?? '';
  const dotIndex = lastSegment.lastIndexOf('.');
  return dotIndex > 0 && dotIndex < lastSegment.length - 1;
}

function getDirname(filePath: string): string {
  const normalized = normalizePath(filePath).replace(/\/+$/, '');
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash <= 0) {
    return normalized.includes(':') ? `${normalized.split(':')[0]}:/` : '/';
  }
  return normalized.slice(0, lastSlash);
}

function joinNormalizedPath(baseDir: string, relativePath: string): string {
  const normalizedBase = normalizePath(baseDir);
  const normalizedRelative = normalizePath(relativePath);
  const prefix = extractPathPrefix(normalizedBase);
  const baseSegments = stripPrefix(normalizedBase, prefix).split('/').filter(Boolean);
  const relativeSegments = normalizedRelative.split('/').filter(Boolean);

  const parts = [...baseSegments];
  for (const segment of relativeSegments) {
    if (segment === '.') continue;
    if (segment === '..') {
      if (parts.length > 0) parts.pop();
      continue;
    }
    parts.push(segment);
  }

  if (!prefix) return parts.join('/');
  const joined = parts.join('/');
  if (prefix === '/') return joined ? `/${joined}` : '/';
  if (prefix.startsWith('//')) return joined ? `${prefix}/${joined}` : prefix;
  return joined ? `${prefix}/${joined}` : `${prefix}/`;
}

function extractPathPrefix(path: string): string {
  if (path.startsWith('//')) {
    const parts = path.split('/').filter(Boolean);
    if (parts.length >= 2) {
      return `//${parts[0]}/${parts[1]}`;
    }
    return '//';
  }
  const driveMatch = path.match(/^[A-Za-z]:/);
  if (driveMatch) return driveMatch[0];
  if (path.startsWith('/')) return '/';
  return '';
}

function stripPrefix(path: string, prefix: string): string {
  if (!prefix) return path;
  if (prefix === '/') return path.slice(1);
  if (prefix.startsWith('//')) return path.slice(prefix.length);
  return path.slice(prefix.length).replace(/^\/+/, '');
}
