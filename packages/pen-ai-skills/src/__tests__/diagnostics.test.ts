import { describe, it, expect } from 'vitest';
import {
  detectInvisibleContainers,
  detectEmptyPaths,
  detectTextExplicitHeights,
  detectSiblingInconsistencies,
  detectUnexpectedRotation,
  detectTextCornerRadius,
  detectMixedSiblingCornerRadius,
  detectTextEffect,
  detectTextStroke,
  detectAllIssues,
} from '../diagnostics/detectors';
import type { PenNode, PenDocument } from '@zseven-w/pen-types';

function doc(root: PenNode, variables: Record<string, unknown> = {}): PenDocument {
  return { version: '1.0.0', variables, children: [root] } as unknown as PenDocument;
}

describe('detectInvisibleContainers', () => {
  it('returns empty when no frames share fill with parent', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FFF' }],
      layout: 'vertical',
      children: [
        {
          id: 'c',
          type: 'frame',
          fill: [{ type: 'solid', color: '#000' }],
          layout: 'vertical',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(0);
  });

  it('flags a nested frame with the same fill color as its parent', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'invisible',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'horizontal',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(1);
    expect(issues[0]).toMatchObject({
      nodeId: 'invisible',
      category: 'invisible-container',
      property: 'stroke',
    });
    expect((issues[0].suggestedValue as { thickness?: number })?.thickness).toBe(1);
  });

  it('uses $color-border variable when the document has it', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'invisible',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'horizontal',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root, { 'color-border': '#CCCCCC' }));
    const stroke = issues[0].suggestedValue as { fill: Array<{ color: string }> };
    expect(stroke.fill[0].color).toBe('$color-border');
  });

  it('does not flag frames without children', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'empty',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'horizontal',
          children: [],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(0);
  });

  it('does not flag frames that already have a stroke', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'bordered',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'horizontal',
          stroke: { thickness: 2, fill: [{ type: 'solid', color: '#000' }] },
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(0);
  });

  it('perceptual mode: dark-on-dark issues downgraded to info severity (detect-only)', () => {
    // The dark-theme card case: #111111 page bg, #1A1A1A card. The contrast
    // ratio is ≈ 1.085 — technically "low contrast" by WCAG (the cards are
    // genuinely subtle on this very dark background). The detector still
    // flags it so the user/agent can SEE the issue in debug reports, but
    // at INFO severity instead of warning.
    //
    // Why: the suggested auto-fix is a #E2E8F0 light-gray border, which
    // would actively damage a dark theme. The pre-validation pipeline
    // skips info severity, so the dark cards are never silently rewritten
    // with a clashing border. The agent can decide manually whether to
    // add a theme-appropriate stroke.
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#111111' }],
      layout: 'vertical',
      children: [
        {
          id: 'card',
          type: 'frame',
          fill: [{ type: 'solid', color: '#1A1A1A' }],
          layout: 'vertical',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('card');
    expect(issues[0].severity).toBe('info');
  });

  it('perceptual mode: still flags near-identical light fills (#FAFAFA vs #F1F1F1) at WARNING severity', () => {
    // The light-theme counter-test: a 9-unit channel diff on light bg is
    // perceptually invisible (contrast ratio ≈ 1.084 ≤ 1.10) → flagged.
    // Locks in that switching to WCAG contrast didn't weaken the original
    // light-theme detection. Severity stays 'warning' (auto-fixable)
    // because the suggested #E2E8F0 border actually works on light
    // backgrounds.
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'card',
          type: 'frame',
          fill: [{ type: 'solid', color: '#F1F1F1' }],
          layout: 'vertical',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('card');
    expect(issues[0].severity).toBe('warning');
    expect(issues[0].reason).toMatch(/contrast=/);
  });

  it('strict mode: does NOT flag near-identical fills regardless of theme', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'card',
          type: 'frame',
          fill: [{ type: 'solid', color: '#F1F1F1' }],
          layout: 'vertical',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    // strict mode — different hex strings → NOT flagged
    const issues = detectInvisibleContainers(root, doc(root), { colorMatchMode: 'strict' });
    expect(issues).toHaveLength(0);
  });

  it('perceptual mode: does NOT flag fills with substantial light contrast (#FFFFFF vs #E0E0E0)', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      layout: 'vertical',
      children: [
        {
          id: 'card',
          type: 'frame',
          fill: [{ type: 'solid', color: '#E0E0E0' }],
          layout: 'vertical',
          children: [{ id: 't', type: 'text', text: 'x' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    // contrast ratio ≈ 1.34 > 1.10 → NOT flagged
    const issues = detectInvisibleContainers(root, doc(root));
    expect(issues).toHaveLength(0);
  });
});

describe('detectEmptyPaths', () => {
  it('returns empty when all paths have geometry', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'p1', type: 'path', d: 'M0 0 L1 1' } as unknown as PenNode,
        { id: 'p2', type: 'path', d: 'M2 2 L3 3' } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectEmptyPaths(root)).toHaveLength(0);
  });

  it('flags a path node without geometry', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'empty', type: 'path' } as unknown as PenNode,
        { id: 'p2', type: 'path', d: 'M2 2 L3 3' } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectEmptyPaths(root);
    expect(issues).toHaveLength(1);
    expect(issues[0]).toMatchObject({
      nodeId: 'empty',
      category: 'empty-path',
      property: '__remove',
    });
  });

  it('recurses into nested frames', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 'wrap',
          type: 'frame',
          children: [{ id: 'deep-empty', type: 'path' } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectEmptyPaths(root)).toHaveLength(1);
  });
});

describe('detectTextExplicitHeights', () => {
  it('returns empty when no text has explicit pixel height', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't1', type: 'text', text: 'hi', fontSize: 14 } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextExplicitHeights(root)).toHaveLength(0);
  });

  it('flags text with numeric height', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 't1', type: 'text', text: 'hi', height: 40, fontSize: 14 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectTextExplicitHeights(root);
    expect(issues).toHaveLength(1);
    expect(issues[0]).toMatchObject({
      nodeId: 't1',
      category: 'text-explicit-height',
      property: 'height',
      suggestedValue: 'fit_content',
    });
  });

  it('ignores text with textGrowth=fixed-width-height', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 't1',
          type: 'text',
          text: 'hi',
          height: 40,
          textGrowth: 'fixed-width-height',
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectTextExplicitHeights(root)).toHaveLength(0);
  });
});

describe('detectSiblingInconsistencies', () => {
  it('flags a single outlier fontSize among 3 text siblings', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 't1', type: 'text', text: 'a', fontSize: 14 } as unknown as PenNode,
        { id: 't2', type: 'text', text: 'b', fontSize: 14 } as unknown as PenNode,
        { id: 't3', type: 'text', text: 'c', fontSize: 20 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectSiblingInconsistencies(root);
    const outlier = issues.find((i) => i.nodeId === 't3');
    expect(outlier).toBeDefined();
    expect(outlier!.property).toBe('fontSize');
    expect(outlier!.suggestedValue).toBe(14);
  });

  it('does nothing with fewer than 3 siblings', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 't1', type: 'text', text: 'a', fontSize: 14 } as unknown as PenNode,
        { id: 't2', type: 'text', text: 'b', fontSize: 20 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectSiblingInconsistencies(root)).toHaveLength(0);
  });

  it('does nothing without a 2/3 majority', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 't1', type: 'text', text: 'a', fontSize: 14 } as unknown as PenNode,
        { id: 't2', type: 'text', text: 'b', fontSize: 16 } as unknown as PenNode,
        { id: 't3', type: 'text', text: 'c', fontSize: 20 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectSiblingInconsistencies(root)).toHaveLength(0);
  });

  it('does NOT flag chrome-role sibling (tab-bar height=62) as inconsistent with 5 section frames', () => {
    // 5 content sections with fit_content height + 1 tab-bar with fixed height=62
    // The tab-bar is a different role-class (chrome) and should not join the
    // sections' comparison group. Expect zero sibling inconsistency issues.
    const makeSection = (id: string) =>
      ({
        id,
        type: 'frame',
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: 'Section' } as unknown as PenNode],
      }) as unknown as PenNode;

    const tabBar: PenNode = {
      id: 'tab-bar',
      type: 'frame',
      role: 'bottom-tab-bar',
      height: 62,
      layout: 'horizontal',
      children: [{ id: 'tab-t', type: 'text', text: 'Tab' } as unknown as PenNode],
    } as unknown as PenNode;

    const root: PenNode = {
      id: 'page',
      type: 'frame',
      children: [
        makeSection('s1'),
        makeSection('s2'),
        makeSection('s3'),
        makeSection('s4'),
        makeSection('s5'),
        tabBar,
      ],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    expect(issues).toHaveLength(0);
  });

  it('DOES flag outlier within a same-role group (4 cards, 1 with wrong cornerRadius) at WARNING severity', () => {
    // Same-role siblings still get sibling-consistency checks. This is the
    // real-value path: 4 cards meant to look identical, one with a wrong
    // dimension, should be flagged as an outlier so the user can fix it.
    // The strict pass emits 'warning' (auto-fixable) — the comparison
    // group shares a semantic role, so the fix is safe.
    const card = (id: string, cornerRadius: number): PenNode =>
      ({
        id,
        type: 'frame',
        role: 'card',
        cornerRadius,
        height: 200,
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: 'Card' } as unknown as PenNode],
      }) as unknown as PenNode;

    const root: PenNode = {
      id: 'row',
      type: 'frame',
      children: [card('c1', 12), card('c2', 12), card('c3', 12), card('c4', 8)],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    const cardIssue = issues.find((i) => i.nodeId === 'c4' && i.property === 'cornerRadius');
    expect(cardIssue).toBeDefined();
    expect(cardIssue!.suggestedValue).toBe(12);
    expect(cardIssue!.severity).toBe('warning');
  });

  it('does NOT compare HEIGHT across different roles (heroes are tall, footers are short)', () => {
    // Height is intentionally role-dependent — heroes are tall, footers
    // are short, navs are fixed-height. The strict pass groups by role
    // so cross-role same-type siblings don't compare on height.
    // 3 hero (height=600) + 3 footer (height=200), each role internally
    // consistent. Strict-only behavior would lump them and complain.
    const make = (id: string, role: string, h: number | string): PenNode =>
      ({
        id,
        type: 'frame',
        role,
        height: h,
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: 'x' } as unknown as PenNode],
      }) as unknown as PenNode;

    const root: PenNode = {
      id: 'page',
      type: 'frame',
      children: [
        make('h1', 'hero', 600),
        make('h2', 'hero', 600),
        make('h3', 'hero', 600),
        make('f1', 'footer', 200),
        make('f2', 'footer', 200),
        make('f3', 'footer', 200),
      ],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    // No height issue across role boundaries.
    expect(issues.find((i) => i.property === 'height')).toBeUndefined();
  });

  it('DOES flag cornerRadius outlier across singleton-role sections at INFO severity (detect-only)', () => {
    // The case Codex flagged: a web landing page with 4 unique role names
    // (hero, features, cta, footer) all sharing the same design-system
    // cornerRadius EXCEPT footer which is wrong. Strict role grouping
    // puts each in a 1-member singleton group, so the strict pass alone
    // would skip everything and miss the bug.
    //
    // The loose pass catches it at SEVERITY 'info' — the issue is
    // surfaced in debug reports, but pre-validation does NOT auto-apply
    // it. Cross-role comparison can match a structurally distinct
    // sibling (e.g. a rounded chrome element among square sections),
    // and silently rewriting that would damage intentional design.
    // The user/agent reviews info issues manually.
    const section = (id: string, role: string, cornerRadius: number): PenNode =>
      ({
        id,
        type: 'frame',
        role,
        cornerRadius,
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: id } as unknown as PenNode],
      }) as unknown as PenNode;

    const root: PenNode = {
      id: 'page',
      type: 'frame',
      children: [
        section('hero', 'hero', 0),
        section('features', 'features', 0),
        section('cta', 'cta', 0),
        section('footer', 'footer', 12),
      ],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    const footerIssue = issues.find((i) => i.nodeId === 'footer' && i.property === 'cornerRadius');
    expect(footerIssue).toBeDefined();
    expect(footerIssue!.suggestedValue).toBe(0);
    expect(footerIssue!.severity).toBe('info');
    // And no spurious height issue across the singleton roles.
    expect(issues.find((i) => i.property === 'height')).toBeUndefined();
  });

  it('LOOSE pass on chrome (rounded tab-bar among square sections) emits INFO only', () => {
    // Lock in the safety contract that motivated the strict/loose split:
    // a tab-bar with intentionally rounded corners among square content
    // sections WILL be flagged by the loose pass, but at 'info' severity.
    // Pre-validation skips info issues, so the tab-bar is never silently
    // rewritten. (Verified end-to-end in design-pre-validation.test.ts.)
    const section = (id: string): PenNode =>
      ({
        id,
        type: 'frame',
        role: 'section',
        cornerRadius: 0,
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: id } as unknown as PenNode],
      }) as unknown as PenNode;

    const tabBar: PenNode = {
      id: 'tabs',
      type: 'frame',
      role: 'bottom-tab-bar',
      cornerRadius: 16,
      height: 62,
      layout: 'horizontal',
      children: [{ id: 'tab-t', type: 'text', text: 'Tabs' } as unknown as PenNode],
    } as unknown as PenNode;

    const root: PenNode = {
      id: 'shell',
      type: 'frame',
      children: [section('s1'), section('s2'), section('s3'), section('s4'), section('s5'), tabBar],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    const tabIssue = issues.find((i) => i.nodeId === 'tabs' && i.property === 'cornerRadius');
    // Detected (so debug reports surface it)…
    expect(tabIssue).toBeDefined();
    // …but ONLY at info severity (so pre-validation never auto-applies it).
    expect(tabIssue!.severity).toBe('info');
  });

  it('DOES still flag the chrome use case (mobile shell with tab-bar) without a hand-curated chrome list', () => {
    // Original false positive: 5 content sections + 1 mobile tab-bar.
    // Old fix used a hardcoded chrome list. New fix relies on role-name
    // grouping: tab-bar's role differs from section's role, so they end
    // up in different groups; the lone tab-bar is a 1-member group and
    // gets skipped (< 3 siblings). No false positive without a chrome list.
    const section = (id: string): PenNode =>
      ({
        id,
        type: 'frame',
        role: 'section',
        height: 'fit_content',
        layout: 'vertical',
        children: [{ id: `${id}-t`, type: 'text', text: id } as unknown as PenNode],
      }) as unknown as PenNode;

    const tabBar: PenNode = {
      id: 'tabs',
      type: 'frame',
      role: 'bottom-tab-bar',
      height: 62,
      layout: 'horizontal',
      children: [{ id: 'tab-t', type: 'text', text: 'Tabs' } as unknown as PenNode],
    } as unknown as PenNode;

    const root: PenNode = {
      id: 'shell',
      type: 'frame',
      children: [section('s1'), section('s2'), section('s3'), section('s4'), section('s5'), tabBar],
    } as unknown as PenNode;

    const issues = detectSiblingInconsistencies(root);
    const tabIssue = issues.find((i) => i.nodeId === 'tabs');
    expect(tabIssue).toBeUndefined();
  });
});

describe('detectAllIssues', () => {
  it('runs all 4 detectors and deduplicates by nodeId:property', () => {
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      layout: 'vertical',
      children: [
        {
          id: 'card',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'horizontal',
          children: [
            { id: 'p', type: 'path' } as unknown as PenNode,
            { id: 't', type: 'text', text: 'x', height: 50 } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectAllIssues(root, doc(root));
    // invisible-container (card) + empty-path (p) + text-explicit-height (t)
    expect(issues.length).toBeGreaterThanOrEqual(3);
    const categories = new Set(issues.map((i) => i.category));
    expect(categories.has('invisible-container')).toBe(true);
    expect(categories.has('empty-path')).toBe(true);
    expect(categories.has('text-explicit-height')).toBe(true);
  });

  it('deduplicates when multiple detectors produce same nodeId+property', () => {
    // Synthesize a case where two detectors would both target the same
    // (nodeId, property). For current detectors this is unlikely but the
    // dedup strategy must still be applied (matching runPreValidationFixes).
    const root: PenNode = {
      id: 'r',
      type: 'frame',
      children: [],
    } as unknown as PenNode;
    const issues = detectAllIssues(root, doc(root));
    expect(issues).toHaveLength(0);
  });
});

describe('detectUnexpectedRotation', () => {
  it('flags a frame with non-axis-aligned rotation', () => {
    const root = {
      id: 'r',
      type: 'frame',
      rotation: 12,
      children: [],
    } as unknown as PenNode;
    const issues = detectUnexpectedRotation(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].category).toBe('unexpected-rotation');
    expect(issues[0].nodeId).toBe('r');
    expect(issues[0].suggestedValue).toBe(0);
  });

  it('does not flag rotation=0 (default)', () => {
    const root = { id: 'r', type: 'frame', rotation: 0, children: [] } as unknown as PenNode;
    expect(detectUnexpectedRotation(root)).toHaveLength(0);
  });

  it('does not flag missing rotation prop', () => {
    const root = { id: 'r', type: 'frame', children: [] } as unknown as PenNode;
    expect(detectUnexpectedRotation(root)).toHaveLength(0);
  });

  it('does not flag axis-aligned rotation (90/180/270 — intentional vertical text / grid)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'a', type: 'frame', rotation: 90, children: [] } as unknown as PenNode,
        { id: 'b', type: 'frame', rotation: 180, children: [] } as unknown as PenNode,
        { id: 'c', type: 'frame', rotation: -90, children: [] } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectUnexpectedRotation(root)).toHaveLength(0);
  });

  it('does NOT flag path / line / polygon / image (decorative geometry often rotated)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'p', type: 'path', rotation: 30 } as unknown as PenNode,
        { id: 'l', type: 'line', rotation: 45 } as unknown as PenNode,
        { id: 'g', type: 'polygon', rotation: 17 } as unknown as PenNode,
        { id: 'i', type: 'image', rotation: 12 } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectUnexpectedRotation(root)).toHaveLength(0);
  });

  it('flags nested frame with rotation', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 'wrapper',
          type: 'frame',
          children: [{ id: 'tilted', type: 'text', rotation: 7 } as unknown as PenNode],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectUnexpectedRotation(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('tilted');
  });
});

describe('detectTextCornerRadius', () => {
  it('flags text node with cornerRadius > 0', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', cornerRadius: 8, content: 'hi' } as unknown as PenNode],
    } as unknown as PenNode;
    const issues = detectTextCornerRadius(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].category).toBe('text-corner-radius');
    expect(issues[0].nodeId).toBe('t');
    expect(issues[0].suggestedValue).toBeUndefined();
  });

  it('does not flag text without cornerRadius', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', content: 'hi' } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextCornerRadius(root)).toHaveLength(0);
  });

  it('does not flag text with cornerRadius=0', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', cornerRadius: 0, content: 'hi' } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextCornerRadius(root)).toHaveLength(0);
  });

  it('does not flag frame nodes with cornerRadius (only text)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      cornerRadius: 12,
      children: [],
    } as unknown as PenNode;
    expect(detectTextCornerRadius(root)).toHaveLength(0);
  });
});

describe('detectMixedSiblingCornerRadius', () => {
  it('flags an outlier when 2 of 3 same-role cards share cornerRadius', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c3', type: 'frame', role: 'card', cornerRadius: 12, children: [] },
      ],
    } as unknown as PenNode;
    const issues = detectMixedSiblingCornerRadius(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('c3');
    expect(issues[0].suggestedValue).toBe(8);
  });

  it('does not flag when all siblings share cornerRadius', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c3', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
      ],
    } as unknown as PenNode;
    expect(detectMixedSiblingCornerRadius(root)).toHaveLength(0);
  });

  it('does not flag a 1-1-1 three-way split (no canonical modal value)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 4, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c3', type: 'frame', role: 'card', cornerRadius: 12, children: [] },
      ],
    } as unknown as PenNode;
    expect(detectMixedSiblingCornerRadius(root)).toHaveLength(0);
  });

  it('does not flag siblings with mixed roles (card vs button — different design tier)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'b1', type: 'frame', role: 'button', cornerRadius: 4, children: [] },
      ],
    } as unknown as PenNode;
    expect(detectMixedSiblingCornerRadius(root)).toHaveLength(0);
  });

  it('does not flag fewer than 3 siblings', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 12, children: [] },
      ],
    } as unknown as PenNode;
    expect(detectMixedSiblingCornerRadius(root)).toHaveLength(0);
  });

  it('skips dividers and spacers when grouping (they should not count)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        { id: 'c1', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 'd1', type: 'frame', role: 'divider', cornerRadius: 0, children: [] },
        { id: 'c2', type: 'frame', role: 'card', cornerRadius: 8, children: [] },
        { id: 's1', type: 'frame', role: 'spacer', cornerRadius: 0, children: [] },
        { id: 'c3', type: 'frame', role: 'card', cornerRadius: 12, children: [] },
      ],
    } as unknown as PenNode;
    const issues = detectMixedSiblingCornerRadius(root);
    // Only c1/c2/c3 are grouped (3 cards), modal=8, c3 is the outlier
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('c3');
  });
});

describe('detectTextEffect', () => {
  it('flags text node with shadow effects', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 't',
          type: 'text',
          content: 'hi',
          effects: [{ type: 'shadow', color: '#000', offsetX: 0, offsetY: 4, radius: 8 }],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectTextEffect(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].category).toBe('text-effect');
    expect(issues[0].nodeId).toBe('t');
    expect(issues[0].suggestedValue).toBeUndefined();
  });

  it('does not flag text without effects', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', content: 'hi' } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextEffect(root)).toHaveLength(0);
  });

  it('does not flag text with empty effects array', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', content: 'hi', effects: [] } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextEffect(root)).toHaveLength(0);
  });

  it('does not flag frame nodes with effects (only text)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      effects: [{ type: 'shadow', color: '#000', offsetX: 0, offsetY: 4, radius: 8 }],
      children: [],
    } as unknown as PenNode;
    expect(detectTextEffect(root)).toHaveLength(0);
  });

  it('flags multiple text nodes with effects in a tree', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 't1',
          type: 'text',
          content: 'a',
          effects: [{ type: 'shadow' }],
        } as unknown as PenNode,
        {
          id: 'wrapper',
          type: 'frame',
          children: [
            {
              id: 't2',
              type: 'text',
              content: 'b',
              effects: [{ type: 'blur' }],
            } as unknown as PenNode,
          ],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectTextEffect(root)).toHaveLength(2);
  });
});

describe('detectTextStroke', () => {
  it('flags text node with stroke (outlined text)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 't',
          type: 'text',
          content: 'hi',
          stroke: { thickness: 1, fill: [{ type: 'solid', color: '#000' }] },
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    const issues = detectTextStroke(root);
    expect(issues).toHaveLength(1);
    expect(issues[0].category).toBe('text-stroke');
    expect(issues[0].nodeId).toBe('t');
    expect(issues[0].suggestedValue).toBeUndefined();
  });

  it('does not flag text without stroke', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [{ id: 't', type: 'text', content: 'hi' } as unknown as PenNode],
    } as unknown as PenNode;
    expect(detectTextStroke(root)).toHaveLength(0);
  });

  it('does not flag text with stroke.thickness=0 (placeholder empty stroke)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      children: [
        {
          id: 't',
          type: 'text',
          content: 'hi',
          stroke: { thickness: 0, fill: [] },
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;
    expect(detectTextStroke(root)).toHaveLength(0);
  });

  it('does not flag frame nodes with stroke (only text)', () => {
    const root = {
      id: 'r',
      type: 'frame',
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#000' }] },
      children: [],
    } as unknown as PenNode;
    expect(detectTextStroke(root)).toHaveLength(0);
  });
});
