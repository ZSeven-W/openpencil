import { describe, it, expect, beforeEach } from 'vitest';
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store';
import { runPreValidationFixes } from '../design-pre-validation';
import type { PenDocument, PenNode } from '@/types/pen';

/**
 * Baseline behavior tests for design-pre-validation.
 *
 * Purpose: lock in the current behavior of `runPreValidationFixes()` so we
 * can refactor detect logic into pure functions without silently breaking
 * anything. Do not modify assertions when you do the refactor — if they
 * fail, the refactor has changed behavior.
 *
 * Fixture note: `runPreValidationFixes()` calls `getNodeById(DEFAULT_FRAME_ID)`
 * ('root-frame') on the active page's children. Each fixture therefore wraps
 * the test nodes inside a root frame with id === DEFAULT_FRAME_ID so that
 * the function can find its entry point.
 */

function loadDocument(doc: PenDocument): void {
  useDocumentStore.getState().applyExternalDocument(doc);
}

/**
 * Build a minimal PenDocument whose active page contains a single root frame
 * (id === DEFAULT_FRAME_ID) with `children` as its descendants.
 */
function makeDoc(children: PenNode[], variables: Record<string, unknown> = {}): PenDocument {
  return {
    version: '1.0.0',
    variables,
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
            children,
          } as unknown as PenNode,
        ],
      },
    ],
    children: [],
  } as unknown as PenDocument;
}

describe('design-pre-validation (baseline)', () => {
  beforeEach(() => {
    useDocumentStore.getState().newDocument();
  });

  it('returns 0 fixes for an empty document', () => {
    loadDocument(makeDoc([]));
    expect(runPreValidationFixes()).toBe(0);
  });

  it('adds stroke to a frame with same fill as parent (invisible container)', () => {
    loadDocument(
      makeDoc([
        {
          id: 'invisible-card',
          type: 'frame',
          name: 'root',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'vertical',
          children: [
            {
              id: 'inner',
              type: 'frame',
              name: 'inner-card',
              fill: [{ type: 'solid', color: '#FAFAFA' }],
              layout: 'horizontal',
              children: [
                { id: 'label', type: 'text', text: 'x', fontSize: 14 } as unknown as PenNode,
              ],
            },
          ],
        } as unknown as PenNode,
      ]),
    );
    const fixes = runPreValidationFixes();
    expect(fixes).toBeGreaterThanOrEqual(1);
    const inner = useDocumentStore.getState().getNodeById('inner') as PenNode & {
      stroke?: { thickness?: number };
    };
    expect(inner?.stroke?.thickness).toBe(1);
  });

  it('removes path nodes without geometry (empty paths)', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FFF' }],
          children: [
            { id: 'empty-path', type: 'path' } as unknown as PenNode,
            { id: 'good-path', type: 'path', d: 'M0 0 L10 10' } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    runPreValidationFixes();
    expect(useDocumentStore.getState().getNodeById('empty-path')).toBeUndefined();
    expect(useDocumentStore.getState().getNodeById('good-path')).toBeDefined();
  });

  it('rewrites explicit text height to fit_content', () => {
    loadDocument(
      makeDoc([
        {
          id: 'wrapper',
          type: 'frame',
          children: [
            {
              id: 'tall-text',
              type: 'text',
              text: 'hi',
              height: 120,
              fontSize: 16,
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    runPreValidationFixes();
    const node = useDocumentStore.getState().getNodeById('tall-text') as PenNode & {
      height?: unknown;
    };
    expect(node?.height).toBe('fit_content');
  });

  it('does not remove status-bar nodes even when detected as empty path', () => {
    loadDocument(
      makeDoc([
        {
          id: 'phone',
          type: 'frame',
          children: [
            {
              id: 'status',
              type: 'path',
              role: 'status-bar',
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    runPreValidationFixes();
    expect(useDocumentStore.getState().getNodeById('status')).toBeDefined();
  });

  it('does NOT silently rewrite a rounded chrome cornerRadius across role boundaries', () => {
    // The case Codex flagged: a tab-bar with intentionally rounded corners
    // (cornerRadius=16) sitting among square content sections
    // (cornerRadius=0). The detector's loose pass DOES emit an issue for
    // the tab-bar (so debug tools surface it), but at 'info' severity.
    // Pre-validation must NOT auto-apply 'info' issues — silently
    // rewriting the tab-bar to cornerRadius=0 would destroy the
    // intentional design choice.
    const section = (id: string) =>
      ({
        id,
        type: 'frame',
        role: 'section',
        cornerRadius: 0,
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: id, fontSize: 14 } as unknown as PenNode],
      }) as unknown as PenNode;

    loadDocument(
      makeDoc([
        section('s1'),
        section('s2'),
        section('s3'),
        section('s4'),
        section('s5'),
        {
          id: 'tabs',
          type: 'frame',
          role: 'bottom-tab-bar',
          cornerRadius: 16,
          height: 62,
          layout: 'horizontal',
          children: [
            { id: 'tab-t', type: 'text', text: 'Tabs', fontSize: 12 } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );

    // The tab-bar's cornerRadius is detected (info severity) but not
    // applied. Returned count should be 0 — nothing was actually changed.
    const applied = runPreValidationFixes();
    expect(applied).toBe(0);

    const tabBar = useDocumentStore.getState().getNodeById('tabs') as PenNode & {
      cornerRadius?: number;
    };
    expect(tabBar?.cornerRadius).toBe(16);
  });

  it('returned count reflects fixes ACTUALLY applied, not detected', () => {
    // Mixed document: one warning-severity issue (text explicit height)
    // that gets applied + one info-severity issue (cross-role cornerRadius
    // outlier) that does NOT. The returned count must be 1, not 2.
    //
    // The 4 sections each have a UNIQUE role, so the strict pass puts
    // them in singleton groups (skipped), and only the loose pass fires
    // on cornerRadius — at info severity, so it's never applied.
    const section = (id: string, role: string, cornerRadius: number) =>
      ({
        id,
        type: 'frame',
        role,
        cornerRadius,
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: id, fontSize: 14 } as unknown as PenNode],
      }) as unknown as PenNode;

    loadDocument(
      makeDoc([
        section('s1', 'hero', 0),
        section('s2', 'features', 0),
        section('s3', 'cta', 0),
        section('s4', 'footer', 8), // info-severity loose-pass issue (cross-role)
        {
          id: 'tall-text-wrap',
          type: 'frame',
          children: [
            {
              id: 'tall-text',
              type: 'text',
              text: 'hi',
              height: 99, // warning-severity (will be applied)
              fontSize: 16,
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );

    const applied = runPreValidationFixes();
    // Only the text-explicit-height fix counts; the cornerRadius info
    // issue on s4 is detected but skipped, so it must not inflate the
    // count.
    expect(applied).toBe(1);

    // Verify the actual side effects: text height was rewritten…
    const text = useDocumentStore.getState().getNodeById('tall-text') as PenNode & {
      height?: unknown;
    };
    expect(text?.height).toBe('fit_content');
    // …and the s4 cornerRadius was NOT touched.
    const s4 = useDocumentStore.getState().getNodeById('s4') as PenNode & {
      cornerRadius?: number;
    };
    expect(s4?.cornerRadius).toBe(8);
  });

  // Aesthetic detector pipeline tests — added 2026-05-10 alongside the new
  // detectors (53435bf7 / 7aef1b14 / cd1e4325 / ad025c95). Each verifies
  // the detector → applyFixes → store mutation chain end-to-end.

  it('rotation: auto-fixes a tilted UI frame to rotation=0', () => {
    loadDocument(
      makeDoc([
        {
          id: 'tilted',
          type: 'frame',
          name: 'Card',
          rotation: 12,
          children: [
            { id: 'content', type: 'text', text: 'hello', fontSize: 14 } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const node = useDocumentStore.getState().getNodeById('tilted') as PenNode & {
      rotation?: number;
    };
    expect(node?.rotation).toBe(0);
  });

  it('text-corner-radius: clears cornerRadius on a text node', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          children: [
            {
              id: 'rounded-text',
              type: 'text',
              content: 'hi',
              cornerRadius: 8,
              fontSize: 14,
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const text = useDocumentStore.getState().getNodeById('rounded-text') as PenNode & {
      cornerRadius?: number;
    };
    expect(text?.cornerRadius).toBeUndefined();
  });

  it('text-stroke: clears stroke on a text node', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          children: [
            {
              id: 'outlined-text',
              type: 'text',
              content: 'hi',
              fontSize: 14,
              stroke: { thickness: 1, fill: [{ type: 'solid', color: '#000' }] },
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const text = useDocumentStore.getState().getNodeById('outlined-text') as PenNode & {
      stroke?: unknown;
    };
    expect(text?.stroke).toBeUndefined();
  });

  it('mixed-sibling-corner-radius: rewrites the outlier card to the modal value', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          children: [
            { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
            { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
            { id: 'c3', type: 'frame', role: 'card', cornerRadius: 12, children: [] },
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const c3 = useDocumentStore.getState().getNodeById('c3') as PenNode & {
      cornerRadius?: number;
    };
    expect(c3?.cornerRadius).toBe(8);
  });

  it('text-effect: clears effects array on a text node', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          children: [
            {
              id: 'shadowed-text',
              type: 'text',
              content: 'hi',
              fontSize: 14,
              effects: [{ type: 'shadow', color: '#000', offsetX: 0, offsetY: 4, radius: 8 }],
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const text = useDocumentStore.getState().getNodeById('shadowed-text') as PenNode & {
      effects?: unknown;
    };
    expect(text?.effects).toBeUndefined();
  });

  it('mixed-sibling-padding: rewrites the padding outlier to the modal value', () => {
    loadDocument(
      makeDoc([
        {
          id: 'parent',
          type: 'frame',
          children: [
            { id: 'c1', type: 'frame', role: 'card', padding: 16, children: [] },
            { id: 'c2', type: 'frame', role: 'card', padding: 16, children: [] },
            { id: 'c3', type: 'frame', role: 'card', padding: 20, children: [] },
          ],
        } as unknown as PenNode,
      ]),
    );
    expect(runPreValidationFixes()).toBeGreaterThanOrEqual(1);
    const c3 = useDocumentStore.getState().getNodeById('c3') as PenNode & {
      padding?: unknown;
    };
    // Modal collapsed back to scalar 16 (all 4 sides equal)
    expect(c3?.padding).toBe(16);
  });
});
