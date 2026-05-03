import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import {
  assignIdsRecursively,
  buildAlert,
  buildBodyText,
  buildComment,
  buildFormField,
  buildHeading,
  buildListRow,
  buildModalShell,
  buildNotificationRow,
  buildQuoteBlock,
  buildTextarea,
  buildToast,
  buildTooltip,
  type ElementTree,
} from '../element-builders/index.js';
import { computeLayoutPositions } from '../layout/engine.js';

/**
 * Boundary-value fuzz for text-carrying builders. Real-world LLM
 * output very occasionally ships strings in the thousands-of-chars
 * range (a whole paragraph stuffed into one tool call, a long
 * comment body, an unwieldy FAQ answer). This test feeds 10 000-char
 * strings into every builder that accepts user text and verifies:
 *
 *   1. The builder returns a valid ElementTree (no throw)
 *   2. `computeLayoutPositions` doesn't throw
 *   3. No NaN / Infinity coordinates leak in
 *   4. The text content is preserved verbatim (no truncation inside
 *      the builder — truncation is a caller / renderer concern)
 *
 * Each builder gets a dedicated row so a regression points at the
 * exact tool.
 */

const LONG_STRING = 'Lorem ipsum '.repeat(850).trim(); // ≈ 10 200 chars

interface LongStringCase {
  name: string;
  build: (text: string) => ElementTree;
  /** Which field in the tree should hold the string (for verification). */
  verify: (tree: PenNode) => string | undefined;
}

function findTextByRole(n: PenNode, role: string): PenNode | undefined {
  const r = (n as PenNode & { role?: string }).role;
  if (r === role && n.type === 'text') return n;
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  for (const c of children) {
    const hit = findTextByRole(c, role);
    if (hit) return hit;
  }
  return undefined;
}

function textContentByRole(tree: PenNode, role: string): string | undefined {
  const node = findTextByRole(tree, role);
  if (!node) return undefined;
  return (node as PenNode & { content?: string }).content;
}

const CASES: LongStringCase[] = [
  {
    name: 'heading',
    build: (text) => buildHeading({ content: text }),
    verify: (t) => (t as PenNode & { content?: string }).content,
  },
  {
    name: 'body-text',
    build: (text) => buildBodyText({ content: text }),
    verify: (t) => (t as PenNode & { content?: string }).content,
  },
  {
    name: 'list-row (subtitle)',
    build: (text) => buildListRow({ title: 'Short', subtitle: text }),
    verify: (t) => {
      const walk = (n: PenNode): PenNode | undefined => {
        if (n.type === 'text' && (n as PenNode & { name?: string }).name === 'Subtitle') return n;
        const kids = (n as PenNode & { children?: PenNode[] }).children ?? [];
        for (const c of kids) {
          const hit = walk(c);
          if (hit) return hit;
        }
        return undefined;
      };
      const hit = walk(t);
      return hit ? (hit as PenNode & { content?: string }).content : undefined;
    },
  },
  {
    name: 'form-field (placeholder)',
    build: (text) => buildFormField({ label: 'Email', placeholder: text }),
    verify: (t) => {
      // Placeholder isn't role-tagged; find by name.
      const walk = (n: PenNode): PenNode | undefined => {
        if (n.type === 'text' && (n as PenNode & { name?: string }).name === 'Placeholder')
          return n;
        const kids = (n as PenNode & { children?: PenNode[] }).children ?? [];
        for (const c of kids) {
          const hit = walk(c);
          if (hit) return hit;
        }
        return undefined;
      };
      const hit = walk(t);
      return hit ? (hit as PenNode & { content?: string }).content : undefined;
    },
  },
  {
    name: 'textarea (placeholder)',
    build: (text) => buildTextarea({ label: 'Bio', placeholder: text }),
    verify: (t) => {
      const walk = (n: PenNode): PenNode | undefined => {
        if (n.type === 'text' && (n as PenNode & { name?: string }).name === 'Placeholder')
          return n;
        const kids = (n as PenNode & { children?: PenNode[] }).children ?? [];
        for (const c of kids) {
          const hit = walk(c);
          if (hit) return hit;
        }
        return undefined;
      };
      const hit = walk(t);
      return hit ? (hit as PenNode & { content?: string }).content : undefined;
    },
  },
  {
    name: 'alert (message)',
    build: (text) => buildAlert({ message: text }),
    verify: (t) => textContentByRole(t, 'alert-message'),
  },
  {
    name: 'toast (message)',
    build: (text) => buildToast({ message: text }),
    verify: (t) => textContentByRole(t, 'toast-message'),
  },
  {
    name: 'quote-block',
    build: (text) => buildQuoteBlock({ quote: text, author: 'Anon' }),
    verify: (t) => textContentByRole(t, 'quote-text'),
  },
  {
    name: 'tooltip',
    build: (text) => buildTooltip({ text }),
    verify: (t) => textContentByRole(t, 'tooltip-text'),
  },
  {
    name: 'comment (body)',
    build: (text) => buildComment({ author: 'A', body: text }),
    verify: (t) => textContentByRole(t, 'comment-body'),
  },
  {
    name: 'notification-row (body)',
    build: (text) => buildNotificationRow({ title: 'T', body: text }),
    verify: (t) => textContentByRole(t, 'notification-body'),
  },
  {
    name: 'modal-shell (subtitle)',
    build: (text) => buildModalShell({ title: 'Title', subtitle: text }),
    verify: (t) => textContentByRole(t, 'modal-subtitle'),
  },
];

function hasFiniteCoords(n: PenNode): boolean {
  const r = n as PenNode & { x?: number; y?: number; width?: unknown; height?: unknown };
  if (typeof r.x === 'number' && !Number.isFinite(r.x)) return false;
  if (typeof r.y === 'number' && !Number.isFinite(r.y)) return false;
  if (typeof r.width === 'number' && !Number.isFinite(r.width)) return false;
  if (typeof r.height === 'number' && !Number.isFinite(r.height)) return false;
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  return children.every(hasFiniteCoords);
}

describe('element builders — 10k character input boundary', () => {
  for (const c of CASES) {
    it(`${c.name}: 10k-char input → no throw, preserved verbatim, finite layout`, () => {
      // Build
      const tree = c.build(LONG_STRING) as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);

      // Wrap in a fixed container so layout math has concrete parent bounds
      const parent: PenNode = {
        id: 'root',
        type: 'frame',
        name: 'Page',
        x: 0,
        y: 0,
        width: 375,
        height: 812,
        layout: 'vertical',
        children: [tree],
      } as unknown as PenNode;

      // Layout doesn't throw
      expect(() => {
        const kids = (parent as PenNode & { children?: PenNode[] }).children ?? [];
        computeLayoutPositions(parent, kids);
      }).not.toThrow();

      // Finite coords
      expect(hasFiniteCoords(parent), `${c.name}: non-finite coord`).toBe(true);

      // Content preserved verbatim (no builder-side truncation)
      const found = c.verify(tree);
      expect(found, `${c.name}: verify() returned undefined`).toBeDefined();
      expect(found).toBe(LONG_STRING);
    });
  }
});

describe('element builders — 100k character stress (outer boundary)', () => {
  // Cheap sanity on an order-of-magnitude larger input — the
  // builders should still terminate quickly (no O(n²) hidden in a
  // character-by-character scan).
  it('buildHeading accepts 100k-char content without timing out', () => {
    const huge = 'x'.repeat(100_000);
    const t0 = Date.now();
    const h = buildHeading({ content: huge });
    const elapsed = Date.now() - t0;
    expect(elapsed, `buildHeading 100k took ${elapsed}ms (should be <100)`).toBeLessThan(100);
    expect((h as Record<string, unknown>).content).toBe(huge);
  });

  it('buildBodyText accepts 100k-char content without timing out', () => {
    const huge = 'y'.repeat(100_000);
    const t0 = Date.now();
    const b = buildBodyText({ content: huge });
    const elapsed = Date.now() - t0;
    expect(elapsed).toBeLessThan(100);
    expect((b as Record<string, unknown>).content).toBe(huge);
  });
});
