import { describe, expect, it } from 'vitest';
import {
  buildCodegenFiles,
  parseDelimitedCodegenFiles,
  validateCodegenFiles,
} from '../codegen-files';

describe('codegen-files', () => {
  it('parses a delimited UniApp bundle into ordered files', () => {
    const files = parseDelimitedCodegenFiles(
      [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
        '---FILE: pages/index/index.vue---',
        '<template><view>Home</view></template>',
      ].join('\n'),
    );

    expect(files).toEqual([
      expect.objectContaining({ path: 'App.vue', language: 'vue', role: 'entry', orderIndex: 0 }),
      expect.objectContaining({
        path: 'pages.json',
        language: 'json',
        role: 'config',
        orderIndex: 1,
      }),
      expect.objectContaining({
        path: 'pages/index/index.vue',
        language: 'vue',
        role: 'page',
        orderIndex: 2,
      }),
    ]);
  });

  it('ignores unsafe file paths in delimited output', () => {
    const files = parseDelimitedCodegenFiles(
      [
        '---FILE: ../secret.ts---',
        'secret',
        '---FILE: /absolute.ts---',
        'absolute',
        '---FILE: pages/index/index.vue---',
        '<template><view /></template>',
      ].join('\n'),
    );

    expect(files.map((file) => file.path)).toEqual(['pages/index/index.vue']);
  });

  it('falls back to a single framework file when no delimiters exist', () => {
    const files = buildCodegenFiles({
      framework: 'react',
      code: 'export default function Design() { return null; }',
    });

    expect(files).toEqual([
      expect.objectContaining({
        path: 'design.tsx',
        language: 'tsx',
        role: 'entry',
        content: 'export default function Design() { return null; }',
      }),
    ]);
  });

  it('validates a complete UniApp file tree and page route mapping', () => {
    const files = buildCodegenFiles({
      framework: 'uniapp',
      code: [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: main.ts---',
        "import { createSSRApp } from 'vue';",
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
        '---FILE: manifest.json---',
        '{"name":"OpenPencil"}',
        '---FILE: uni.scss---',
        '$uni-color-primary: #0f172a;',
        '---FILE: pages/index/index.vue---',
        '<template><view>Home</view></template>',
      ].join('\n'),
    });

    expect(validateCodegenFiles('uniapp', files)).toEqual({ valid: true, issues: [] });
  });

  it('rejects UniApp bundles missing required files and page targets', () => {
    const files = buildCodegenFiles({
      framework: 'uniapp',
      code: [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/missing/index"}]}',
      ].join('\n'),
    });

    const validation = validateCodegenFiles('uniapp', files);

    expect(validation.valid).toBe(false);
    expect(validation.issues).toEqual(
      expect.arrayContaining([
        'UniApp output missing main.ts',
        'UniApp output missing manifest.json',
        'UniApp output missing uni.scss',
        'UniApp pages.json references missing page file pages/missing/index.vue',
      ]),
    );
  });

  it('rejects invalid UniApp pages.json', () => {
    const files = buildCodegenFiles({
      framework: 'uniapp',
      code: [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: main.ts---',
        "import { createSSRApp } from 'vue';",
        '---FILE: pages.json---',
        '{not json}',
        '---FILE: manifest.json---',
        '{"name":"OpenPencil"}',
        '---FILE: uni.scss---',
        '$uni-color-primary: #0f172a;',
        '---FILE: pages/index/index.vue---',
        '<template><view>Home</view></template>',
      ].join('\n'),
    });

    expect(validateCodegenFiles('uniapp', files).issues).toContain(
      'UniApp pages.json is invalid JSON',
    );
  });
});
