import type { PenNode } from '@zseven-w/pen-types';
import type { Issue } from './types';

/**
 * Roles that legitimately span the full mobile viewport width and SHOULD
 * have 0 horizontal padding from the page root. Sectioning detectors must
 * skip these — flagging them produces false positives that the autofix
 * would damage (forcing 16px gutters on a top-nav inset breaks chrome).
 */
const FULL_BLEED_ROLES = new Set([
  'hero',
  'banner',
  'cover',
  'header',
  'top-nav',
  'bottom-nav',
  'status-bar',
  'tab-bar',
  'tabbar',
  'navbar',
]);

function getPaddingLeft(node: PenNode): number {
  const p = (node as unknown as { padding?: unknown }).padding;
  if (typeof p === 'number') return p;
  if (Array.isArray(p)) {
    if (p.length === 4) return Number(p[3] ?? 0);
    if (p.length === 2) return Number(p[1] ?? 0);
    if (p.length === 1) return Number(p[0] ?? 0);
  }
  return 0;
}

function getPaddingRight(node: PenNode): number {
  const p = (node as unknown as { padding?: unknown }).padding;
  if (typeof p === 'number') return p;
  if (Array.isArray(p)) {
    if (p.length === 4) return Number(p[1] ?? 0);
    if (p.length === 2) return Number(p[1] ?? 0);
    if (p.length === 1) return Number(p[0] ?? 0);
  }
  return 0;
}

function hasTextOrIconDescendant(node: PenNode): boolean {
  if (node.type === 'text') return true;
  if ((node.type as string) === 'icon_font') return true;
  if ('children' in node && Array.isArray(node.children)) {
    for (const c of node.children) {
      if (hasTextOrIconDescendant(c)) return true;
    }
  }
  return false;
}

/**
 * A child whose only descendants are images / image-placeholders is treated
 * as a deliberately full-bleed media tile — flagging its 0-padding would
 * force a gutter on banner-shaped cells that the user explicitly designed
 * edge-to-edge.
 */
function isImageOnlySection(node: PenNode): boolean {
  if (!('children' in node) || !Array.isArray(node.children)) return false;
  if (node.children.length === 0) return false;
  for (const c of node.children) {
    if (c.type === 'image') continue;
    const role = ((c as { role?: string }).role ?? '').toLowerCase();
    if (role === 'image-placeholder') continue;
    return false;
  }
  return true;
}

/**
 * Aesthetic detector: mobile-page content section glued to the screen edge.
 *
 * Detects the "Categories no padding" bug from the 2026-05-09 user report —
 * a mobile page where the root has 0 left padding AND a content section
 * also has 0 left padding AND that section contains visible text or icon
 * descendants. The combined chain means content visually butts against the
 * viewport edge, which is unintentional in nearly every mobile UI.
 *
 * Why root-level fix instead of per-section:
 *   The user's mental model is "the page has gutters" — a single root
 *   padding update affects every section at once and matches how mobile
 *   pages are typically expressed in production codebases (`safe-area`
 *   inset on the page container, not on every child). Per-section padding
 *   would compound when a section also wants its own internal padding.
 *
 * False-positive guards:
 *   - Mobile-shaped root only (width 320–480 + layout + ≥2 children)
 *   - Skip child sections with chrome roles (top-nav / bottom-nav / status-bar)
 *   - Skip child sections with full-bleed roles (hero / banner / cover / header)
 *   - Skip image-only sections (intentional full-bleed media)
 *   - Only flag when the offending section contains text or icon_font
 *
 * Suggested fix: set root.padding to [top, 16, bottom, 16] preserving
 * vertical padding. 16 is the iOS HIG / Material default mobile gutter.
 */
export function detectEdgeSectionPadding(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    const width = (node as unknown as { width?: unknown }).width;
    const height = (node as unknown as { height?: unknown }).height;
    // Page-shaped only — width AND height must look like a real mobile
    // device. Width 320–480 alone matches a bottom-nav (393×88), a card
    // (393×200), a modal (393×500) and similar components which would
    // get a wrong page-level gutter forced on them. Height >= 568
    // (iPhone SE 1st gen, the shortest production phone) excludes these
    // while still covering iPhone 8 (667), iPhone 14 Pro (852), Pixel 8
    // Pro (1080) and every taller device. Aspect ratio h/w >= 1.5 is a
    // second guard against accidentally tall components like a side
    // drawer or settings panel.
    const looksLikeMobilePage =
      node.type === 'frame' &&
      typeof width === 'number' &&
      width >= 320 &&
      width <= 480 &&
      typeof height === 'number' &&
      height >= 568 &&
      height >= width * 1.5;
    const isMobileRoot =
      looksLikeMobilePage &&
      'layout' in node &&
      (node as { layout?: unknown }).layout != null &&
      'children' in node &&
      Array.isArray(node.children) &&
      node.children.length >= 2;

    if (isMobileRoot) {
      const rootPadL = getPaddingLeft(node);
      if (rootPadL === 0) {
        // Two-pass scan over content children (skipping full-bleed
        // chrome / image-only banners which are intentionally edge-to-
        // edge):
        //   - any with own h-padding > 0  → design has chosen the
        //                                   per-section gutter pattern,
        //                                   adding root padding would
        //                                   double-inset everything else
        //   - any with no padding but text/icon descendants → would
        //                                   genuinely glue content to
        //                                   the viewport edge, so flag
        //
        // 2026-05-11 user feedback: previous version flagged whenever
        // ≥1 child had no padding, even when other siblings already
        // carried per-section padding. Result: root got extra 16px on
        // top of the existing per-section gutters → "边距过大". The new
        // rule treats any per-section-padded sibling as a signal that
        // the design has opted out of root-level gutter and aborts the
        // detector entirely.
        let hasPerSectionPaddedSibling = false;
        const offendingChildren: PenNode[] = [];
        for (const child of (node as { children: PenNode[] }).children) {
          if (child.type !== 'frame') continue;
          const role = ((child as { role?: string }).role ?? '').toLowerCase();
          if (FULL_BLEED_ROLES.has(role)) continue;
          if (isImageOnlySection(child)) continue;
          if (getPaddingLeft(child) > 0) {
            hasPerSectionPaddedSibling = true;
            continue;
          }
          if (!hasTextOrIconDescendant(child)) continue;
          offendingChildren.push(child);
        }
        if (offendingChildren.length > 0 && !hasPerSectionPaddedSibling) {
          const currentPad = (node as unknown as { padding?: unknown }).padding;
          let suggested: number[];
          if (Array.isArray(currentPad) && currentPad.length === 4) {
            suggested = [Number(currentPad[0]) || 0, 16, Number(currentPad[2]) || 0, 16];
          } else if (Array.isArray(currentPad) && currentPad.length === 2) {
            suggested = [Number(currentPad[0]) || 0, 16, Number(currentPad[0]) || 0, 16];
          } else if (typeof currentPad === 'number') {
            suggested = [currentPad, 16, currentPad, 16];
          } else {
            suggested = [0, 16, 0, 16];
          }
          issues.push({
            nodeId: node.id,
            category: 'edge-section-padding',
            severity: 'warning',
            property: 'padding',
            currentValue: currentPad ?? null,
            suggestedValue: suggested,
            reason: `mobile root has 0 horizontal padding while ${offendingChildren.length} content section(s) glue text/icon to screen edge`,
          });
        }
      }
    }

    if ('children' in node && Array.isArray(node.children)) {
      for (const c of node.children) walk(c);
    }
  }
}

/**
 * Aesthetic detector: page root and a direct content section both apply
 * horizontal padding, producing a doubled inset.
 *
 * 2026-05-10 user-reported "Bistro" mobile design hit this: root carried
 * `padding: [0, 16, 0, 16]` and `Today's Specials` section carried
 * `padding: [0, 24]`. Effective gutter = 16 + 24 = 40px on a 375px page,
 * leaving only 295px for content. Read as "too much padding" / pinched.
 *
 * The skill prompt now teaches "page gutter goes on ONE layer, not both"
 * (see layout.md AESTHETIC HYGIENE). This detector is the cheap
 * post-pass guard for when the LLM ignores the rule.
 *
 * Severity is INFO (detect-only). Auto-fix is tempting (zero out section
 * horizontal padding, keep root) but a section may legitimately want a
 * larger inset for visual emphasis (a hero card with deeper gutter than
 * surrounding sections). Surface in the audit panel; let the user/agent
 * decide. The skill rule covers the preventive side.
 *
 * Page-shape filter mirrors detectEdgeSectionPadding to avoid firing on
 * components-with-internal-padding (chip → status-badge stacking is
 * legitimate; the doubled-gutter problem is unique to mobile pages).
 */
export function detectStackedHorizontalPadding(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    const width = (node as unknown as { width?: unknown }).width;
    const height = (node as unknown as { height?: unknown }).height;
    const looksLikeMobilePage =
      node.type === 'frame' &&
      typeof width === 'number' &&
      width >= 320 &&
      width <= 480 &&
      typeof height === 'number' &&
      height >= 568 &&
      height >= width * 1.5;

    if (
      looksLikeMobilePage &&
      'children' in node &&
      Array.isArray(node.children) &&
      node.children.length >= 2
    ) {
      const rootL = getPaddingLeft(node);
      const rootR = getPaddingRight(node);
      if (rootL > 0 || rootR > 0) {
        for (const child of (node as { children: PenNode[] }).children) {
          if (child.type !== 'frame') continue;
          const role = ((child as { role?: string }).role ?? '').toLowerCase();
          if (FULL_BLEED_ROLES.has(role)) continue;
          const childL = getPaddingLeft(child);
          const childR = getPaddingRight(child);
          if (childL === 0 && childR === 0) continue;
          // Section's horizontal padding stacks with root's. Flag the section
          // (the offender — root is the established gutter holder).
          issues.push({
            nodeId: child.id,
            category: 'stacked-horizontal-padding',
            severity: 'info',
            property: 'padding',
            currentValue: (child as unknown as { padding?: unknown }).padding,
            suggestedValue: null,
            reason: `section h-padding [${childL}/${childR}] stacks with root h-padding [${rootL}/${rootR}] — combined gutter ${rootL + childL}/${rootR + childR}`,
          });
        }
      }
    }

    if ('children' in node && Array.isArray(node.children)) {
      for (const c of node.children) walk(c);
    }
  }
}
