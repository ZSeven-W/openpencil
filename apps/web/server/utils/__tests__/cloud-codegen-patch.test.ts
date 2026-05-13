import { describe, expect, it, vi } from 'vitest';
import { generateCodePatch } from '../cloud-codegen-patch';
import type { CodegenTextCollector } from '../../../src/services/ai/code-generation-pipeline';
import type { CloudCodeGenerationDetail } from '../../../src/types/cloud';
import type { PenNode } from '../../../src/types/pen';

const selectedNode: PenNode = {
  id: 'node-1',
  type: 'rectangle',
  name: 'Problem Card',
  x: 0,
  y: 0,
  width: 120,
  height: 80,
};

const baseGeneration: CloudCodeGenerationDetail = {
  id: 'base-generation',
  fileId: 'file-1',
  pageId: 'page-1',
  framework: 'react',
  targetKind: 'selection',
  nodeIds: ['node-1'],
  targetHash: 'hash-1',
  documentRevision: 1,
  status: 'done',
  finalCode: 'export default function Design() { return <div>Old</div>; }',
  entryFile: 'design.tsx',
  degraded: false,
  assetsManifest: [],
  metadata: {},
  model: 'gpt-5.4',
  provider: 'openai',
  error: null,
  createdAt: '2026-05-13T08:00:00.000Z',
  completedAt: '2026-05-13T08:00:01.000Z',
  files: [
    {
      id: 'file-row-1',
      generationId: 'base-generation',
      path: 'design.tsx',
      language: 'tsx',
      role: 'entry',
      content: 'export default function Design() { return <div>Old</div>; }',
      sizeBytes: 56,
      orderIndex: 0,
      createdAt: '2026-05-13T08:00:00.000Z',
    },
  ],
  chunks: [],
};

describe('generateCodePatch', () => {
  it('asks for a complete updated source file using the base generation and selected nodes', async () => {
    const collectText: CodegenTextCollector = vi.fn(async () => (
      '```tsx\nexport default function Design() { return <div className="fixed">New</div>; }\n```'
    ));

    const result = await generateCodePatch({
      framework: 'react',
      nodes: [selectedNode],
      instruction: 'Fix selected spacing.',
      baseGeneration,
      model: 'gpt-5.4',
      provider: 'openai',
      collectText,
    });

    expect(collectText).toHaveBeenCalledWith(
      expect.stringContaining('Do not return a diff'),
      expect.stringContaining('Fix selected spacing.'),
      'gpt-5.4',
      'openai',
    );
    const mockCollectText = vi.mocked(collectText);
    expect(mockCollectText.mock.calls[0]?.[1]).toContain('"id":"node-1"');
    expect(mockCollectText.mock.calls[0]?.[1]).toContain('---FILE: design.tsx---');
    expect(result.code).toContain('className="fixed"');
    expect(result.code).not.toContain('```');
    expect(result.files).toEqual([
      expect.objectContaining({
        path: 'design.tsx',
        content: expect.stringContaining('className="fixed"'),
      }),
    ]);
  });

  it('returns a complete multi-file bundle for UniApp patch output', async () => {
    const collectText: CodegenTextCollector = vi.fn(async () =>
      [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
        '---FILE: pages/index/index.vue---',
        '<template><view class="fixed">New</view></template>',
      ].join('\n'),
    );

    const result = await generateCodePatch({
      framework: 'uniapp',
      nodes: [selectedNode],
      instruction: 'Fix selected card.',
      baseGeneration: { ...baseGeneration, framework: 'uniapp' },
      model: 'gpt-5.4',
      provider: 'openai',
      collectText,
    });

    expect(vi.mocked(collectText).mock.calls[0]?.[0]).toContain(
      'Return the complete updated multi-file bundle',
    );
    expect(result.files.map((file) => file.path)).toEqual([
      'App.vue',
      'pages.json',
      'pages/index/index.vue',
    ]);
  });
});
