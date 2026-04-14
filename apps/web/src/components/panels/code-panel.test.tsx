// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PenNode } from '@/types/pen';

const selectedNode: PenNode = {
  id: 'node-1',
  type: 'rectangle',
  x: 0,
  y: 0,
  width: 100,
  height: 80,
};

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: (selector: (state: unknown) => unknown) =>
    selector({
      selection: { selectedIds: ['node-1'] },
      activePageId: 'page-1',
    }),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: (selector: (state: unknown) => unknown) =>
    selector({
      getNodeById: (id: string) => (id === 'node-1' ? selectedNode : undefined),
      document: { variables: {} },
    }),
  getActivePageChildren: () => [selectedNode],
}));

vi.mock('@/stores/ai-store', () => ({
  useAIStore: (selector: (state: unknown) => unknown) =>
    selector({
      model: 'test-model',
      modelGroups: [{ provider: 'builtin', models: [{ value: 'test-model' }] }],
    }),
}));

const generatedResult = {
  code: 'export default function Design() { return null; }',
  degraded: false,
  assets: [
    {
      id: 'asset-1',
      relativePath: './assets/hero-card-1.png',
      zipPath: 'assets/hero-card-1.png',
      mimeType: 'image/png',
      bytes: new Uint8Array([1, 2, 3]),
      sourceNodeId: 'node-1',
      sourceNodeName: 'Hero Card',
      sourceKind: 'image-fill' as const,
    },
  ],
};

const generateCodeMock = vi.fn(async (args: unknown[]) => {
  const onProgress = args[3] as ((event: Record<string, unknown>) => void) | undefined;
  onProgress?.({
    step: 'complete',
    finalCode: generatedResult.code,
    degraded: generatedResult.degraded,
  });
  return generatedResult;
});

vi.mock('@/services/ai/code-generation-pipeline', () => ({
  generateCode: (...args: unknown[]) => generateCodeMock(args),
}));

vi.mock('@/services/ai/codegen-assets', () => ({
  buildCodegenBundleManifest: vi.fn(async () => ({ version: 2, assets: [] })),
}));

vi.mock('@/services/ai/structure-bundle', () => ({
  buildAIStructureBundle: vi.fn(async () => ({
    fileName: 'ai-structure-bundle.zip',
    zipEntries: {},
  })),
  encodeAIStructureBundleZip: vi.fn(() => new ArrayBuffer(0)),
}));

vi.mock('@/utils/syntax-highlight', () => ({
  highlightCode: (code: string) => code,
}));

import CodePanel from './code-panel';

afterEach(() => {
  cleanup();
});

describe('CodePanel export affordances', () => {
  it('shows the AI bundle export action in the empty state', () => {
    render(<CodePanel />);

    expect(screen.getByRole('button', { name: /Export AI Bundle/i })).toBeTruthy();
  });

  it('shows AI bundle and zip download actions after generation with assets', async () => {
    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: /AI Bundle/i }).length).toBeGreaterThan(0);
      expect(screen.getByRole('button', { name: /Download ZIP/i })).toBeTruthy();
    });
  });
});
