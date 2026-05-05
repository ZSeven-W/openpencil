import type { PenNode, FrameNode, SizingBehavior } from '@/types/pen';
import type { PathNode } from '@/types/pen';
import type { PenFill, PenStroke, PenEffect, SolidFill } from '@/types/styles';
import {
  toSizeNumber,
  toGapNumber,
  parsePaddingValues,
  estimateNodeIntrinsicHeight,
  getTextContentForNode,
  hasCjkText,
} from './generation-utils';
import { resolveIconPathBySemanticName } from './icon-resolver';

// ---------------------------------------------------------------------------
// Context passed to each role rule function
// ---------------------------------------------------------------------------

export interface RoleContext {
  /** Role of the parent node, if any */
  parentRole?: string;
  /** Layout of the parent node */
  parentLayout?: 'none' | 'vertical' | 'horizontal';
  /** Width of the parent node's content area (px) */
  parentContentWidth?: number;
  /** Root canvas width (1200 for desktop, 375 for mobile) */
  canvasWidth: number;
  /** Whether CJK text is detected in the design context */
  hasCjk?: boolean;
  /** Whether this node is inside a table-like structure */
  isTableContext?: boolean;
  /**
   * Document theme detected from the page root fill at the start of
   * `resolveTreeRoles`. Roles that have visual defaults (navbar, card,
   * input, divider) read this so the LLM doesn't get a #FFFFFF default
   * painted on top of an explicitly dark page background.
   */
  theme?: 'dark' | 'light';
}

// ---------------------------------------------------------------------------
// Role defaults — partial properties that fill unset values on a node
// ---------------------------------------------------------------------------

export type RoleDefaults = Partial<{
  layout: 'none' | 'vertical' | 'horizontal';
  gap: number;
  padding: number | [number, number] | [number, number, number, number];
  justifyContent: 'start' | 'center' | 'end' | 'space_between' | 'space_around';
  alignItems: 'start' | 'center' | 'end';
  width: SizingBehavior;
  height: SizingBehavior;
  clipContent: boolean;
  cornerRadius: number;
  textGrowth: 'auto' | 'fixed-width' | 'fixed-width-height';
  textAlign: 'left' | 'center' | 'right';
  textAlignVertical: 'top' | 'middle' | 'bottom';
  lineHeight: number;
  letterSpacing: number;
  fill: PenFill[];
  stroke: PenStroke;
  effects: PenEffect[];
}>;

/** A role rule function computes defaults based on context. */
export type RoleRuleFn = (node: PenNode, ctx: RoleContext) => RoleDefaults;

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

const roleRegistry = new Map<string, RoleRuleFn>();

/**
 * Register a role rule. Any string is a valid role name.
 * If the same role is registered twice, the later one wins.
 */
export function registerRole(role: string, ruleFn: RoleRuleFn): void {
  roleRegistry.set(role, ruleFn);
}

// ---------------------------------------------------------------------------
// Name-based role inference for models that don't output `role`
// ---------------------------------------------------------------------------

/** Exact name → role mappings (case-insensitive). */
const NAME_EXACT_MAP: Record<string, string> = {
  navbar: 'navbar',
  navigation: 'navbar',
  'navigation bar': 'navbar',
  'nav bar': 'navbar',
  nav: 'navbar',
  header: 'navbar',
  'top bar': 'navbar',
  topbar: 'navbar',
  hero: 'hero',
  'hero section': 'hero',
  footer: 'footer',
  'search bar': 'search-bar',
  searchbar: 'search-bar',
  'search input': 'search-bar',
  search: 'search-bar',
  avatar: 'avatar',
  divider: 'divider',
  separator: 'divider',
  spacer: 'spacer',
  badge: 'badge',
  tag: 'tag',
  pill: 'pill',
  table: 'table',
};

/** Names that indicate a container rather than an individual component. */
const CONTAINER_SUFFIXES = /\b(group|row|container|wrapper|section|list|area|stack|grid|bar)s?\b/i;

/**
 * Words that, when combined with a role-like word, turn the node into a
 * PART of that role rather than an instance of it. "Card Header",
 * "Card Body", "Card Footer" are all structural pieces inside a parent
 * card — they must NOT inherit the card's role defaults (white fill,
 * shadow, rounded corners…).
 *
 * Stored as a Set (not a regex) because the check is "is the first word
 * after the role keyword exactly one of these" — position-sensitive and
 * word-scoped, not a substring scan. A substring scan would wrongly
 * reject prepositional variants like "Card with Icon" or "Button with
 * Image", where the part word is separated from the role word by a
 * modifier ("with") and the whole name describes a variant of the role
 * rather than an internal piece of it.
 */
const ROLE_PART_WORDS = new Set([
  'header',
  'body',
  'footer',
  'title',
  'subtitle',
  'content',
  'wrapper',
  'container',
  'area',
  'label',
  'value',
  'caption',
  'description',
  'image',
  'media',
  'icon',
  'action',
  'actions',
  'meta',
  'row',
  'column',
  'stack',
  'grid',
]);

/**
 * Extract the first meaningful alphabetic token from a string, skipping
 * leading whitespace, punctuation, digits, and single-letter fragments.
 * Returns null when no qualifying token exists.
 *
 * Why alpha-only and min-length 2: sub-agents frequently name nodes
 * "Card 1 Content", "Card 2 Header", "Button 3 Label" with a numeric
 * index between the role word and the part word. A naive `\w+` would
 * match the index ("1") and lose the trailing "content"/"header"/
 * "label" that actually determines whether the node is a structural
 * piece. We skip anything non-alpha (or alpha shorter than 2 chars) and
 * scan forward until we land on a real word, so "Card 1 Content"
 * correctly surfaces "content" and gets treated as a card piece.
 */
function firstWordToken(s: string): string | null {
  const m = /[a-z]{2,}/i.exec(s);
  return m ? m[0].toLowerCase() : null;
}

/** Substring patterns → role (checked in order, first match wins). */
const NAME_PATTERN_MAP: [RegExp, string, boolean?][] = [
  [/\bbtn\b|\bbutton\b/i, 'button', true],
  [/\bcard\b/i, 'card', true],
  [/\binput\b|text\s*field|text\s*box/i, 'input'],
  [/\bform\b/i, 'form-group'],
  [/\bsearch/i, 'search-bar'],
  [/\bnav\s*link/i, 'nav-link'],
  [/\bstat/i, 'stat-card', true],
  [/\bpricing/i, 'pricing-card', true],
  [/\btestimonial\b|\breview\b|\bquote\b/i, 'testimonial'],
  [/\bcta\b|call\s*to\s*action/i, 'cta-section'],
  [/\bfeature/i, 'feature-card', true],
  [/\bicon\b/i, 'icon'],
];

/**
 * Infer a semantic role from a node's name when no explicit role is set.
 * Only applies to frame nodes — text, path, image, etc. don't need role inference.
 */
function inferRoleFromName(node: PenNode): string | undefined {
  if (node.type !== 'frame') return undefined;
  const name = node.name;
  if (!name) return undefined;

  const lower = name.toLowerCase().trim();

  // Exact match first
  const exact = NAME_EXACT_MAP[lower];
  if (exact) return exact;

  // Pattern match
  for (const [pattern, role, skipContainers] of NAME_PATTERN_MAP) {
    // Use exec (not test) so we know WHERE in the name the role word sits.
    // Position matters for the ROLE_PART_WORDS guard below.
    const match = pattern.exec(lower);
    if (!match) continue;

    if (skipContainers) {
      // Skip container-like names (e.g. "Button Group", "Buttons Row")
      if (CONTAINER_SUFFIXES.test(lower)) continue;
      // Skip "Card Header", "Card Body", "Button Label", etc. — when the
      // FIRST word after the role keyword is a part word, the node is a
      // PIECE of the role. Two positional guards matter here:
      //   1. We look at the text AFTER the match, so "Icon Button"
      //      (part word before role) is correctly kept as button.
      //   2. We only check the FIRST word in that suffix, so
      //      "Card with Icon" / "Button with Image" (prepositional
      //      variants: "a card that HAS an icon") keep their role —
      //      the first word is "with", not a part word.
      const afterMatch = lower.slice(match.index + match[0].length);
      const nextWord = firstWordToken(afterMatch);
      if (nextWord && ROLE_PART_WORDS.has(nextWord)) continue;
    }
    return role;
  }

  return undefined;
}

// ---------------------------------------------------------------------------
// Per-node resolution
// ---------------------------------------------------------------------------

/**
 * Apply role-based defaults to a single node.
 * Only fills in properties that are NOT already set by the AI.
 * The AI's explicit properties always win.
 * If no explicit role is set, attempts to infer one from the node name.
 */
export function resolveNodeRole(node: PenNode, ctx: RoleContext): void {
  let role = node.role;

  // Infer role from name if not explicitly set
  if (!role) {
    role = inferRoleFromName(node);

    // Page-chrome inference is wrong inside a card-family parent. The
    // LLM frequently names a card's internal sections "Header" and
    // "Footer" (the card's title row and action row, respectively),
    // but `NAME_EXACT_MAP` blindly maps those to the page-level
    // 'navbar' and 'footer' roles — which then inject navbar fill +
    // border or footer padding into the inner card section, turning
    // it into a glaring white bar (the heart-rate "Mini Chart"
    // regression). Strip the inference when the immediate parent is
    // already a card-family role: those are container roles whose
    // children are card pieces, not page chrome.
    if (
      role &&
      PAGE_CHROME_ROLES.has(role) &&
      ctx.parentRole &&
      CARD_LIKE_ROLES.has(ctx.parentRole)
    ) {
      role = undefined;
    }

    if (role) {
      (node as unknown as Record<string, unknown>).role = role;
    }
  }

  if (!role) return;

  // Size sanity check for card-family roles. The `inferRoleFromName`
  // pattern matcher is lexical — a 6×6 node named "Status Dot" trips
  // the `/\bstat/` regex and gets `role: 'stat-card'`, which then
  // injects 24px padding, a card shadow, and cornerRadius. Refuse to
  // apply card-like roles on nodes too small to plausibly be a card;
  // delete the role entirely so downstream passes also treat the
  // node as unroled. This catches both name-inferred and LLM-
  // emitted-directly versions of the same mistake.
  if (CARD_LIKE_ROLES.has(role) && isAbsurdlyTinyForCardRole(node)) {
    delete (node as { role?: string }).role;
    return;
  }

  const ruleFn = roleRegistry.get(role);
  if (!ruleFn) return; // unknown role — pass through unchanged

  const defaults = ruleFn(node, ctx);
  if (!defaults) return;

  applyDefaults(node, defaults);
}

/**
 * Roles that inject heavy visual defaults (padding ≥ 16, card shadow,
 * cornerRadius ≥ 12, fill) which only make sense on a container large
 * enough to hold content. Applying them to a tiny element (e.g. a 6×6
 * status dot whose name happens to match `/\bstat/` and trips the
 * stat-card pattern) silently inflates it into an oversized card with
 * 24px padding and a drop shadow.
 *
 * When `resolveNodeRole` sees a role in this list AND the node's
 * declared width or height is below `CARD_LIKE_MIN_DIMENSION`, the
 * role is stripped (set back to undefined) before any defaults are
 * applied. The node keeps whatever it already had.
 */
const CARD_LIKE_ROLES = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
]);
const CARD_LIKE_MIN_DIMENSION = 40;

/**
 * Roles that only make sense at the top of a page tree — they paint
 * page-level chrome (navbar bar across the top, footer band across
 * the bottom, full-bleed hero block, full-width call-to-action band).
 *
 * When the LLM names a card's INTERNAL sections "Header" or "Footer"
 * (the card's title row / action row), the lexical name match in
 * `NAME_EXACT_MAP` blindly returns 'navbar' / 'footer' — which then
 * injects navbar fill + border or footer padding into a section
 * inside the card, turning it into a glaring white bar that doesn't
 * belong there. `resolveNodeRole` strips any of these inferred roles
 * whose immediate parent is in `CARD_LIKE_ROLES`, on the principle
 * that page-chrome roles cannot live inside a card.
 *
 * search-bar is intentionally NOT included: a search input legitimately
 * appears inside settings cards, profile cards, etc. The visual
 * defaults for search-bar are also harmless (rounded input fill), so
 * even if it's mis-inferred the visual cost is small.
 */
const PAGE_CHROME_ROLES = new Set(['navbar', 'footer', 'hero', 'cta-section']);

/**
 * Read a declared dimension as a pixel number when possible. Returns
 * `null` for `'fill_container'`, `'fit_content'`, `undefined`, or any
 * non-numeric value — those sizing modes don't tell us whether the
 * final render will be small, so we refuse to make a decision from
 * them and fall back to the permissive default (apply the role).
 */
function readDeclaredPixelSize(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  return null;
}

function isAbsurdlyTinyForCardRole(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  const w = readDeclaredPixelSize((node as { width?: unknown }).width);
  const h = readDeclaredPixelSize((node as { height?: unknown }).height);
  // Only reject when BOTH dimensions are declared numbers AND at least
  // one is below the threshold. Unknown/sizing-keyword dimensions are
  // left alone (we can't tell what they'll resolve to).
  if (w == null || h == null) return false;
  return w < CARD_LIKE_MIN_DIMENSION || h < CARD_LIKE_MIN_DIMENSION;
}

/**
 * Apply defaults to a node, only setting properties that are undefined/missing.
 */
function applyDefaults(node: PenNode, defaults: RoleDefaults): void {
  const record = node as unknown as Record<string, unknown>;

  for (const [key, value] of Object.entries(defaults)) {
    if (value === undefined) continue;

    // Only set if the property is not already present on the node
    if (record[key] === undefined) {
      record[key] = value;
    }
  }
}

// ---------------------------------------------------------------------------
// Tree-level resolution
// ---------------------------------------------------------------------------

/**
 * Walk the tree depth-first, resolving roles for each node.
 * This replaces the old applyGenerationHeuristics tree walk.
 */
export function resolveTreeRoles(
  root: PenNode,
  canvasWidth: number,
  parentRole?: string,
  parentLayout?: 'none' | 'vertical' | 'horizontal',
  parentContentWidth?: number,
  isTableContext = false,
  theme?: 'dark' | 'light',
): void {
  // Detect theme from the root node's fill on the first (entry) call.
  // Subsequent recursive calls inherit the parent's resolved theme so
  // every node sees the same value.
  const resolvedTheme = theme ?? detectThemeFromNode(root);

  const ctx: RoleContext = {
    parentRole,
    parentLayout,
    parentContentWidth,
    canvasWidth,
    isTableContext,
    theme: resolvedTheme,
  };

  // Detect CJK in text nodes
  if (root.type === 'text') {
    const text = getTextContentForNode(root);
    ctx.hasCjk = hasCjkText(text);
  }

  resolveNodeRole(root, ctx);

  // Recurse into children
  if (!('children' in root) || !Array.isArray(root.children)) return;

  const nodeW = toSizeNumber(
    ('width' in root ? root.width : undefined) as number | string | undefined,
    0,
  );
  const pad = parsePaddingValues('padding' in root ? root.padding : undefined);
  const contentW = nodeW > 0 ? nodeW - pad.left - pad.right : 0;

  const childTableContext = isTableContext || root.role === 'table' || root.role === 'table-row';

  for (const child of root.children) {
    resolveTreeRoles(
      child,
      canvasWidth,
      root.role,
      'layout' in root ? root.layout : undefined,
      contentW || parentContentWidth,
      childTableContext,
      resolvedTheme,
    );
  }
}

/**
 * Detect light vs dark theme from a node's fill color.
 *
 * Used by `resolveTreeRoles` (and exported for sanitize-time call sites
 * in `design-canvas-ops.ts`) so role default functions can pick a fill
 * that matches the page background instead of always defaulting to
 * light-theme `#FFFFFF`.
 *
 * IMPORTANT: pass the actual PAGE ROOT here, not whatever sub-tree the
 * resolver is currently walking. A card inside a dark page has no fill
 * of its own (the LLM omitted it because it expected the dark page bg
 * to show through) — calling this on the card returns 'light' (default
 * fallback), which is the wrong answer. Always look up the document
 * page root and pass it explicitly when resolving an MCP-emitted
 * subtree before insertion.
 *
 * Heuristic: if the first solid fill color has WCAG relative luminance
 * below 0.3, the design is dark theme. Otherwise (or when the fill is
 * missing / a variable ref / not a solid color) we default to 'light'
 * for backward compatibility with all existing light-theme designs.
 */
export function detectThemeFromNode(node: PenNode): 'dark' | 'light' {
  if (!('fill' in node) || !Array.isArray((node as { fill?: unknown[] }).fill)) return 'light';
  const fills = (node as { fill: Array<{ type?: string; color?: string }> }).fill;
  const first = fills[0];
  if (!first || first.type !== 'solid' || typeof first.color !== 'string') return 'light';
  const color = first.color.trim();
  // Skip variable refs ($color-1, etc.) — we can't resolve them here.
  if (color.startsWith('$')) return 'light';
  const m = color.match(/^#([0-9a-fA-F]{3,8})$/);
  if (!m) return 'light';
  let hex = m[1];
  if (hex.length === 3)
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  if (hex.length !== 6 && hex.length !== 8) return 'light';
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return 'light';
  // sRGB → relative luminance
  const lin = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  const Y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  return Y < 0.3 ? 'dark' : 'light';
}

// ---------------------------------------------------------------------------
// Post-pass: cross-node fixes that need the full tree
// ---------------------------------------------------------------------------

/**
 * Apply cross-node fixes after the full tree has been role-resolved.
 * These fixes need sibling/parent context that per-node rules can't see.
 */
export function resolveTreePostPass(
  root: PenNode,
  canvasWidth: number,
  getNodeById?: (id: string) => PenNode | undefined,
  updateNode?: (id: string, updates: Partial<PenNode>) => void,
  parentNode?: PenNode,
): void {
  if (root.type !== 'frame') return;
  if (!('children' in root) || !Array.isArray(root.children)) return;

  // `updateNode` goes through Zustand's immutable `updateNodeInTree`, which
  // shallow-clones every ancestor along the update path. Once we call it, our
  // `root` parameter reference becomes detached from the store: later direct
  // mutations to `root` (fill/effects) would be silently dropped, and stale
  // reads (e.g. the fill we pass down to children as `parentNode`) could lie
  // about the live tree state. After every updateNode call, re-fetch a fresh
  // reference via `getNodeById` and rebind `currentRoot` + `children` before
  // continuing. Children array identity is preserved across
  // `updateNode(currentRoot.id, patch)` because the patch never touches
  // `children`, but we rebind it via the fresh root for clarity and safety.
  let currentRoot: FrameNode = root as FrameNode;
  const refreshRoot = () => {
    if (!getNodeById) return;
    const fresh = getNodeById(currentRoot.id);
    if (fresh && fresh.type === 'frame') {
      currentRoot = fresh as FrameNode;
    }
  };
  const currentChildren = (): PenNode[] =>
    Array.isArray(currentRoot.children) ? currentRoot.children : [];

  // --- Card row equalization ---
  if (currentRoot.layout === 'horizontal' && currentChildren().length >= 2) {
    equalizeCardRow(currentRoot, currentChildren());
  }

  // --- Horizontal overflow fix ---
  if (
    currentRoot.layout === 'horizontal' &&
    typeof currentRoot.width === 'number' &&
    currentChildren().length >= 2
  ) {
    fixHorizontalOverflow(currentRoot, currentChildren(), canvasWidth);
  }

  // --- Form input consistency ---
  if (
    currentRoot.layout === 'vertical' &&
    currentRoot.width !== 'fit_content' &&
    currentChildren().length >= 2
  ) {
    normalizeFormInputWidths(currentRoot, currentChildren());
  }

  // --- Input trailing icon alignment ---
  if (currentRoot.layout === 'horizontal' && currentChildren().length >= 2) {
    normalizeInputTrailingIconAlignment(currentRoot, currentChildren());
  }

  // --- Placeholder icon repair ---
  repairPlaceholderIcons(currentRoot, parentNode);

  // --- Text height estimation ---
  if (currentRoot.layout && currentRoot.layout !== 'none') {
    fixTextHeights(currentRoot, currentChildren(), canvasWidth);
  }

  // --- Frame height expansion ---
  if (
    typeof currentRoot.height === 'number' &&
    currentRoot.layout &&
    currentRoot.layout !== 'none'
  ) {
    const intrinsic = estimateNodeIntrinsicHeight(currentRoot, undefined, canvasWidth);
    const maxExpansion = currentRoot.height * 1.3;
    if (intrinsic > currentRoot.height && intrinsic <= maxExpansion) {
      if (updateNode) {
        updateNode(currentRoot.id, { height: Math.round(intrinsic) });
        refreshRoot();
      } else {
        (currentRoot as unknown as Record<string, unknown>).height = Math.round(intrinsic);
      }
    }
  }

  // --- clipContent for frames with cornerRadius + image children ---
  if (!currentRoot.clipContent) {
    const cr =
      typeof currentRoot.cornerRadius === 'number'
        ? currentRoot.cornerRadius
        : Array.isArray(currentRoot.cornerRadius) && currentRoot.cornerRadius.length > 0
          ? currentRoot.cornerRadius[0]
          : 0;
    if (cr > 0 && currentChildren().some((c) => c.type === 'image')) {
      if (updateNode) {
        updateNode(currentRoot.id, { clipContent: true } as Partial<PenNode>);
        refreshRoot();
      } else {
        currentRoot.clipContent = true;
      }
    }
  }

  // --- Button foreground contrast ---
  fixButtonForegroundContrast(currentRoot);

  // --- Section background alternation ---
  if (currentRoot.layout === 'vertical' && currentChildren().length >= 3) {
    fixSectionAlternation(currentRoot, currentChildren());
  }

  // --- Orphan container contrast ---
  // This one is the primary motivation for the refreshRoot dance above:
  // it mutates `currentRoot.fill` and `currentRoot.effects` directly, and
  // would silently lose its writes if we still held the stale `root`.
  fixOrphanContainerContrast(currentRoot, parentNode);

  // --- Input sibling fill/stroke consistency ---
  if (currentRoot.layout === 'vertical' && currentChildren().length >= 2) {
    fixInputSiblingConsistency(currentRoot, currentChildren());
  }

  // Recurse. Pass `currentRoot` as the parentNode so descendants see
  // whatever state this pass just wrote (e.g. a newly assigned fill from
  // fixOrphanContainerContrast), not the pre-mutation snapshot.
  for (const child of currentChildren()) {
    resolveTreePostPass(child, canvasWidth, getNodeById, updateNode, currentRoot);
  }
}

// ---------------------------------------------------------------------------
// Visual helpers (exported for testing)
// ---------------------------------------------------------------------------

/**
 * Compute perceived luminance from a hex color string.
 * Returns 0 (black) to 1 (white). Handles #RRGGBB and #RRGGBBAA.
 */
export function hexLuminance(hex: string): number {
  const h = hex.replace('#', '');
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

/**
 * Check if a node has a non-empty fill array.
 * Does NOT distinguish AI-explicit from role-default fills.
 */
/**
 * Returns true when a node has ANY declared fill entry, visible or not.
 *
 * This is the "overwrite protection" predicate: heuristics like
 * `fixOrphanContainerContrast` and `fixSectionAlternation` ask this to
 * decide whether the author has already made a deliberate fill choice
 * that they should respect. An explicit transparent fill
 * (`#00000000`, `"transparent"`, `"none"`) IS a deliberate choice —
 * "I want this container to be see-through" — and must be preserved,
 * not swapped for a default white background.
 *
 * When you instead need to know "will this draw a visible color on
 * screen?" (e.g. to decide whether the button foreground contrast
 * pass needs to paint in a readable color), use `hasVisibleFill`
 * instead.
 */
export function hasFill(node: PenNode): boolean {
  return 'fill' in node && Array.isArray(node.fill) && node.fill.length > 0;
}

/**
 * Returns true when a node has a fill that will actually render a
 * visible color. Differs from `hasFill` by rejecting fills that draw
 * nothing:
 *   - Solid fills whose color is `#00000000`, any 8-digit hex with
 *     `00` alpha, or the CSS keywords `"transparent"` / `"none"`
 *   - Any fill (solid, gradient, image) whose `opacity` field is
 *     `0` (or any non-positive number — negative values are treated
 *     defensively as zero)
 *
 * Use this when deciding whether a node needs a color PAINTED ONTO it
 * (button foreground contrast, focus ring supply, etc.). Do NOT use
 * this to decide whether to overwrite an author's fill choice —
 * transparent is a legitimate choice. See `hasFill` for that case.
 */
export function hasVisibleFill(node: PenNode): boolean {
  if (!('fill' in node) || !Array.isArray(node.fill) || node.fill.length === 0) return false;
  const first = node.fill[0];
  if (!first) return false;
  return !isFillInvisible(first);
}

/** A fill is invisible when its opacity is <= 0 or (for solids) its
 *  color is an explicit-transparent hex / CSS keyword. */
function isFillInvisible(fill: PenFill): boolean {
  const opacity = (fill as { opacity?: unknown }).opacity;
  if (typeof opacity === 'number' && opacity <= 0) return true;
  if (fill.type === 'solid') {
    return isInvisibleColor((fill as SolidFill).color);
  }
  return false;
}

function isInvisibleColor(color: unknown): boolean {
  if (typeof color !== 'string') return false;
  const c = color.trim().toLowerCase();
  if (c === 'transparent' || c === 'none') return true;
  // 8-digit hex with 00 alpha (#RRGGBB00). Valid hex color literal, but
  // it draws nothing.
  if (/^#[0-9a-f]{6}00$/i.test(c)) return true;
  return false;
}

/**
 * Extract the first solid fill color from a node, or undefined.
 * Used by post-pass visual fixes (Tasks 5, 7, 8).
 */
export function getFirstSolidColor(node: PenNode): string | undefined {
  if (!hasFill(node)) return undefined;
  const fills = (node as unknown as { fill: PenFill[] }).fill;
  const solid = fills.find((f): f is SolidFill => f.type === 'solid');
  return solid?.color;
}

// ---------------------------------------------------------------------------
// Post-pass helpers
// ---------------------------------------------------------------------------

function fixButtonForegroundContrast(parent: FrameNode): void {
  if (parent.role !== 'button' && parent.role !== 'icon-button') return;
  // A transparent button has no background color to compute contrast
  // against — nothing to do, and we definitely should not paint text
  // white on an invisible button.
  if (!hasVisibleFill(parent)) return;

  const bgColor = getFirstSolidColor(parent);
  if (!bgColor) return;

  const lum = hexLuminance(bgColor);
  const fgColor = lum < 0.5 ? '#FFFFFF' : '#0F172A';
  const fgFill: PenFill[] = [{ type: 'solid', color: fgColor }];

  if (!('children' in parent) || !Array.isArray(parent.children)) return;

  for (const child of parent.children) {
    const rec = child as unknown as Record<string, unknown>;

    if (child.type === 'text' || child.type === 'icon_font') {
      // `hasVisibleFill` treats transparent-hex placeholder fills as
      // unfilled, so the normalizer's #00000000 leftover does not
      // block contrast from supplying a visible color.
      if (!hasVisibleFill(child)) {
        rec.fill = fgFill;
      }
    } else if (child.type === 'path') {
      const hasStroke = 'stroke' in child && child.stroke != null;
      const hasStrokeFill =
        hasStroke &&
        Array.isArray((child.stroke as PenStroke)?.fill) &&
        (child.stroke as PenStroke).fill!.length > 0;

      if (hasVisibleFill(child)) {
        // fill-style icon — already styled, skip
      } else if (hasStroke && !hasStrokeFill) {
        (child.stroke as unknown as Record<string, unknown>).fill = fgFill;
      } else if (!hasStroke && !hasVisibleFill(child)) {
        rec.fill = fgFill;
      }
    }
  }
}

const SECTION_ROLES = new Set(['section', 'hero', 'cta-section', 'stats-section', 'footer']);
const ALTERNATING_BG = ['#FFFFFF', '#F8FAFC'];

function fixSectionAlternation(parent: FrameNode, children: PenNode[]): void {
  if (parent.layout !== 'vertical') return;

  // Only alternate on light-themed pages. ALTERNATING_BG is hardcoded to
  // #FFFFFF/#F8FAFC, which paints visible white strips over a dark root
  // background — the opposite of what the user wants. Dark themes rely on
  // card/component internal contrast to group sections, not an outer
  // section-background wash. When the parent has no solid fill we fall
  // through to the existing (light-mode) behavior.
  const parentBg = getFirstSolidColor(parent);
  if (parentBg && hexLuminance(parentBg) < 0.5) return;

  const runs: PenNode[][] = [];
  let current: PenNode[] = [];

  for (const child of children) {
    if (child.type === 'frame' && child.role && SECTION_ROLES.has(child.role)) {
      current.push(child);
    } else {
      if (current.length > 0) {
        runs.push(current);
        current = [];
      }
    }
  }
  if (current.length > 0) runs.push(current);

  for (const run of runs) {
    const unfilled = run.filter((c) => !hasFill(c));
    if (unfilled.length < 3) continue;

    let idx = 0;
    for (const section of run) {
      if (!hasFill(section)) {
        (section as unknown as Record<string, unknown>).fill = [
          { type: 'solid', color: ALTERNATING_BG[idx % 2] },
        ];
        idx++;
      }
    }
  }
}

const STRUCTURAL_DENYLIST = new Set([
  'section',
  'row',
  'column',
  'centered-content',
  'form-group',
  'feature-grid',
  'screenshot-frame',
  'phone-mockup',
  'navbar',
  'nav-links',
  'hero',
  'footer',
  'cta-section',
  'stats-section',
  'table',
  'table-row',
  'table-header',
  'spacer',
  'divider',
]);

const CARD_LIKE_ALLOWLIST = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
]);

function fixOrphanContainerContrast(node: FrameNode, parentNode?: PenNode): void {
  if (!parentNode) return;
  if (hasFill(node)) return;
  if (hasFill(parentNode)) return;
  if (isRingLikeDecorativeContainer(node)) return;

  const cr =
    typeof node.cornerRadius === 'number'
      ? node.cornerRadius
      : Array.isArray(node.cornerRadius) && node.cornerRadius.length > 0
        ? node.cornerRadius[0]
        : 0;
  if (cr <= 0) return;

  if (!('children' in node) || !Array.isArray(node.children) || node.children.length === 0) return;

  const role = node.role;
  if (role && STRUCTURAL_DENYLIST.has(role)) return;
  if (role && !CARD_LIKE_ALLOWLIST.has(role)) return;

  const rec = node as unknown as Record<string, unknown>;
  rec.fill = [{ type: 'solid', color: '#FFFFFF' }];
  rec.effects = [
    { type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: '#0000001A' },
    { type: 'shadow', offsetX: 0, offsetY: 1, blur: 2, spread: -1, color: '#0000000F' },
  ];
}

function isRingLikeDecorativeContainer(node: FrameNode): boolean {
  const label = `${node.id ?? ''} ${node.name ?? ''}`.toLowerCase();
  if (!/(ring|circle|progress|activity)/.test(label)) return false;
  if (!node.stroke) return false;

  const width = toSizeNumber(node.width, 0);
  const height = toSizeNumber(node.height, 0);
  if (width <= 0 || height <= 0) return false;

  const roughlySquare = Math.abs(width - height) <= Math.max(2, Math.max(width, height) * 0.08);
  if (!roughlySquare) return false;

  const cr =
    typeof node.cornerRadius === 'number'
      ? node.cornerRadius
      : Array.isArray(node.cornerRadius) && node.cornerRadius.length > 0
        ? node.cornerRadius[0]
        : 0;

  return cr >= Math.min(width, height) * 0.35;
}

function fixInputSiblingConsistency(_parent: FrameNode, children: PenNode[]): void {
  const inputs = children.filter(
    (c) => c.type === 'frame' && (c.role === 'input' || c.role === 'form-input') && hasFill(c),
  );
  if (inputs.length < 2) return;

  const firstColor = getFirstSolidColor(inputs[0]);
  if (!firstColor) return;
  const allMatch = inputs.every((inp) => getFirstSolidColor(inp) === firstColor);
  if (allMatch) return;

  const sourceFill = (inputs[0] as unknown as Record<string, unknown>).fill;
  const sourceStroke = (inputs[0] as unknown as Record<string, unknown>).stroke;

  for (let i = 1; i < inputs.length; i++) {
    const rec = inputs[i] as unknown as Record<string, unknown>;
    rec.fill = sourceFill;
    if (sourceStroke) {
      rec.stroke = sourceStroke;
    }
  }
}

function equalizeCardRow(parent: FrameNode, children: PenNode[]): void {
  if (parent.width === 'fit_content') return;

  const cardCandidates = children.filter(
    (c) =>
      c.type === 'frame' &&
      c.role !== 'divider' &&
      c.role !== 'phone-mockup' &&
      toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  );
  if (cardCandidates.some((c) => 'width' in c && c.width === 'fill_container')) return;

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  );
  if (fixedFrames.length < 2) return;

  const widths = fixedFrames.map((c) => toSizeNumber('width' in c ? c.width : undefined, 0));
  const maxW = Math.max(...widths);
  const minW = Math.min(...widths);
  if (maxW <= 0 || minW / maxW >= 0.6) return;

  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0));
  const maxH = Math.max(...heights);
  const minH = Math.min(...heights);
  if (maxH <= 0 || minH / maxH <= 0.5) return;

  for (const child of fixedFrames) {
    (child as unknown as Record<string, unknown>).width = 'fill_container';
    (child as unknown as Record<string, unknown>).height = 'fill_container';
  }
}

function fixHorizontalOverflow(parent: FrameNode, children: PenNode[], canvasWidth: number): void {
  const parentW = toSizeNumber(parent.width, 0);
  if (parentW <= 0) return;

  const pad = parsePaddingValues(parent.padding);
  const gap = toGapNumber(parent.gap);
  const availW = parentW - pad.left - pad.right;

  let childrenTotalW = 0;
  for (const child of children) {
    const cw = toSizeNumber(
      'width' in child ? (child as { width?: number | string }).width : undefined,
      0,
    );
    if (typeof (child as { width?: unknown }).width === 'number' && cw > 0) {
      childrenTotalW += cw;
    } else {
      childrenTotalW += 80;
    }
  }
  const gapTotal = gap * (children.length - 1);
  childrenTotalW += gapTotal;

  if (childrenTotalW <= availW) return;

  // Strategy 1: Reduce gap
  for (const tryGap of [8, 4]) {
    if (gap > tryGap) {
      const reduced = childrenTotalW - gapTotal + tryGap * (children.length - 1);
      if (reduced <= availW) {
        (parent as unknown as Record<string, unknown>).gap = tryGap;
        childrenTotalW = reduced;
        break;
      }
    }
  }

  // Strategy 2: Expand parent
  if (childrenTotalW > availW) {
    const neededW = Math.round(childrenTotalW + pad.left + pad.right);
    if (neededW > parentW && neededW <= canvasWidth) {
      (parent as unknown as Record<string, unknown>).width = neededW;
    } else if (neededW > canvasWidth * 0.8) {
      (parent as unknown as Record<string, unknown>).width = 'fill_container';
    }
  }
}

function normalizeFormInputWidths(_parent: FrameNode, children: PenNode[]): void {
  const hasFillSibling = children.some(
    (c) => c.type === 'frame' && c.width === 'fill_container' && c.role !== 'divider',
  );
  if (!hasFillSibling) return;

  for (const child of children) {
    if (child.type !== 'frame') continue;
    if (child.role === 'divider') continue;
    if (child.role !== 'input' && child.role !== 'form-input') continue;
    if (typeof child.width !== 'number') continue;
    (child as unknown as Record<string, unknown>).width = 'fill_container';
  }
}

function normalizeInputTrailingIconAlignment(parent: FrameNode, children: PenNode[]): void {
  if (parent.role !== 'input' && parent.role !== 'form-input') return;
  if (parent.justifyContent && parent.justifyContent !== 'start') return;

  const visibleChildren = children.filter((c) => c.visible !== false);
  if (visibleChildren.length < 2) return;

  const trailing = visibleChildren[visibleChildren.length - 1];
  if (!isIconLikeNode(trailing)) return;

  const textChildren = visibleChildren.slice(0, -1).filter((child) => child.type === 'text');
  if (textChildren.length === 0) return;

  // Make text children fill available space so trailing icon is pushed to the
  // right edge while text stays left-aligned. This avoids the centering effect
  // that space_between causes with [icon, text, icon] layouts.
  for (const textChild of textChildren) {
    if (textChild.width !== 'fill_container') {
      (textChild as unknown as Record<string, unknown>).width = 'fill_container';
    }
    if (!textChild.textGrowth) {
      (textChild as unknown as Record<string, unknown>).textGrowth = 'fixed-width';
    }
  }
}

function isIconLikeNode(node: PenNode): boolean {
  if (node.type === 'path' || node.type === 'image') return true;

  if (node.type === 'frame') {
    if (node.role === 'icon' || node.role === 'icon-button') return true;
    const w = toSizeNumber(node.width, 0);
    const h = toSizeNumber(node.height, 0);
    if (w > 0 && h > 0 && Math.max(w, h) <= 32) return true;
  }

  return false;
}

function repairPlaceholderIcons(node: FrameNode, parentNode?: PenNode): void {
  if (!Array.isArray(node.children) || node.children.length === 0) return;

  for (const child of node.children) {
    if (!isPlaceholderCircleIcon(child)) continue;
    const semanticName = inferSemanticIconName(child, node, parentNode);
    if (!semanticName) continue;
    resolveIconPathBySemanticName(child as PathNode, semanticName);
  }
}

function isPlaceholderCircleIcon(node: PenNode): boolean {
  return (
    node.type === 'path' && (node.iconId === 'lucide:circle' || node.iconId === 'feather:circle')
  );
}

function inferSemanticIconName(
  node: PenNode,
  localParent: FrameNode,
  parentNode?: PenNode,
): string | null {
  const candidates = [
    node.name,
    localParent.name,
    ...collectNearbyText(localParent, 2, node),
    ...(parentNode ? collectNearbyText(parentNode, 2, localParent) : []),
    parentNode?.name,
  ]
    .filter((value): value is string => typeof value === 'string' && value.trim().length > 0)
    .map((value) => value.toLowerCase());

  for (const text of candidates) {
    if (/(run|jog|walk|hike|cardio|activity|exercise|training)/.test(text)) return 'activity';
    if (/(workout|workouts|gym|strength|dumbbell|barbell)/.test(text)) return 'dumbbell';
    if (/(yoga|meditation|stretch|profile|account|person|user)/.test(text)) return 'user';
    if (/(nutrition|meal|food|diet|apple|fruit)/.test(text)) return 'apple';
    if (/(today|sun|morning)/.test(text)) return 'sun';
  }

  return null;
}

function collectNearbyText(node: PenNode, depth: number, exclude?: PenNode): string[] {
  if (depth < 0 || node === exclude) return [];

  const out: string[] = [];
  if (node.type === 'text') {
    const content = getTextContentForNode(node).trim();
    if (content) out.push(content);
  } else if (typeof node.name === 'string' && node.name.trim()) {
    out.push(node.name.trim());
  }

  if ('children' in node && Array.isArray(node.children) && depth > 0) {
    for (const child of node.children) {
      if (child === exclude) continue;
      out.push(...collectNearbyText(child, depth - 1, exclude));
    }
  }

  return out;
}

function fixTextHeights(_parent: FrameNode, children: PenNode[], _canvasWidth: number): void {
  for (const child of children) {
    if (child.type !== 'text') continue;
    // Strip explicit pixel heights from text nodes — the layout engine auto-calculates
    // height from content + fontSize + lineHeight. Explicit heights always cause
    // clipping (height too small) or wasted space (height too large).
    if (typeof child.height === 'number' && child.textGrowth !== 'fixed-width-height') {
      delete (child as { height?: unknown }).height;
    }
  }
}
