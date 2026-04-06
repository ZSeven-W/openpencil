import type { PenNode } from '@zseven-w/pen-types';

/**
 * Detect and repair "fake phone mockup" frames — frames that look like a
 * phone mockup (cornerRadius 28-40 + width 240-320, or literally named
 * "Phone Mockup") but are stuffed with multiple children or use a horizontal/
 * vertical layout. A genuine phone mockup is `ONE` frame with at most one
 * placeholder child and `layout: 'none'`.
 *
 * Failure mode this addresses: weaker AI sub-agents (M2 family, GLM, Kimi)
 * sometimes read a generic "Phone mockup = ONE frame, cornerRadius 32" prompt
 * fragment and apply those visual properties to their own root frame, then
 * shove the entire section content inside. The resulting wrapper has a
 * 260px-wide horizontal layout that compresses every section into a 40px
 * column — visually a complete mess (see 2026-04-06 health-tracker case).
 *
 * Two recovery modes:
 *
 * 1. **Unwrap (preferred)** — when the fake wrapper is a CHILD of `node`, we
 *    drop it and promote its children up one level. Visual styling is
 *    discarded along with the wrapper.
 *
 * 2. **Sanitize (fallback)** — when `node` ITSELF is the fake wrapper (the
 *    sub-agent's own root frame is the fake; we have no parent in scope),
 *    we strip the phone bezel visuals (cornerRadius, fixed width, layout) and
 *    rename it. Children stay; they may be duplicates, but at least they
 *    render in a normal vertical stack instead of being crushed into a 260px
 *    horizontal column.
 *
 * Returns `true` if any node was mutated. Callers operating on store-owned
 * nodes MUST publish a Zustand update when this returns true — the in-place
 * mutation does not by itself notify subscribers, so canvas would otherwise
 * keep showing the old (fake-mockup) state until something else triggered a
 * store write.
 */
export function unwrapFakePhoneMockups(node: PenNode): boolean {
  let changed = false;

  // Mode (2): self-sanitize if the root itself is the fake wrapper.
  if (isFakePhoneMockup(node)) {
    sanitizeFakePhoneMockupRoot(node);
    changed = true;
  }

  if (!('children' in node) || !Array.isArray(node.children)) return changed;

  // Mode (1): drop fake wrappers among the children, promote their content.
  const newChildren: PenNode[] = [];
  let unwrappedAnyChild = false;
  for (const child of node.children) {
    if (isFakePhoneMockup(child)) {
      const grandchildren =
        ('children' in child && Array.isArray(child.children) ? child.children : []) ?? [];
      for (const gc of grandchildren) newChildren.push(gc);
      unwrappedAnyChild = true;
    } else {
      newChildren.push(child);
    }
  }
  if (unwrappedAnyChild) {
    node.children = newChildren;
    changed = true;
  }

  // Recurse into the (possibly updated) children. Promoted grandchildren are
  // visited too — if a fake mockup wrapped another fake mockup, both are
  // handled in a single pass.
  for (const child of node.children) {
    if (unwrapFakePhoneMockups(child)) {
      changed = true;
    }
  }

  return changed;
}

/**
 * Strip phone bezel visual signature from a frame in place. Used when the
 * frame itself is the fake wrapper and we cannot detach it from a parent.
 */
function sanitizeFakePhoneMockupRoot(node: PenNode): void {
  const rec = node as PenNode & {
    name?: string;
    cornerRadius?: unknown;
    width?: unknown;
    height?: unknown;
    layout?: 'none' | 'vertical' | 'horizontal';
    fill?: unknown;
  };
  // Drop misleading visuals
  delete rec.cornerRadius;
  // Restore a sensible container width — fill_container lets the parent's
  // layout decide actual width instead of locking us at 260px.
  rec.width = 'fill_container';
  // Drop the fixed bezel height; let content drive intrinsic height.
  rec.height = 'fit_content';
  // Force vertical so children stack instead of getting compressed
  // horizontally inside a fake bezel.
  rec.layout = 'vertical';
  // Clear the misleading "Phone Mockup" name so downstream role inference
  // doesn't act on it.
  if (typeof rec.name === 'string' && /phone\s*mockup|app\s*mockup/i.test(rec.name)) {
    rec.name = 'Section';
  }
}

function isFakePhoneMockup(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  if (!('children' in node) || !Array.isArray(node.children)) return false;

  // Detection signal #1: literal name match. Models often copy the prompt's
  // wording verbatim into the node name.
  const name = (node.name ?? '').toLowerCase();
  const hasPhoneName = /phone\s*mockup|app\s*mockup|手机\s*样机|手机\s*模型|device\s*frame/.test(
    name,
  );

  // Detection signal #2: visual signature. The combination of large radius
  // (>= 28) and narrow width (240-320) is the classic phone bezel.
  const cornerR = readCornerRadius(node);
  const widthNum = typeof node.width === 'number' ? node.width : null;
  const hasPhoneShape =
    cornerR != null &&
    cornerR >= 28 &&
    cornerR <= 40 &&
    widthNum != null &&
    widthNum >= 240 &&
    widthNum <= 320;

  if (!hasPhoneName && !hasPhoneShape) return false;

  // A real phone mockup is ONE frame with at most one placeholder child and
  // `layout: 'none'` (or layout unset). Anything more elaborate is a sub-agent
  // mistake.
  const childCount = node.children.length;
  const layout = (node as PenNode & { layout?: string }).layout;
  const tooManyChildren = childCount > 1;
  const wrongLayout = layout === 'horizontal' || layout === 'vertical';
  return tooManyChildren || wrongLayout;
}

function readCornerRadius(node: PenNode): number | null {
  const cr = (node as PenNode & { cornerRadius?: number | number[] }).cornerRadius;
  if (typeof cr === 'number') return cr;
  if (Array.isArray(cr) && cr.length > 0 && typeof cr[0] === 'number') return cr[0];
  return null;
}
