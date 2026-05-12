import { describe, expect, it } from 'vitest';

import { buildCodePreviewDocument } from '../codegen-preview';

describe('buildCodePreviewDocument', () => {
  it('wraps an html fragment into a preview document', () => {
    const result = buildCodePreviewDocument({
      framework: 'html',
      code: '<main class="hero">Hello</main><style>.hero{color:red}</style>',
      assets: [],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('<main class="hero">Hello</main>');
      expect(result.srcDoc).toContain('Content-Security-Policy');
    }
  });

  it('uses full html documents directly inside the sandbox document shell', () => {
    const result = buildCodePreviewDocument({
      framework: 'html',
      code: '<!doctype html><html><body><section>Full</section></body></html>',
      assets: [],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('<section>Full</section>');
    }
  });

  it('extracts static template and style from Vue SFC output', () => {
    const result = buildCodePreviewDocument({
      framework: 'vue',
      code: '<script setup lang="ts">const ignored = true</script><template><article>Vue</article></template><style scoped>article{color:blue}</style>',
      assets: [],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('<article>Vue</article>');
      expect(result.srcDoc).toContain('article{color:blue}');
      expect(result.srcDoc).not.toContain('const ignored');
    }
  });

  it('extracts a UniApp page from a generated multi-file bundle', () => {
    const result = buildCodePreviewDocument({
      framework: 'uniapp',
      code: [
        '---FILE: App.vue---',
        '<script setup lang="ts"></script><style>page{background:#fff}</style>',
        '---FILE: pages/index/index.vue---',
        '<script setup lang="ts">const ignored = true</script>',
        '<template><view class="page">UniApp</view></template>',
        '<style scoped>.page{color:purple}</style>',
      ].join('\n'),
      assets: [],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('<view class="page">UniApp</view>');
      expect(result.srcDoc).toContain('.page{color:purple}');
      expect(result.srcDoc).not.toContain('const ignored');
    }
  });

  it('extracts static markup and style from Svelte output', () => {
    const result = buildCodePreviewDocument({
      framework: 'svelte',
      code: '<script lang="ts">let ignored = true;</script><section>Svelte</section><style>section{color:green}</style>',
      assets: [],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('<section>Svelte</section>');
      expect(result.srcDoc).toContain('section{color:green}');
      expect(result.srcDoc).not.toContain('let ignored');
    }
  });

  it('replaces generated asset paths with data URLs for new windows', () => {
    const result = buildCodePreviewDocument({
      framework: 'html',
      code: [
        '<img src="./assets/card.png" />',
        '<img src="assets/card.png" />',
        '<img src="/assets/card.png" />',
        '<style>.hero{background-image:url(assets/card.png)}</style>',
      ].join(''),
      assets: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          bytes: new Uint8Array([1, 2, 3]),
          sourceNodeId: 'node-1',
          sourceKind: 'image-fill',
        },
      ],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).not.toContain('./assets/card.png');
      expect(result.srcDoc).not.toContain('"/assets/card.png');
      expect(result.srcDoc.match(/data:image\/png;base64,AQID/g)).toHaveLength(4);
    }
  });

  it('replaces history asset paths with signed URLs', () => {
    const result = buildCodePreviewDocument({
      framework: 'html',
      code: '<img src="./assets/card.png" /><style>.x{background:url(/assets/card.png)}</style>',
      assets: [],
      assetsManifest: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          sizeBytes: 3,
          storagePath: 'user/file/code-generations/gen/assets/card.png',
          signedUrl: 'https://example.test/signed-card.png',
          sourceKind: 'image-fill',
        },
      ],
    });

    expect(result.kind).toBe('previewable');
    if (result.kind === 'previewable') {
      expect(result.srcDoc).toContain('https://example.test/signed-card.png');
      expect(result.srcDoc).not.toContain('./assets/card.png');
      expect(result.srcDoc).not.toContain('/assets/card.png');
    }
  });

  it('does not preview history code when asset URLs are unavailable', () => {
    const result = buildCodePreviewDocument({
      framework: 'html',
      code: '<img src="./assets/card.png" />',
      assets: [],
      assetsManifest: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          sizeBytes: 3,
          sourceKind: 'image-fill',
        },
      ],
    });

    expect(result).toEqual({
      kind: 'unsupported',
      reason: 'History assets are unavailable for preview. Regenerate or download the bundle.',
    });
  });

  it('does not preview React in v1', () => {
    const result = buildCodePreviewDocument({
      framework: 'react',
      code: 'export function App() { return <div /> }',
      assets: [],
    });

    expect(result.kind).toBe('unsupported');
  });
});
