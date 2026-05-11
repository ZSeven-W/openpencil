import type { Framework } from '@zseven-w/pen-types';
import type { CodegenAssetFile } from '@/services/ai/codegen-assets';
import type { CodegenAssetManifestEntry } from '@/types/cloud';

export type PreviewBuildResult =
  | { kind: 'previewable'; srcDoc: string }
  | { kind: 'unsupported'; reason: string };

export const PREVIEW_UNSUPPORTED_REASONS = {
  historyAssetsUnavailable:
    'History assets are unavailable for preview. Regenerate or download the bundle.',
  vueTemplateRequired: 'Vue preview requires a <template> block.',
  reactUnsupported: 'React preview is not supported in v1.',
  frameworkUnsupported: 'This framework cannot be previewed in the browser yet.',
} as const;

const PREVIEW_CSP_META = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: blob: https:; style-src 'unsafe-inline'; script-src https://unpkg.com 'unsafe-inline'; font-src data: https:;">`;
const PREVIEW_BOOTSTRAP_SCRIPT = [
  '<script src="https://unpkg.com/lucide@latest/dist/umd/lucide.js"></script>',
  '<script>try{window.lucide&&window.lucide.createIcons&&window.lucide.createIcons()}catch(e){}</script>',
].join('');

interface BuildCodePreviewInput {
  framework: Framework;
  code: string;
  assets: CodegenAssetFile[];
  assetsManifest?: CodegenAssetManifestEntry[];
}

function extractBlock(code: string, tag: string): string | null {
  const match = code.match(new RegExp(`<${tag}[^>]*>([\\s\\S]*?)<\\/${tag}>`, 'i'));
  return match?.[1]?.trim() ?? null;
}

function removeBlocks(code: string, tags: string[]): string {
  return tags.reduce(
    (next, tag) => next.replace(new RegExp(`<${tag}[^>]*>[\\s\\S]*?<\\/${tag}>`, 'gi'), ''),
    code,
  );
}

function isFullHtmlDocument(code: string): boolean {
  return /<!doctype\s+html/i.test(code) || /<html[\s>]/i.test(code);
}

function bytesToBase64(bytes: Uint8Array): string {
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(bytes).toString('base64');
  }

  let binary = '';
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    const chunk = bytes.subarray(offset, offset + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function addPathAliases(map: Map<string, string>, path: string, url: string): void {
  const normalized = path.replace(/^\.?\//, '');
  map.set(path, url);
  map.set(normalized, url);
  map.set(`./${normalized}`, url);
  map.set(`/${normalized}`, url);
}

function buildAssetUrlMap(
  assets: CodegenAssetFile[],
  assetsManifest: CodegenAssetManifestEntry[] = [],
): Map<string, string> {
  const map = new Map<string, string>();

  for (const asset of assets) {
    const bytes = Uint8Array.from(asset.bytes);
    const url = `data:${asset.mimeType};base64,${bytesToBase64(bytes)}`;
    addPathAliases(map, asset.relativePath, url);
    addPathAliases(map, asset.zipPath, url);
  }

  for (const asset of assetsManifest) {
    if (!asset.signedUrl) continue;
    addPathAliases(map, asset.relativePath, asset.signedUrl);
    addPathAliases(map, asset.zipPath, asset.signedUrl);
  }

  return map;
}

function replaceAssetReferences(code: string, assetMap: Map<string, string>): string {
  const paths = Array.from(assetMap.keys()).sort((a, b) => b.length - a.length);
  if (paths.length === 0) return code;

  const pattern = new RegExp(
    paths.map((path) => path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'),
    'g',
  );
  return code.replace(pattern, (path) => assetMap.get(path) ?? path);
}

function wrapPreviewDocument(body: string, styles = ''): string {
  return [
    '<!doctype html>',
    '<html>',
    '<head>',
    '<meta charset="utf-8" />',
    '<meta name="viewport" content="width=device-width, initial-scale=1" />',
    PREVIEW_CSP_META,
    '<style>',
    'html,body{margin:0;min-height:100%;background:#fff;color:#111;font-family:Inter,ui-sans-serif,system-ui,sans-serif;}',
    'body{overflow:auto;}',
    styles,
    '</style>',
    '</head>',
    '<body>',
    body,
    PREVIEW_BOOTSTRAP_SCRIPT,
    '</body>',
    '</html>',
  ].join('');
}

function buildFullHtmlPreview(code: string): string {
  const withCsp = /http-equiv=["']Content-Security-Policy["']/i.test(code)
    ? code
    : code.replace(/<head([^>]*)>/i, `<head$1>${PREVIEW_CSP_META}`);
  if (/<\/body>/i.test(withCsp)) {
    return withCsp.replace(/<\/body>/i, `${PREVIEW_BOOTSTRAP_SCRIPT}</body>`);
  }
  return `${withCsp}${PREVIEW_BOOTSTRAP_SCRIPT}`;
}

function buildHtmlPreview(code: string): string {
  if (isFullHtmlDocument(code)) return buildFullHtmlPreview(code);

  const style = extractBlock(code, 'style') ?? '';
  const body = removeBlocks(code, ['style', 'script']);
  return wrapPreviewDocument(body, style);
}

function buildVuePreview(code: string): PreviewBuildResult {
  const template = extractBlock(code, 'template');
  if (!template) {
    return { kind: 'unsupported', reason: PREVIEW_UNSUPPORTED_REASONS.vueTemplateRequired };
  }
  const style = extractBlock(code, 'style') ?? '';
  return { kind: 'previewable', srcDoc: wrapPreviewDocument(template, style) };
}

function buildSveltePreview(code: string): string {
  const style = extractBlock(code, 'style') ?? '';
  const body = removeBlocks(code, ['script', 'style']);
  return wrapPreviewDocument(body, style);
}

function hasUnresolvedGeneratedAsset(code: string): boolean {
  return /["'(](?:\.?\/)?assets\//.test(code);
}

export function buildCodePreviewDocument(input: BuildCodePreviewInput): PreviewBuildResult {
  const assetMap = buildAssetUrlMap(input.assets, input.assetsManifest);
  const code = replaceAssetReferences(input.code, assetMap);

  if (
    input.assets.length === 0 &&
    (input.assetsManifest?.length ?? 0) > 0 &&
    hasUnresolvedGeneratedAsset(code)
  ) {
    return { kind: 'unsupported', reason: PREVIEW_UNSUPPORTED_REASONS.historyAssetsUnavailable };
  }

  switch (input.framework) {
    case 'html':
      return { kind: 'previewable', srcDoc: buildHtmlPreview(code) };
    case 'vue': {
      const result = buildVuePreview(code);
      if (result.kind === 'unsupported') return result;
      return result;
    }
    case 'svelte':
      return { kind: 'previewable', srcDoc: buildSveltePreview(code) };
    case 'react':
      return { kind: 'unsupported', reason: PREVIEW_UNSUPPORTED_REASONS.reactUnsupported };
    default:
      return { kind: 'unsupported', reason: PREVIEW_UNSUPPORTED_REASONS.frameworkUnsupported };
  }
}
