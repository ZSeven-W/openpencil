import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock canvas-text-measure to avoid CanvasKit WASM dependency in tests
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

// Mock image-search-pipeline so upsertNodesToCanvas doesn't try to hit the network
vi.mock('../image-search-pipeline', () => ({
  scanAndFillImages: vi.fn(() => Promise.resolve()),
  enqueueImageForSearch: vi.fn(),
  resetImageSearchQueue: vi.fn(),
}));

import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store';
import { upsertNodesToCanvas } from '../design-canvas-ops';
import '../role-definitions/index';
import type { PenDocument, PenNode } from '@/types/pen';

/**
 * Regression test for the theme detection path Codex caught.
 *
 * The bug: `sanitizeNodesForUpsert`'s helper `detectActiveDocumentTheme()`
 * used to read the cached `generationRootFrameId` module variable, which
 * is only populated by `resetGenerationRemapping()` at the start of an
 * orchestrator generation flow. For direct MCP/upsert call paths that
 * bypass that init, the cached value was stale or default — so theme
 * detection silently fell through to 'light' even on a dark page, and
 * an inserted navbar got the white role default stamped on top.
 *
 * The fix is to read the LIVE active-page primary frame on every call.
 * This test loads a dark page, immediately calls upsertNodesToCanvas
 * (with NO generation init), and asserts the inserted navbar got the
 * dark theme fill.
 */

function loadDocument(doc: PenDocument): void {
  useDocumentStore.getState().applyExternalDocument(doc);
}

/**
 * Build a minimal dark-theme document. The root frame has ONE
 * placeholder child so `isCanvasOnlyEmptyFrame()` returns false and
 * `upsertNodesToCanvas` exercises the normal addNode path (the path
 * with the theme-aware sanitize) instead of the empty-frame replace
 * shortcut.
 */
function makeDarkPageDoc(): PenDocument {
  return {
    version: '1.0.0',
    variables: {},
    pages: [
      {
        id: 'page-1',
        name: 'Page 1',
        children: [
          {
            id: DEFAULT_FRAME_ID,
            type: 'frame',
            name: 'Root',
            x: 0,
            y: 0,
            width: 375,
            height: 800,
            fill: [{ type: 'solid', color: '#111111' }],
            children: [
              {
                id: 'placeholder-content',
                type: 'frame',
                name: 'Placeholder',
                width: 100,
                height: 40,
              } as unknown as PenNode,
            ],
          } as unknown as PenNode,
        ],
      },
    ],
    children: [],
  } as unknown as PenDocument;
}

describe('design-canvas-ops theme detection (regression)', () => {
  beforeEach(() => {
    useDocumentStore.getState().newDocument();
  });

  it('upsertNodesToCanvas detects DARK theme from the live active page even without resetGenerationRemapping()', () => {
    // Load a dark page directly. We deliberately do NOT call
    // resetGenerationRemapping() — this is the path Codex flagged
    // (direct MCP upserts that bypass the orchestrator init).
    loadDocument(makeDarkPageDoc());

    // A navbar with NO fill — the LLM expected the dark page bg to
    // show through. Without theme detection, the role default would
    // stamp #FFFFFF on top.
    const navbar = {
      id: 'inserted-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 375,
      height: 56,
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('inserted-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#111111');
  });

  it('upsertNodesToCanvas detects LIGHT theme on a white page (baseline preserved)', () => {
    // Counter-test: same path but a white page must still produce the
    // original light-theme navbar default. Locks in that the dark fix
    // is purely additive.
    const lightDoc: PenDocument = {
      version: '1.0.0',
      variables: {},
      pages: [
        {
          id: 'page-1',
          name: 'Page 1',
          children: [
            {
              id: DEFAULT_FRAME_ID,
              type: 'frame',
              name: 'Root',
              x: 0,
              y: 0,
              width: 1200,
              height: 800,
              fill: [{ type: 'solid', color: '#FFFFFF' }],
              children: [
                {
                  id: 'placeholder-content',
                  type: 'frame',
                  name: 'Placeholder',
                  width: 100,
                  height: 40,
                } as unknown as PenNode,
              ],
            } as unknown as PenNode,
          ],
        },
      ],
      children: [],
    } as unknown as PenDocument;
    loadDocument(lightDoc);

    const navbar = {
      id: 'light-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 1200,
      height: 72,
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('light-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('does NOT overwrite an LLM-supplied navbar fill on either theme', () => {
    // applyDefaults overwrite-protection must still hold even with
    // theme awareness: a fill of #FF00AA stays #FF00AA regardless of
    // page bg.
    loadDocument(makeDarkPageDoc());

    const navbar = {
      id: 'magenta-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 375,
      height: 56,
      fill: [{ type: 'solid', color: '#FF00AA' }],
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('magenta-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored?.fill?.[0]?.color).toBe('#FF00AA');
  });
});
