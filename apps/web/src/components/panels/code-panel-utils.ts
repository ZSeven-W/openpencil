import type { Framework } from '@zseven-w/pen-types';
import type { SaveCodegenFileInput } from '@/types/cloud';
import type { SyntaxLanguage } from '@/utils/syntax-highlight';

export const TAB_LABELS: Record<Framework, string> = {
  react: 'React',
  vue: 'Vue',
  svelte: 'Svelte',
  html: 'HTML',
  flutter: 'Flutter',
  swiftui: 'SwiftUI',
  compose: 'Compose',
  'react-native': 'RN',
  uniapp: 'UniApp',
};

export const HIGHLIGHT_LANG: Record<Framework, SyntaxLanguage> = {
  react: 'jsx',
  vue: 'html',
  svelte: 'html',
  html: 'html',
  flutter: 'dart',
  swiftui: 'swift',
  compose: 'kotlin',
  'react-native': 'jsx',
  uniapp: 'html',
};

export function getFileHighlightLanguage(
  file: SaveCodegenFileInput,
  fallback: SyntaxLanguage,
): SyntaxLanguage {
  if (file.language === 'css' || file.language === 'scss') return 'css';
  if (file.language === 'dart') return 'dart';
  if (file.language === 'kotlin') return 'kotlin';
  if (file.language === 'swift') return 'swift';
  if (
    file.language === 'html' ||
    file.language === 'vue' ||
    file.language === 'svelte' ||
    file.language === 'json'
  ) {
    return 'html';
  }
  if (file.language === 'ts' || file.language === 'tsx') return 'jsx';
  return fallback;
}

export function triggerDownload(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const link = globalThis.document.createElement('a');
  link.href = url;
  link.download = fileName;
  globalThis.document.body.appendChild(link);
  link.click();
  globalThis.document.body.removeChild(link);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
