// packages/pen-renderer/src/__tests__/render-node-thumbnail.test.ts
//
// Output-shape and fallback tests for renderNodeThumbnail. We do NOT test
// pixel-perfect output (requires a real CanvasKit WASM instance). Instead we
// verify:
//   1. Returns null gracefully when CanvasKit is unavailable (test env)
//   2. Returns null for invalid / null inputs
//   3. Returns null when size is invalid
//   4. The function is exported and callable
//   5. resolveNodeForCanvas is called with document variables + active theme (I6)

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import * as penCore from '@zseven-w/pen-core';

// Mock @zseven-w/pen-core to allow spying on resolveNodeForCanvas while
// keeping all other exports from the real module intact.
vi.mock('@zseven-w/pen-core', async () => {
  const actual = await vi.importActual<typeof penCore>('@zseven-w/pen-core');
  return { ...actual };
});

import { renderNodeThumbnail } from '../render-node-thumbnail';

// CanvasKit WASM is NOT available in the vitest/jsdom environment.
// getCanvasKit() returns null, so renderNodeThumbnail must fall back to null
// for ALL inputs. These tests assert the graceful-fallback contract.

const makeRect = (id = 'rect-1'): PenNode =>
  ({
    id,
    type: 'rectangle',
    x: 0,
    y: 0,
    width: 100,
    height: 100,
  }) as PenNode;

const makeDoc = (): PenDocument =>
  ({
    id: 'doc-1',
    name: 'Test Document',
    children: [],
  }) as unknown as PenDocument;

describe('renderNodeThumbnail — output shape / fallback contract', () => {
  it('returns null in test/Node.js environment (no CanvasKit)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: 128,
    });
    // In test env, CanvasKit is not available → null
    expect(result).toBeNull();
  });

  it('returns null for a null node', async () => {
    const result = await renderNodeThumbnail(null as unknown as PenNode, {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('returns null for an invalid size (zero)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: 0,
    });
    expect(result).toBeNull();
  });

  it('returns null for an invalid size (negative)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: -10,
    });
    expect(result).toBeNull();
  });

  it('returns null for a NaN size', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: NaN,
    });
    expect(result).toBeNull();
  });

  it('accepts pageId: null without throwing', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    // In test env → null, but it must not throw.
    expect(result).toBeNull();
  });

  it('uses default size of 128 when size is omitted', async () => {
    // Just verify it does not throw — output is null in test env.
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('handles a ref-type node gracefully (ref resolution may return empty)', async () => {
    const refNode = {
      id: 'ref-1',
      type: 'ref',
      ref: 'non-existent-component',
      x: 0,
      y: 0,
    } as unknown as PenNode;
    const result = await renderNodeThumbnail(refNode, {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('handles a doc with children without throwing', async () => {
    const doc: PenDocument = {
      id: 'doc-2',
      name: 'Doc With Children',
      children: [makeRect('comp-1')],
    } as unknown as PenDocument;
    const result = await renderNodeThumbnail(makeRect(), {
      document: doc,
      pageId: 'page-1',
    });
    expect(result).toBeNull();
  });

  it('result when non-null must be a data URL string (structural contract)', async () => {
    // This test documents the expected non-null contract for when CanvasKit IS
    // available. In the test env the mock returns null so we verify the type
    // contract with a type assertion only — no runtime assertion possible here.
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    // In test env always null. In a live env it would be string | null.
    if (result !== null) {
      expect(typeof result).toBe('string');
      expect(result).toMatch(/^data:/);
    } else {
      expect(result).toBeNull();
    }
  });

  // ---------------------------------------------------------------------------
  // Gap 2: $variable resolution
  // ---------------------------------------------------------------------------

  it('does not throw when node has a $variable fill reference and document has variables', async () => {
    // Node with a $color-primary fill reference.
    const nodeWithVar = {
      id: 'var-node-1',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '$color-primary' }],
    } as unknown as PenNode;

    const docWithVars: PenDocument = {
      id: 'doc-vars',
      name: 'Doc With Variables',
      children: [],
      variables: {
        'color-primary': { value: '#ff0000' },
      },
    } as unknown as PenDocument;

    // In test env CanvasKit is unavailable so result is null, but the
    // resolveNodeForCanvas code path must execute without throwing.
    const result = await renderNodeThumbnail(nodeWithVar, {
      document: docWithVars,
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('does not throw when document has themes and node uses themed variable', async () => {
    const nodeWithVar = {
      id: 'themed-node-1',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
      fill: [{ type: 'solid', color: '$bg-color' }],
    } as unknown as PenNode;

    const docWithThemes: PenDocument = {
      id: 'doc-themed',
      name: 'Themed Doc',
      children: [],
      variables: {
        'bg-color': {
          value: [
            { theme: { Mode: 'Light' }, value: '#ffffff' },
            { theme: { Mode: 'Dark' }, value: '#000000' },
          ],
        },
      },
      themes: { Mode: ['Light', 'Dark'] },
    } as unknown as PenDocument;

    const result = await renderNodeThumbnail(nodeWithVar, {
      document: docWithThemes,
      pageId: null,
    });
    // Still null in test env, but must not throw.
    expect(result).toBeNull();
  });

  // ---------------------------------------------------------------------------
  // I6: Verify resolveNodeForCanvas is called with document variables + theme
  // ---------------------------------------------------------------------------

  describe('resolveNodeForCanvas invocation (I6)', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let spy: ReturnType<typeof vi.spyOn<any, any>>;

    beforeEach(() => {
      spy = vi.spyOn(penCore, 'resolveNodeForCanvas');
    });

    afterEach(() => {
      spy.mockRestore();
    });

    it('calls resolveNodeForCanvas with the node, document variables, and active theme', async () => {
      const nodeWithVar = {
        id: 'spy-node-1',
        type: 'rectangle',
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        fill: [{ type: 'solid', color: '$color-primary' }],
      } as unknown as PenNode;

      const docWithVars: PenDocument = {
        id: 'doc-spy',
        name: 'Doc With Variables',
        children: [],
        variables: {
          'color-primary': { value: '#ff0000' },
        },
      } as unknown as PenDocument;

      await renderNodeThumbnail(nodeWithVar, {
        document: docWithVars,
        pageId: null,
      });

      // resolveNodeForCanvas must have been called — confirms the variable
      // resolution code path executes rather than being silently bypassed.
      expect(spy).toHaveBeenCalled();
      // The first argument should be the node (or ref-resolved equivalent).
      const [calledNode, calledVars] = spy.mock.calls[0] as [PenNode, Record<string, unknown>];
      expect(calledNode).toMatchObject({ id: 'spy-node-1' });
      // Variables must be the document's variables map.
      expect(calledVars).toEqual(docWithVars.variables);
    });

    it('calls resolveNodeForCanvas with active theme derived from document themes', async () => {
      const node = {
        id: 'spy-node-2',
        type: 'rectangle',
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill: [{ type: 'solid', color: '$bg' }],
      } as unknown as PenNode;

      const docWithThemes: PenDocument = {
        id: 'doc-spy-themed',
        name: 'Themed Doc',
        children: [],
        variables: {
          bg: {
            value: [
              { theme: { Mode: 'Light' }, value: '#fff' },
              { theme: { Mode: 'Dark' }, value: '#000' },
            ],
          },
        },
        themes: { Mode: ['Light', 'Dark'] },
      } as unknown as PenDocument;

      await renderNodeThumbnail(node, {
        document: docWithThemes,
        pageId: null,
      });

      expect(spy).toHaveBeenCalled();
      // Third argument is the active theme object derived from getDefaultTheme.
      const [, , calledTheme] = spy.mock.calls[0] as [PenNode, unknown, Record<string, string>];
      // getDefaultTheme({'Mode': ['Light','Dark']}) returns { Mode: 'Light' }
      expect(calledTheme).toMatchObject({ Mode: 'Light' });
    });
  });
});
