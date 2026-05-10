import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { resolveColorRef, getDefaultTheme } from '@zseven-w/pen-core';
import type { Issue } from './types';
import { colorContrast } from './color-utils';

/** WCAG 2.x AA threshold for normal-size text. */
const WCAG_AA_NORMAL = 4.5;
/** WCAG 2.x AA threshold for large text (>= 18pt or >= 14pt bold). */
const WCAG_AA_LARGE = 3.0;

/**
 * Pull the first solid color out of a fill array. Gradients get reduced to
 * their first stop as a coarse approximation — text on a gradient bg is
 * already a hard contrast call, and reading the gradient midpoint is
 * outside this detector's scope. Returns null when there's no usable
 * solid color (image fills, missing fills, transparent overlays).
 */
function firstSolidColor(fills: unknown): string | null {
  if (!Array.isArray(fills) || fills.length === 0) return null;
  for (const fill of fills) {
    if (!fill || typeof fill !== 'object') continue;
    const f = fill as { type?: string; color?: string; stops?: Array<{ color?: string }> };
    if (f.type === 'solid' && typeof f.color === 'string') return f.color;
    if (
      (f.type === 'linear_gradient' || f.type === 'radial_gradient') &&
      Array.isArray(f.stops) &&
      f.stops.length > 0 &&
      typeof f.stops[0].color === 'string'
    ) {
      return f.stops[0].color;
    }
  }
  return null;
}

/**
 * WCAG 2.x large-text rule: >= 18pt OR >= 14pt bold (weight >= 700).
 * Web maps 18pt ≈ 24px and 14pt ≈ 18.66px → use 24/19 in pixel terms.
 */
function isLargeText(node: PenNode): boolean {
  const t = node as PenNode & { fontSize?: number; fontWeight?: number | string };
  if (typeof t.fontSize !== 'number') return false;
  if (t.fontSize >= 24) return true;
  const weight =
    typeof t.fontWeight === 'number'
      ? t.fontWeight
      : typeof t.fontWeight === 'string'
        ? parseInt(t.fontWeight, 10)
        : NaN;
  if (t.fontSize >= 19 && Number.isFinite(weight) && weight >= 700) return true;
  return false;
}

/**
 * Aesthetic detector: text whose color contrast against its rendered
 * background falls below WCAG AA threshold (4.5:1 for normal text,
 * 3.0:1 for large text).
 *
 * Background determination walks the parent chain and picks the first
 * solid fill (or gradient first-stop) it finds. Missing fills are
 * "transparent" — the walk continues to the next ancestor; if the chain
 * exhausts with no color, the detector defaults to white (#FFFFFF), the
 * canvas bg in OpenPencil's renderer.
 *
 * Both text and background colors are resolved through the document's
 * variable table and active theme so `$color-text-primary` etc. compare
 * correctly. Unresolvable refs (typo'd token names, runtime-only refs)
 * skip the check rather than guess.
 *
 * Severity is INFO (detect-only, never auto-fix). The 2026-05-09 user
 * feedback was explicit that auto-replacing brand colors via "nearest
 * token" heuristics was unsafe — this detector surfaces issues into the
 * audit panel and chat status line so the user / agent can decide,
 * without silently rewriting their fills.
 */
export function detectTextBgContrast(root: PenNode, doc: PenDocument): Issue[] {
  const issues: Issue[] = [];
  const variables = doc.variables ?? {};
  const theme = getDefaultTheme(doc.themes);

  walk(root, []);
  return issues;

  function walk(node: PenNode, ancestors: PenNode[]): void {
    if (node.type === 'text') {
      checkText(node, ancestors);
    }
    if ('children' in node && Array.isArray(node.children)) {
      const nextAncestors = ancestors.concat(node);
      for (const c of node.children) walk(c, nextAncestors);
    }
  }

  function ancestorBgColor(ancestors: PenNode[]): string {
    // Walk closest-first; skip transparent/imageless ancestors. Default
    // to white when the chain runs out — matches OpenPencil's canvas
    // default and avoids the "everything fails" report when a doc has
    // no explicit page fill.
    for (let i = ancestors.length - 1; i >= 0; i--) {
      const fill = (ancestors[i] as unknown as { fill?: unknown }).fill;
      const raw = firstSolidColor(fill);
      if (!raw) continue;
      const resolved = resolveColorRef(raw, variables, theme);
      if (typeof resolved === 'string') return resolved;
    }
    return '#FFFFFF';
  }

  function checkText(node: PenNode, ancestors: PenNode[]): void {
    const textFill = (node as unknown as { fill?: unknown }).fill;
    const rawText = firstSolidColor(textFill);
    if (!rawText) return; // no fill or non-solid — renderer default applies, skip
    const textColor = resolveColorRef(rawText, variables, theme);
    if (typeof textColor !== 'string') return; // unresolvable ref

    const bgColor = ancestorBgColor(ancestors);
    const ratio = colorContrast(textColor, bgColor);
    if (!Number.isFinite(ratio)) return; // either color failed to parse

    const threshold = isLargeText(node) ? WCAG_AA_LARGE : WCAG_AA_NORMAL;
    if (ratio >= threshold) return;

    issues.push({
      nodeId: node.id,
      category: 'text-bg-contrast',
      // Detect-only — auto-replacing the fill with a "nearest brand
      // token" is the path the 2026-05-09 review explicitly rejected.
      severity: 'info',
      property: 'fill',
      currentValue: textFill ?? null,
      // No suggestedValue: the right replacement depends on the design
      // system + theme + intent, which only the user/agent can decide.
      suggestedValue: null,
      reason: `text/bg contrast ${ratio.toFixed(2)}:1 below WCAG AA ${threshold}:1 (text=${textColor} on bg=${bgColor})`,
    });
  }
}
