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
import {
  upsertNodesToCanvas,
  insertStreamingNode,
  resetGenerationRemapping,
} from '../design-canvas-ops';
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

  it('detects DARK theme from INPUT NODES even when store still has light default root', () => {
    // The bug Claude exposed in the fitness app rerun: a fresh
    // generation emits a new dark root frame INSIDE the upsert input
    // nodes, but the store still holds the empty light default root
    // from the brand-new document. The previous theme detector read
    // ONLY the live store, found a light root, and stamped white
    // cardFill defaults onto every card-family child of the new dark
    // root before it reached the store.
    //
    // The fix walks the input nodes BFS first and uses the outermost
    // frame's fill as the theme source, falling back to the live
    // store only when input has no readable fill. This test exercises
    // that path: brand-new document (light default), then upsert a
    // dark root with a child card that has NO fill of its own. The
    // child card must come out with the dark cardFill default
    // (#1A1A1A), not the light default (#FFFFFF).

    // newDocument() leaves the store with a default empty root that
    // has no explicit fill — this is the "store says light" state.
    const darkRoot = {
      id: 'fresh-dark-root',
      type: 'frame',
      name: 'Page',
      x: 0,
      y: 0,
      width: 375,
      height: 1166,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [
        {
          id: 'fresh-dark-card',
          type: 'frame',
          name: 'Heart Rate',
          role: 'card',
          width: 343,
          height: 200,
          children: [],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    upsertNodesToCanvas([darkRoot]);

    const card = useDocumentStore.getState().getNodeById('fresh-dark-card') as
      | (PenNode & { fill?: Array<{ color?: string }>; effects?: unknown[] })
      | undefined;
    expect(card).toBeDefined();
    // The card must get the DARK card fill (#1A1A1A), not the light
    // default (#FFFFFF). This is the entire bug fix in one assertion.
    expect(card?.fill?.[0]?.color).toBe('#1A1A1A');
    // And the card-shadow should still be applied (the role itself
    // wasn't stripped, just the fill picked the right theme).
    expect(card?.effects).toBeDefined();
    expect(Array.isArray(card?.effects) && (card?.effects?.length ?? 0) > 0).toBe(true);
  });

  // -------------------------------------------------------------------------
  // Streaming path coverage (insertStreamingNode)
  // -------------------------------------------------------------------------
  // Codex stop-hook caught: the streaming path bypasses
  // sanitizeNodesForInsert/Upsert entirely. Every per-node streaming
  // insert calls resolveNodeRole directly with a freshly-built RoleContext
  // that previously did not include `theme`. Result: card-family role
  // defaults always used cardFill('light')=#FFFFFF even on a dark page.
  // The fix passes `detectActiveDocumentTheme([node])` into the streaming
  // ctx, which reads the input node first then falls back to the live
  // store. These tests lock in both shapes.

  it('streaming: root node with dark fill propagates DARK theme to its own role defaults', () => {
    // First streaming node is the page root frame itself. Store still
    // has the empty default root from newDocument(). Input-first
    // detection sees the dark fill on the incoming root node and uses
    // it for theme. The root frame here has role: card to exercise the
    // theme-aware default path; cardFill('dark') = #1A1A1A would be
    // injected if fill were missing. (Author already supplied #0A0A0A,
    // which overwrite-protection keeps.)
    resetGenerationRemapping();
    const root = {
      id: 'stream-dark-root',
      type: 'frame',
      name: 'Page',
      x: 0,
      y: 0,
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [],
    } as unknown as PenNode;

    insertStreamingNode(root, null);
    // Now the root is committed to the store. Stream a fillless card
    // child — it should pick the dark theme cardFill default.
    const card = {
      id: 'stream-dark-card',
      type: 'frame',
      name: 'Heart Card',
      role: 'card',
      width: 343,
      height: 200,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(card, 'stream-dark-root');

    const stored = useDocumentStore.getState().getNodeById('stream-dark-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('streaming: card child under a dark page root gets DARK cardFill default (no white bar)', () => {
    // Same as above but using a pre-loaded dark document instead of
    // streaming the root. This is the closer match to a fresh
    // generation where the page root is already on canvas (e.g.
    // re-running on the same active page).
    loadDocument(makeDarkPageDoc());
    resetGenerationRemapping();

    const card = {
      id: 'stream-card-on-dark',
      type: 'frame',
      name: 'Activity',
      role: 'card',
      width: 343,
      height: 180,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(card, DEFAULT_FRAME_ID);

    const stored = useDocumentStore.getState().getNodeById('stream-card-on-dark') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('streaming: navbar inferred from name "Header" inside a dark page root gets DARK navbarFill', () => {
    // The exact failing case from the fitness app rerun: a streamed
    // child named "Header" was inferred to navbar role and got the
    // light #FFFFFF fill default. After the fix it should pick
    // navbarFill('dark') = #111111.
    loadDocument(makeDarkPageDoc());
    resetGenerationRemapping();

    const header = {
      id: 'stream-page-header',
      type: 'frame',
      name: 'Header', // name inference → navbar
      width: 375,
      height: 56,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(header, DEFAULT_FRAME_ID);

    const stored = useDocumentStore.getState().getNodeById('stream-page-header') as
      | (PenNode & { role?: string; fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored?.role).toBe('navbar');
    expect(stored?.fill?.[0]?.color).toBe('#111111');
  });

  it('does NOT let a small white nested card outvote the dark page root for theme detection', () => {
    // BFS guarantee: when the input forest has multiple frames with
    // fills, the OUTERMOST one wins. A dark page root containing a
    // small white card must still detect as 'dark' so OTHER cards
    // (the ones with no fill) get the dark default.
    const darkRoot = {
      id: 'mixed-root',
      type: 'frame',
      name: 'Page',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [
        // An explicit white card (LLM author intent — kept verbatim)
        {
          id: 'mixed-white-card',
          type: 'frame',
          name: 'White Card',
          role: 'card',
          width: 343,
          height: 100,
          fill: [{ type: 'solid', color: '#FFFFFF' }],
          children: [],
        } as unknown as PenNode,
        // A fillless card that should get the DARK default
        {
          id: 'mixed-default-card',
          type: 'frame',
          name: 'Default Card',
          role: 'card',
          width: 343,
          height: 100,
          children: [],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    upsertNodesToCanvas([darkRoot]);

    const whiteCard = useDocumentStore.getState().getNodeById('mixed-white-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    const defaultCard = useDocumentStore.getState().getNodeById('mixed-default-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;

    // Author intent preserved
    expect(whiteCard?.fill?.[0]?.color).toBe('#FFFFFF');
    // Outermost dark root wins for theme detection → fillless card
    // gets dark default, not light
    expect(defaultCard?.fill?.[0]?.color).toBe('#1A1A1A');
  });
});
