import type { Framework } from '@zseven-w/pen-types';
import type { SaveCodegenFileInput } from '@/types/cloud';

const FILE_MARKER_PATTERN = /^---FILE:\s*(.+?)\s*---\s*$/gm;

const DEFAULT_FILE_BY_FRAMEWORK: Record<Framework, string> = {
  react: 'design.tsx',
  vue: 'design.vue',
  svelte: 'design.svelte',
  html: 'design.html',
  flutter: 'lib/main.dart',
  swiftui: 'Design.swift',
  compose: 'Design.kt',
  'react-native': 'Design.tsx',
  uniapp: 'uniapp-bundle.txt',
};

const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  css: 'css',
  dart: 'dart',
  html: 'html',
  json: 'json',
  kt: 'kotlin',
  scss: 'scss',
  svelte: 'svelte',
  swift: 'swift',
  ts: 'ts',
  tsx: 'tsx',
  txt: 'text',
  vue: 'vue',
};

export function getDefaultCodegenFilePath(framework: Framework): string {
  return DEFAULT_FILE_BY_FRAMEWORK[framework];
}

export function normalizeCodegenFilePath(path: string): string | null {
  const trimmed = path.trim().replace(/\\/g, '/');
  if (trimmed.startsWith('/')) return null;

  const normalized = trimmed.replace(/^\.\//, '');
  if (!normalized || normalized.startsWith('/') || normalized.split('/').includes('..')) {
    return null;
  }
  return normalized;
}

export function inferCodegenFileLanguage(path: string): string {
  const extension = path.split('.').pop()?.toLowerCase() ?? '';
  return LANGUAGE_BY_EXTENSION[extension] ?? 'text';
}

export function inferCodegenFileRole(path: string): SaveCodegenFileInput['role'] {
  const normalized = path.toLowerCase();
  if (
    normalized === 'app.vue' ||
    normalized === 'main.ts' ||
    normalized === 'main.js' ||
    normalized === 'design.tsx' ||
    normalized === 'design.vue' ||
    normalized === 'design.svelte' ||
    normalized === 'design.html' ||
    normalized === 'design.swift' ||
    normalized === 'design.kt' ||
    normalized === 'lib/main.dart'
  ) {
    return 'entry';
  }
  if (normalized === 'pages.json' || normalized === 'manifest.json') return 'config';
  if (normalized === 'uni.scss' || normalized.endsWith('.css') || normalized.endsWith('.scss')) {
    return 'style';
  }
  if (normalized.startsWith('pages/')) return 'page';
  if (normalized.startsWith('components/')) return 'component';
  if (normalized.includes('/assets/') || normalized.startsWith('assets/')) return 'asset';
  return 'other';
}

function buildCodegenFile(
  path: string,
  content: string,
  orderIndex: number,
): SaveCodegenFileInput | null {
  const normalizedPath = normalizeCodegenFilePath(path);
  if (!normalizedPath || !content.trim()) return null;

  return {
    path: normalizedPath,
    language: inferCodegenFileLanguage(normalizedPath),
    role: inferCodegenFileRole(normalizedPath),
    content: content.replace(/\r?\n$/, ''),
    orderIndex,
  };
}

export function parseDelimitedCodegenFiles(code: string): SaveCodegenFileInput[] {
  const markers = Array.from(code.matchAll(FILE_MARKER_PATTERN));
  const files: SaveCodegenFileInput[] = [];
  const seenPaths = new Set<string>();

  for (let i = 0; i < markers.length; i++) {
    const marker = markers[i];
    if (marker.index === undefined) continue;

    const contentStart = marker.index + marker[0].length;
    const nextMarkerIndex = markers[i + 1]?.index ?? code.length;
    const content = code.slice(contentStart, nextMarkerIndex).replace(/^\r?\n/, '');
    const file = buildCodegenFile(marker[1], content, files.length);
    if (!file || seenPaths.has(file.path)) continue;

    seenPaths.add(file.path);
    files.push(file);
  }

  return files;
}

export function buildCodegenFiles(input: {
  framework: Framework;
  code: string;
}): SaveCodegenFileInput[] {
  const files = parseDelimitedCodegenFiles(input.code);
  if (files.length > 0) return files;

  const fallback = buildCodegenFile(getDefaultCodegenFilePath(input.framework), input.code, 0);
  return fallback ? [fallback] : [];
}

export interface CodegenFilesValidationResult {
  valid: boolean;
  issues: string[];
}

function parseJsonObject(content: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(content);
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function getUniAppPagePaths(pagesJson: Record<string, unknown>): string[] | null {
  if (!Array.isArray(pagesJson.pages)) return null;
  const paths = pagesJson.pages
    .map((page) => {
      if (!page || typeof page !== 'object' || Array.isArray(page)) return null;
      const path = (page as Record<string, unknown>).path;
      return typeof path === 'string' && path.length > 0 ? path : null;
    })
    .filter((path): path is string => path !== null);
  return paths.length > 0 ? paths : null;
}

function validateUniAppFiles(files: SaveCodegenFileInput[]): string[] {
  const issues: string[] = [];
  const byPath = new Map(files.map((file) => [file.path, file]));

  for (const requiredPath of ['App.vue', 'main.ts', 'pages.json', 'manifest.json', 'uni.scss']) {
    if (!byPath.has(requiredPath)) {
      issues.push(`UniApp output missing ${requiredPath}`);
    }
  }

  const pageFiles = files.filter((file) => /^pages\/.+\.vue$/i.test(file.path));
  if (pageFiles.length === 0) {
    issues.push('UniApp output missing at least one pages/**/*.vue page file');
  }

  const pagesJsonFile = byPath.get('pages.json');
  if (!pagesJsonFile) return issues;

  const pagesJson = parseJsonObject(pagesJsonFile.content);
  if (!pagesJson) {
    issues.push('UniApp pages.json is invalid JSON');
    return issues;
  }

  const pagePaths = getUniAppPagePaths(pagesJson);
  if (!pagePaths) {
    issues.push('UniApp pages.json must include at least one page path');
    return issues;
  }

  for (const pagePath of pagePaths) {
    const pageFilePath = pagePath.endsWith('.vue') ? pagePath : `${pagePath}.vue`;
    if (!byPath.has(pageFilePath)) {
      issues.push(`UniApp pages.json references missing page file ${pageFilePath}`);
    }
  }

  return issues;
}

export function validateCodegenFiles(
  framework: Framework,
  files: SaveCodegenFileInput[],
): CodegenFilesValidationResult {
  if (framework !== 'uniapp') {
    return { valid: true, issues: [] };
  }

  const issues = validateUniAppFiles(files);
  return { valid: issues.length === 0, issues };
}
