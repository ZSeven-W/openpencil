import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import type { Issue } from './types';

/** Extract the first fill color from a node (raw, including variable refs) */
function getFirstFillColor(node: PenNode): string | null {
  if (!('fill' in node) || !Array.isArray(node.fill) || node.fill.length === 0) return null;
  const first = node.fill[0];
  if (first && 'color' in first && first.color) return first.color;
  return null;
}

/**
 * Compare two color strings via WCAG relative-luminance contrast ratio.
 * Returns 1.0 for identical colors, growing toward 21.0 as they diverge,
 * or Infinity if either color cannot be parsed (e.g. variable refs).
 *
 * Why WCAG contrast ratio rather than max RGB channel diff:
 * the human eye is much more sensitive to small tonal differences on
 * dark backgrounds than light ones (Weber–Fechner / dark adaptation).
 * A 9-unit RGB diff means very different things at different lightness:
 *
 *   #FAFAFA vs #F1F1F1  (light, RGB diff 9): contrast ratio ≈ 1.07 → invisible
 *   #111111 vs #1A1A1A  (dark,  RGB diff 9): contrast ratio ≈ 1.18 → distinguishable
 *
 * Channel-diff treats them identically and produces false positives on
 * dark theme cards. Contrast ratio is luminance-based, perceptually
 * uniform-ish, and gives the right answer in both regimes. It also
 * matches the metric WCAG and design tools (Stark, Figma) use.
 *
 * Used by detectInvisibleContainers to catch cases where fill colors are
 * visually nearly identical but not strictly equal.
 */
function colorContrast(a: string, b: string): number {
  if (a === b) return 1;
  const pa = parseHexColor(a);
  const pb = parseHexColor(b);
  if (!pa || !pb) return Infinity;
  const lumA = relativeLuminance(pa);
  const lumB = relativeLuminance(pb);
  const lighter = Math.max(lumA, lumB);
  const darker = Math.min(lumA, lumB);
  return (lighter + 0.05) / (darker + 0.05);
}

/** WCAG 2.x relative luminance for sRGB. Returns 0.0–1.0. */
function relativeLuminance(c: { r: number; g: number; b: number }): number {
  const lin = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

/** Parse #rgb / #rrggbb / #rrggbbaa to {r,g,b}. Returns null on parse failure. */
function parseHexColor(s: string): { r: number; g: number; b: number } | null {
  if (typeof s !== 'string') return null;
  const m = s.trim().match(/^#([0-9a-fA-F]{3,8})$/);
  if (!m) return null;
  let hex = m[1];
  if (hex.length === 3) {
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  }
  if (hex.length !== 6 && hex.length !== 8) return null;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return null;
  return { r, g, b };
}

/** Check if a node already has a visible stroke */
function hasStroke(node: PenNode): boolean {
  if (!('stroke' in node)) return false;
  const s = node.stroke as { thickness?: number } | undefined;
  return s != null && (s.thickness ?? 0) > 0;
}

interface PenDocumentLike {
  variables?: Record<string, unknown>;
}

/** Resolve border color: prefer $color-border variable if it exists, else neutral gray */
function getBorderStroke(doc: PenDocumentLike): {
  thickness: number;
  fill: Array<{ type: 'solid'; color: string }>;
} {
  const hasBorderVar = doc.variables && 'color-border' in doc.variables;
  const color = hasBorderVar ? '$color-border' : '#E2E8F0';
  return { thickness: 1, fill: [{ type: 'solid', color }] };
}

export interface DetectInvisibleContainersOptions {
  /**
   * 'perceptual' uses WCAG relative-luminance contrast ratio with
   * `contrastRatioThreshold`. 'strict' uses === equality (the original
   * behavior). Default: 'perceptual'.
   */
  colorMatchMode?: 'strict' | 'perceptual';
  /**
   * Maximum WCAG contrast ratio (1.0–21.0) below which two colors are
   * considered "the same" in perceptual mode. Default: 1.10.
   *
   * Reference points:
   *   1.00 = identical
   *   1.10 = essentially indistinguishable on either light or dark bg
   *   1.50 = subtly visible
   *   3.00 = clearly visible (WCAG AA UI element)
   *   4.50 = WCAG AA text contrast
   *
   * Higher = more aggressive detection (will flag more pairs as
   * "invisible"). Lower = stricter (fewer false positives, more misses).
   */
  contrastRatioThreshold?: number;
}

/**
 * Detect frames whose fill matches their parent's fill (rendered "invisible")
 * and which contain visible content. Suggests adding a subtle stroke so the
 * container becomes distinguishable.
 */
export function detectInvisibleContainers(
  root: PenNode,
  doc: PenDocument,
  opts: DetectInvisibleContainersOptions = {},
): Issue[] {
  const mode = opts.colorMatchMode ?? 'perceptual';
  const threshold = opts.contrastRatioThreshold ?? 1.1;
  const issues: Issue[] = [];
  walk(root, null);
  return issues;

  function walk(node: PenNode, parentFillColor: string | null): void {
    const nodeFill = getFirstFillColor(node);

    if (
      parentFillColor &&
      nodeFill &&
      !hasStroke(node) &&
      node.type === 'frame' &&
      'layout' in node &&
      (node as { layout?: unknown }).layout &&
      'children' in node &&
      node.children &&
      node.children.length > 0
    ) {
      const ratio = colorContrast(nodeFill, parentFillColor);
      const same = mode === 'strict' ? nodeFill === parentFillColor : ratio <= threshold;
      if (same) {
        // Theme-aware severity: on dark backgrounds the suggested
        // light-gray border (#E2E8F0) would actively damage the design.
        // Downgrade dark-on-dark cases to 'info' (detect-only); pre-
        // validation skips info severity so the user/agent can decide
        // manually whether to add a dark-appropriate border. Light-on-
        // light cases stay 'warning' and remain auto-fixable.
        const parsedParent = parseHexColor(parentFillColor);
        const isDarkOnDark = parsedParent != null && relativeLuminance(parsedParent) < 0.1;
        issues.push({
          nodeId: node.id,
          category: 'invisible-container',
          severity: isDarkOnDark ? 'info' : 'warning',
          property: 'stroke',
          currentValue: (node as { stroke?: unknown }).stroke ?? null,
          suggestedValue: getBorderStroke(doc),
          reason: `same fill as parent (${nodeFill} ≈ ${parentFillColor}, contrast=${ratio.toFixed(2)})`,
        });
      }
    }

    if ('children' in node && node.children) {
      for (const child of node.children) {
        walk(child, nodeFill ?? parentFillColor);
      }
    }
  }
}

/**
 * Detect path nodes without geometry data (empty `d` property).
 * These render as invisible empty rectangles on canvas.
 */
export function detectEmptyPaths(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    if (node.type === 'path') {
      const hasD = 'd' in node && (node as unknown as Record<string, unknown>).d;
      if (!hasD) {
        issues.push({
          nodeId: node.id,
          category: 'empty-path',
          severity: 'warning',
          property: '__remove',
          currentValue: null,
          suggestedValue: true,
          reason: 'path node without geometry (renders invisible)',
        });
      }
    }
    if ('children' in node && node.children) {
      for (const child of node.children) walk(child);
    }
  }
}

/**
 * Detect text nodes with explicit pixel heights. Explicit heights on text
 * always cause clipping or overlap; the layout engine should auto-calculate
 * height from content + fontSize + lineHeight instead.
 */
export function detectTextExplicitHeights(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    if (node.type === 'text') {
      const textNode = node as PenNode & { height?: unknown; textGrowth?: string };
      if (typeof textNode.height === 'number' && textNode.textGrowth !== 'fixed-width-height') {
        issues.push({
          nodeId: node.id,
          category: 'text-explicit-height',
          severity: 'warning',
          property: 'height',
          currentValue: textNode.height,
          suggestedValue: 'fit_content',
          reason: `text node has explicit height=${textNode.height}px — causes clipping`,
        });
      }
    }
    if ('children' in node && node.children) {
      for (const child of node.children) walk(child);
    }
  }
}

/**
 * Property classification for sibling consistency checks.
 *
 * STRICT props are intentionally role-dependent — their value SHOULD vary
 * by what the sibling represents (hero is tall, footer is short, tab-bar
 * is fixed-height). They are only compared among same-type, same-role
 * siblings.
 *
 * LOOSE props are design-system tokens that should be uniform across
 * structurally-similar siblings regardless of semantic role (typically
 * cornerRadius). They additionally get a cross-role check among same-type
 * siblings, catching outliers in singleton-role compositions (e.g. a web
 * landing page where every section has a unique role and the strict pass
 * would otherwise skip every group as <3 members).
 *
 * The strict/loose split is what lets this detector handle BOTH:
 *   - Mobile chrome shells without false positives (chrome's fixed height
 *     is in STRICT → never compared against sections cross-role)
 *   - Web landing pages without false negatives (cornerRadius is in LOOSE
 *     → compared across hero/features/cta/footer even though all roles
 *     are singletons)
 */
const FRAME_STRICT_PROPS = ['height'] as const;
const FRAME_LOOSE_PROPS = ['cornerRadius'] as const;
const TEXT_STRICT_PROPS = ['fontSize'] as const;

/**
 * Detect property inconsistencies among siblings (>= 3 siblings,
 * >= 2/3 majority rule). Outliers get an Issue to align with the majority.
 *
 * Two parallel passes:
 *
 *   1. STRICT pass: groups siblings by `${type}:${role || '__none__'}`
 *      and checks STRICT + LOOSE properties. Same-role siblings get the
 *      tightest consistency check. Divider/spacer roles are skipped
 *      entirely (intentionally tiny layout primitives whose dimensions
 *      shouldn't match anything).
 *
 *   2. LOOSE pass: groups siblings by `${type}` only and checks LOOSE
 *      properties. This catches outliers in compositions where every
 *      sibling has a unique role (web landing pages: hero/features/
 *      cta/footer) which would otherwise all be skipped as singleton
 *      strict groups.
 *
 * Issues that overlap between the two passes are deduplicated by nodeId
 * + property; the strict-pass version wins (it iterates first).
 *
 * Why a strict/loose split rather than a hand-curated chrome list:
 *   - Same role names mean different things in different design types:
 *     `navbar` is chrome on mobile but a content section on web; `footer`
 *     is a content section on web but rare on mobile. Any hand-curated
 *     chrome list will be wrong in one of those contexts.
 *   - Splitting by property semantics ("which props are role-dependent?")
 *     is invariant across design contexts and doesn't require curating
 *     a list of structurally-distinct roles.
 */
export function detectSiblingInconsistencies(root: PenNode): Issue[] {
  const raw: Issue[] = [];
  walk(root);

  // Dedupe: strict and loose passes can both emit on the same
  // (nodeId, property) pair (e.g. a same-role cornerRadius outlier
  // is caught by both). Keep first occurrence — strict iterates first.
  const seen = new Set<string>();
  const unique: Issue[] = [];
  for (const issue of raw) {
    const key = `${issue.nodeId}:${issue.property}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(issue);
  }
  return unique;

  function walk(node: PenNode): void {
    if (!('children' in node) || !node.children) return;
    if (node.children.length >= 3) {
      const strictGroups = new Map<string, PenNode[]>();
      const typeGroups = new Map<string, PenNode[]>();
      for (const child of node.children) {
        const childRole = ((child as { role?: string }).role ?? '').toLowerCase() || '__none__';
        // Skip divider/spacer — visual layout primitives whose dimensions
        // are intentionally tiny and don't share structure with siblings;
        // reporting them as outliers is always noise.
        if (childRole === 'divider' || childRole === 'spacer') continue;

        const strictKey = `${child.type}:${childRole}`;
        if (!strictGroups.has(strictKey)) strictGroups.set(strictKey, []);
        strictGroups.get(strictKey)!.push(child);

        if (!typeGroups.has(child.type)) typeGroups.set(child.type, []);
        typeGroups.get(child.type)!.push(child);
      }

      // Strict pass: same-type, same-role siblings get checked on every
      // applicable property (STRICT + LOOSE). Severity 'warning' — these
      // are confidently auto-fixable because the comparison group shares
      // a semantic role.
      for (const [strictKey, siblings] of strictGroups) {
        if (siblings.length < 3) continue;
        const type = strictKey.split(':', 1)[0];
        const props =
          type === 'text'
            ? (TEXT_STRICT_PROPS as readonly string[])
            : ([...FRAME_STRICT_PROPS, ...FRAME_LOOSE_PROPS] as readonly string[]);
        for (const prop of props) {
          checkConsistency(siblings, prop, raw, 'warning');
        }
      }

      // Loose pass: same-type-only siblings get checked on role-independent
      // properties only. Catches singleton-role outliers (web landing page
      // sections) without re-introducing the chrome height false positive
      // (height is in STRICT, never checked here).
      //
      // Severity 'info' — these are DETECT-ONLY. The pre-validation pipeline
      // skips info-severity issues for auto-fix, because cross-role
      // comparison can match a structurally distinct sibling (e.g. a
      // rounded tab-bar among square sections) and silently rewriting it
      // would damage intentional design choices. The issue still appears
      // in debug reports so the user/agent can review it manually.
      for (const [type, siblings] of typeGroups) {
        if (siblings.length < 3) continue;
        if (type === 'text') continue; // text has no role-independent props
        for (const prop of FRAME_LOOSE_PROPS) {
          checkConsistency(siblings, prop, raw, 'info');
        }
      }
    }
    for (const child of node.children) walk(child);
  }
}

/**
 * Run all 4 detectors and return the deduplicated combined issue list.
 * Dedup key: `${nodeId}:${property}` (matches runPreValidationFixes).
 * On collision, the first issue wins (detector execution order below).
 */
export function detectAllIssues(root: PenNode, doc: PenDocument): Issue[] {
  const combined: Issue[] = [
    ...detectInvisibleContainers(root, doc),
    ...detectEmptyPaths(root),
    ...detectTextExplicitHeights(root),
    ...detectSiblingInconsistencies(root),
  ];
  const seen = new Set<string>();
  const unique: Issue[] = [];
  for (const issue of combined) {
    const key = `${issue.nodeId}:${issue.property}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(issue);
  }
  return unique;
}

function checkConsistency(
  siblings: PenNode[],
  property: string,
  issues: Issue[],
  severity: 'warning' | 'info',
): void {
  const values = new Map<string, { value: unknown; nodes: PenNode[] }>();
  for (const node of siblings) {
    const raw = (node as unknown as Record<string, unknown>)[property];
    if (raw == null) continue;
    const key = JSON.stringify(raw);
    if (!values.has(key)) values.set(key, { value: raw, nodes: [] });
    values.get(key)!.nodes.push(node);
  }
  if (values.size < 2) return;

  let majority: { value: unknown; nodes: PenNode[] } | null = null;
  for (const entry of values.values()) {
    if (!majority || entry.nodes.length > majority.nodes.length) majority = entry;
  }
  if (!majority) return;

  const totalWithProp = Array.from(values.values()).reduce((s, e) => s + e.nodes.length, 0);
  if (majority.nodes.length < (totalWithProp * 2) / 3) return;

  for (const entry of values.values()) {
    if (entry === majority) continue;
    for (const node of entry.nodes) {
      issues.push({
        nodeId: node.id,
        category: 'sibling-inconsistency',
        severity,
        property,
        currentValue: (node as unknown as Record<string, unknown>)[property],
        suggestedValue: majority.value,
        reason: `inconsistent with ${majority.nodes.length} siblings`,
      });
    }
  }
}
