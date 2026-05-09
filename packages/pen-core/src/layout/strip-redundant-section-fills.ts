import type { PenNode, PenFill, SolidFill } from '@zseven-w/pen-types';

/**
 * Strip redundant "section-level" fills from the direct children of a
 * page root frame.
 *
 * Weaker sub-agents (MiniMax M2, GLM, Kimi) often hedge by writing a
 * hardcoded dark hex (`#0A0A0A`, `#111`, etc.) on every section root they
 * produce. That hex then completely covers the page root's intended
 * background color, breaking theme switching and creating visible seams
 * between sections. Cards, buttons, chips and other legitimately filled
 * components are NOT affected — only "section container" frames (direct
 * children of the root that either have no role or have a structural
 * role).
 *
 * ⚠️ SCOPE CONTRACT — callers MUST only pass the true page root frame.
 * Calling this on an arbitrary nested frame (a card, a component, a
 * sub-agent's own root that is NOT the page root) will incorrectly treat
 * that frame's inner children as "sections" and may erase intended
 * nested fills (e.g. a card's own dark header). The function itself is
 * strictly non-recursive — it only touches direct children of the passed
 * node — so mis-targeted calls are bounded, but still wrong.
 *
 * Returns `true` when any fill was stripped, so the caller can publish a
 * store update.
 */
export function stripRedundantSectionFills(rootFrame: PenNode): boolean {
  if (!('children' in rootFrame) || !Array.isArray(rootFrame.children)) return false;

  const rootFill = getFirstSolidColor(rootFrame);
  let changed = false;

  for (const child of rootFrame.children) {
    if (child.type !== 'frame') continue;
    if (!isSectionLevelFrame(child)) continue;

    const childFill = getFirstSolidColor(child);
    if (!childFill) continue;

    if (shouldStripFill(childFill, rootFill)) {
      delete (child as PenNode & { fill?: unknown }).fill;
      changed = true;
    }
  }

  return changed;
}

/**
 * Roles that identify visually distinct components and must never have
 * their fill stripped.
 */
const PROTECTED_ROLES = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
  'button',
  'icon-button',
  'badge',
  'chip',
  'tag',
  'pill',
  'input',
  'form-input',
  'search-bar',
  'phone-mockup',
  'banner',
  'metric-card',
  'gallery-item',
  'status-bar',
  // Navigation surfaces — bottom-tab bars, top app bars, and side nav
  // chrome carry their own intentional surface fill (often the cream/white
  // surface tone that visually separates them from the cream root frame).
  // Without protection the strip pass would erase that fill on light-mode
  // docs because cream surface (#FFFFFF / #F1F5F9) is in SAFE_LIGHT_HEXES.
  'navbar',
  'nav',
  'tab-bar',
  'bottom-tab-bar',
  'top-nav-bar',
]);

/**
 * Subset of PROTECTED_ROLES that represent atomic (single-purpose)
 * components — search bars, buttons, badges, etc. These do NOT normally
 * compose other atomic components inside them, so when a frame with this
 * role contains another atomic-role child carrying its own solid fill, the
 * outer is almost always a sub-agent-misrolled wrapper rather than the
 * real atom (e.g. search-bar > input). Container components (card,
 * pricing-card, banner, …) are EXCLUDED from this set: a card legitimately
 * holds buttons/badges/chips with their own fills, and stripping the
 * card's fill in that case would erase its intended surface color.
 */
const ATOMIC_PROTECTED_ROLES = new Set([
  'button',
  'icon-button',
  'badge',
  'chip',
  'tag',
  'pill',
  'input',
  'form-input',
  'search-bar',
]);

/**
 * Subset of ATOMIC_PROTECTED_ROLES that represent PRIMARY atomics —
 * input-class components that constitute the "main" atom in their own
 * right. When one of these appears as a CHILD of a different atomic
 * frame, it's almost certainly a sub-agent misroll (the outer was named
 * 'search-bar' but is actually a section wrapper around the real
 * 'input'). Wrapper detection only triggers on this primary set.
 *
 * SECONDARY atomic roles (button, icon-button, badge, chip, tag, pill)
 * are intentionally excluded — they legitimately nest inside primary
 * atomics as sub-actions or decorations (an input with a clear icon-
 * button, a search-bar with a voice-search button, an input with a
 * reveal-password icon-button). Treating those as wrapper signals
 * would erase the parent atom's intended surface fill.
 */
const PRIMARY_ATOMIC_ROLES = new Set(['input', 'form-input', 'search-bar']);

/**
 * Roles that are considered structural — just a container grouping other
 * nodes. These are candidates for fill stripping when they echo the root
 * background or hedge with a safe-dark fill.
 */
const STRUCTURAL_ROLES = new Set([
  'section',
  'row',
  'column',
  'stack',
  'container',
  'content-area',
  'section-header',
  'wrapper',
  'group',
  'hero',
  'footer',
  'cta-section',
  'stats-section',
]);

function isSectionLevelFrame(node: PenNode): boolean {
  const role = (node as PenNode & { role?: string }).role;
  if (!role) return true; // unrolled section root
  if (PROTECTED_ROLES.has(role)) {
    // A protected-role frame at section depth that ALSO contains a
    // descendant of the SAME role (or a stand-alone fill-bearing child
    // that visibly carries the component's true surface) is almost
    // certainly a sub-agent-misrolled wrapper, NOT the real atom — e.g.
    // sub-agent emits Search Bar(role=search-bar) > Search Input
    // Container(role=input). The outer "search-bar" is just a section
    // wrapper. Treat as section-level so its hedge fill can be stripped.
    if (hasNestedFilledComponent(node, role)) return true;
    return false;
  }
  if (STRUCTURAL_ROLES.has(role)) return true;
  // Unknown role: be conservative, treat as protected so we don't clobber
  // future role additions.
  return false;
}

/**
 * True when a frame is the outer wrapper of a misrolled component pattern:
 *
 *   - The OUTER frame's role is an atomic-component role (search-bar,
 *     button, input, badge, chip, …). Container-component roles (card,
 *     pricing-card, banner, …) are NEVER treated as wrappers — those
 *     legitimately hold filled children like buttons / badges / chips.
 *
 *   - One of its CHILDREN carries either the same role (nested twin) or
 *     a PRIMARY atomic role with its own solid fill (search-bar > input,
 *     form-input > input). Secondary atomic roles (icon-button / button /
 *     badge / chip / tag / pill) inside a primary atomic are legitimate
 *     composition (input + trailing clear button, search-bar + voice
 *     button) — they do NOT signal a wrapper.
 */
function hasNestedFilledComponent(node: PenNode, parentRole: string): boolean {
  if (!ATOMIC_PROTECTED_ROLES.has(parentRole)) return false;
  if (!('children' in node) || !Array.isArray(node.children)) return false;
  for (const child of node.children) {
    const childRole = (child as PenNode & { role?: string }).role;
    if (!childRole) continue;
    // Same atomic role nested again ('search-bar' inside 'search-bar').
    if (childRole === parentRole) return true;
    // Different PRIMARY atomic role with its own solid fill
    // (search-bar > input). Secondary atomics inside a primary atomic
    // (input > icon-button, search-bar > button) are real composition,
    // not wrappers — those keep the parent's fill intact.
    if (PRIMARY_ATOMIC_ROLES.has(childRole) && getFirstSolidColor(child)) return true;
  }
  return false;
}

/**
 * Hex tints that sub-agents reach for when they want a "safe dark"
 * background without knowing the real design background color. Any of
 * these on a section container is almost certainly a hedge, not an
 * intentional visual choice.
 */
const SAFE_DARK_HEXES = new Set([
  '#000000',
  '#000',
  '#0a0a0a',
  '#0f0f0f',
  '#111',
  '#111111',
  '#121212',
  '#141414',
  '#1a1a1a',
  '#181818',
  '#1c1c1c',
  '#1e1e1e',
  '#202020',
]);

/**
 * Light-mode counterparts of SAFE_DARK_HEXES. Two sources produce these
 * on section roots:
 *   1. Weaker sub-agents that hedge with pure white / near-white instead
 *      of the role-correct transparent fill.
 *   2. The legacy `fixSectionAlternation` post-pass that painted an
 *      #FFFFFF / #F8FAFC ladder on every run of ≥3 unfilled sections.
 *      On dark pages that ladder leaves visible white strips; if a doc
 *      with those stale fills is re-opened or re-rendered after the
 *      alternation skip landed, the fills are still there and the skip
 *      alone won't remove them.
 * Treat these the same as safe-dark hedges: strip on any section root.
 */
const SAFE_LIGHT_HEXES = new Set([
  '#ffffff',
  '#fff',
  '#fefefe',
  '#fdfdfd',
  '#fcfcfc',
  '#fafafa',
  '#f9fafb',
  '#f8f8f8',
  '#f8fafc',
  '#f5f5f5',
  '#f4f4f5',
  '#f3f4f6',
]);

function shouldStripFill(childFill: string, rootFill: string | null): boolean {
  const childKey = normalizeHex(childFill);
  if (rootFill) {
    const rootKey = normalizeHex(rootFill);
    if (childKey === rootKey) return true;
  }
  return SAFE_DARK_HEXES.has(childKey) || SAFE_LIGHT_HEXES.has(childKey);
}

function normalizeHex(color: string): string {
  let c = color.trim().toLowerCase();
  // Strip alpha if present (#rrggbbaa → #rrggbb)
  if (c.length === 9 && c.startsWith('#')) c = c.slice(0, 7);
  return c;
}

function getFirstSolidColor(node: PenNode): string | null {
  const fill = (node as PenNode & { fill?: PenFill[] | string }).fill;
  if (!fill) return null;
  if (typeof fill === 'string') return fill;
  if (!Array.isArray(fill) || fill.length === 0) return null;
  const first = fill[0];
  if (first && first.type === 'solid') {
    return (first as SolidFill).color;
  }
  return null;
}
