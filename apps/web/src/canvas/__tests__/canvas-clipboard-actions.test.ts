import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  copySelectedNodeIdsToClipboard,
  copySelectedNodesToInternalClipboard,
} from '@/canvas/canvas-clipboard-actions';
import { useCanvasStore } from '@/stores/canvas-store';
import { createEmptyDocument } from '@/stores/document-tree-utils';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

const rect: PenNode = {
  id: 'rect-1',
  type: 'rectangle',
  name: 'Rect 1',
  x: 12,
  y: 24,
  width: 100,
  height: 80,
  fill: [{ type: 'solid', color: '#ffffff' }],
} as PenNode;

const ellipse: PenNode = {
  id: 'ellipse-1',
  type: 'ellipse',
  name: 'Ellipse 1',
  x: 48,
  y: 64,
  width: 72,
  height: 72,
  fill: [{ type: 'solid', color: '#000000' }],
} as PenNode;

function resetStores() {
  vi.unstubAllGlobals();
  useCanvasStore.setState({
    clipboard: [],
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
    },
  });

  useDocumentStore.setState({
    document: {
      ...createEmptyDocument(),
      pages: [{ id: 'page-1', name: 'Page 1', children: [rect, ellipse] }],
      children: [],
    },
    isDirty: false,
    fileHandle: null,
    fileName: null,
    filePath: null,
  } as any);
}

describe('copySelectedNodesToInternalClipboard', () => {
  beforeEach(() => {
    resetStores();
  });

  it('copies selected document nodes into the internal clipboard', () => {
    useCanvasStore.getState().setSelection(['rect-1', 'ellipse-1'], 'ellipse-1');

    const copiedCount = copySelectedNodesToInternalClipboard();

    expect(copiedCount).toBe(2);
    expect(useCanvasStore.getState().clipboard.map((node) => node.id)).toEqual([
      'rect-1',
      'ellipse-1',
    ]);
  });

  it('stores clones so later document mutations do not alter clipboard nodes', () => {
    useCanvasStore.getState().setSelection(['rect-1'], 'rect-1');

    copySelectedNodesToInternalClipboard();
    useDocumentStore.getState().updateNode('rect-1', { x: 999 });

    expect(useCanvasStore.getState().clipboard[0]?.x).toBe(12);
  });

  it('does nothing when the selection has no existing nodes', () => {
    useCanvasStore.setState({ clipboard: [rect] });
    useCanvasStore.getState().setSelection(['missing-1'], 'missing-1');

    const copiedCount = copySelectedNodesToInternalClipboard();

    expect(copiedCount).toBe(0);
    expect(useCanvasStore.getState().clipboard).toEqual([rect]);
  });
});

describe('copySelectedNodeIdsToClipboard', () => {
  beforeEach(() => {
    resetStores();
  });

  it('writes the selected node id to the system clipboard', async () => {
    const writeText = vi.fn(async (_text: string) => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    useCanvasStore.getState().setSelection(['rect-1'], 'rect-1');

    const result = await copySelectedNodeIdsToClipboard();

    expect(result.copiedIds).toEqual(['rect-1']);
    expect(result.systemClipboardWritten).toBe(true);
    expect(useCanvasStore.getState().clipboard).toEqual([]);
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith('rect-1');
  });

  it('writes multiple selected node ids as a JSON array for MCP array arguments', async () => {
    const writeText = vi.fn(async (_text: string) => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    useCanvasStore.getState().setSelection(['rect-1', 'ellipse-1'], 'ellipse-1');

    const result = await copySelectedNodeIdsToClipboard();

    expect(result.copiedIds).toEqual(['rect-1', 'ellipse-1']);
    expect(result.systemClipboardWritten).toBe(true);
    expect(writeText).toHaveBeenCalledWith('["rect-1","ellipse-1"]');
  });

  it('falls back to the Electron clipboard bridge when browser clipboard writes fail', async () => {
    const writeText = vi.fn(async (_text: string) => {
      throw new Error('permission denied');
    });
    const writeClipboardText = vi.fn(async (_text: string) => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    vi.stubGlobal('window', {
      electronAPI: { writeClipboardText },
    });
    useCanvasStore.getState().setSelection(['ellipse-1'], 'ellipse-1');

    const result = await copySelectedNodeIdsToClipboard();

    expect(result.copiedIds).toEqual(['ellipse-1']);
    expect(result.systemClipboardWritten).toBe(true);
    expect(writeClipboardText).toHaveBeenCalledWith('ellipse-1');
  });
});
