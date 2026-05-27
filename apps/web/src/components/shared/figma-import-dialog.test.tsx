// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import FigmaImportDialog from './figma-import-dialog';
import type { FigmaDecodedFile } from '@/services/figma/figma-types';
import '@/i18n';

const mocks = vi.hoisted(() => {
  const decoded = {
    nodeChanges: [],
    blobs: [],
    imageFiles: new Map(),
  } as unknown as FigmaDecodedFile;

  return {
    decoded,
    parseFigFile: vi.fn(() => decoded),
    figmaToPenDocument: vi.fn(() => ({
      document: {
        version: '1',
        name: 'sample',
        pages: [{ id: 'figma-page-0', name: 'Landing', children: [] }],
        children: [],
      },
      warnings: [],
      imageBlobs: new Map(),
    })),
    figmaAllPagesToPenDocument: vi.fn(),
    getFigmaPages: vi.fn(() => [{ id: 'page-1', name: 'Landing', childCount: 2 }]),
    getFigmaPageLayers: vi.fn(() => [
      { id: 'frame-1', name: 'Checkout flow', type: 'FRAME', childCount: 4, visible: true },
      { id: 'frame-2', name: 'Archive mocks', type: 'FRAME', childCount: 8, visible: true },
    ]),
    replaceDocumentContent: vi.fn(),
    zoomToFitContent: vi.fn(),
  };
});

vi.mock('@/services/figma/fig-parser', () => ({
  parseFigFile: mocks.parseFigFile,
}));

vi.mock('@/services/figma/figma-node-mapper', () => ({
  figmaToPenDocument: mocks.figmaToPenDocument,
  figmaAllPagesToPenDocument: mocks.figmaAllPagesToPenDocument,
  getFigmaPages: mocks.getFigmaPages,
  getFigmaPageLayers: mocks.getFigmaPageLayers,
}));

vi.mock('@/services/figma/figma-image-resolver', () => ({
  resolveImageBlobs: vi.fn(() => 0),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      replaceDocumentContent: mocks.replaceDocumentContent,
    }),
  },
}));

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: {
    getState: () => ({
      pendingFigmaFile: null,
      setPendingFigmaFile: vi.fn(),
    }),
  },
}));

vi.mock('@/canvas/skia-engine-ref', () => ({
  getSkiaEngineRef: () => null,
  zoomToFitContent: mocks.zoomToFitContent,
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    callback(performance.now());
    return 0;
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('FigmaImportDialog', () => {
  it('imports only the selected top-level Figma layers', async () => {
    const { container } = render(<FigmaImportDialog open onClose={vi.fn()} />);
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;

    fireEvent.change(input, {
      target: {
        files: [
          {
            name: 'sample.fig',
            arrayBuffer: vi.fn(async () => new ArrayBuffer(4)),
          },
        ],
      },
    });

    expect(await screen.findByText('Checkout flow')).toBeTruthy();
    expect(screen.getByText('Archive mocks')).toBeTruthy();

    fireEvent.click(screen.getByRole('checkbox', { name: 'Archive mocks' }));
    fireEvent.click(screen.getByRole('button', { name: 'Import selected' }));

    await waitFor(() => {
      expect(mocks.figmaToPenDocument).toHaveBeenCalledWith(
        mocks.decoded,
        'sample',
        0,
        'preserve',
        { topLevelNodeIds: ['frame-1'] },
      );
      expect(mocks.replaceDocumentContent).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'sample' }),
        'sample.op',
      );
    });
  });
});
