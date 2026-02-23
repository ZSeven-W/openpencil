import type { PenNode, PathNode } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import {
  clamp,
  toSizeNumber,
  toStrokeThicknessNumber,
  extractPrimaryColor,
} from './generation-utils'

// ---------------------------------------------------------------------------
// Core UI icon paths (Lucide-style, 24×24 viewBox)
// Only ~30 high-frequency icons for instant sync resolution during streaming.
// All other icons are resolved asynchronously via the Iconify API proxy.
// ---------------------------------------------------------------------------
const ICON_PATH_MAP: Record<string, { d: string; style: 'stroke' | 'fill' }> = {
  menu:           { d: 'M4 6h16M4 12h16M4 18h16', style: 'stroke' },
  x:              { d: 'M18 6L6 18M6 6l12 12', style: 'stroke' },
  close:          { d: 'M18 6L6 18M6 6l12 12', style: 'stroke' },
  check:          { d: 'M20 6L9 17l-5-5', style: 'stroke' },
  plus:           { d: 'M12 5v14M5 12h14', style: 'stroke' },
  minus:          { d: 'M5 12h14', style: 'stroke' },
  search:         { d: 'M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.35-4.35', style: 'stroke' },
  arrowright:     { d: 'M5 12h14M12 5l7 7-7 7', style: 'stroke' },
  arrowleft:      { d: 'M19 12H5M12 19l-7-7 7-7', style: 'stroke' },
  arrowup:        { d: 'M12 19V5M5 12l7-7 7 7', style: 'stroke' },
  arrowdown:      { d: 'M12 5v14M19 12l-7 7-7-7', style: 'stroke' },
  chevronright:   { d: 'M9 18l6-6-6-6', style: 'stroke' },
  chevronleft:    { d: 'M15 18l-6-6 6-6', style: 'stroke' },
  chevrondown:    { d: 'M6 9l6 6 6-6', style: 'stroke' },
  chevronup:      { d: 'M18 15l-6-6-6 6', style: 'stroke' },
  star:           { d: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z', style: 'fill' },
  heart:          { d: 'M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 000-7.78z', style: 'stroke' },
  home:           { d: 'M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9zM9 22V12h6v10', style: 'stroke' },
  user:           { d: 'M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', style: 'stroke' },
  settings:       { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  gear:           { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  mail:           { d: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zm16 2l-10 7L2 6', style: 'stroke' },
  eye:            { d: 'M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  lock:           { d: 'M19 11H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM7 11V7a5 5 0 0110 0v4', style: 'stroke' },
  bell:           { d: 'M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0', style: 'stroke' },
  play:           { d: 'M5 3l14 9-14 9V3z', style: 'fill' },
  pause:          { d: 'M6 4h4v16H6zM14 4h4v16h-4z', style: 'fill' },
  download:       { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3', style: 'stroke' },
  upload:         { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12', style: 'stroke' },
  globe:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z', style: 'stroke' },
  send:           { d: 'M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z', style: 'stroke' },
  code:           { d: 'M16 18l6-6-6-6M8 6l-6 6 6 6', style: 'stroke' },
  dot:            { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  bullet:         { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  point:          { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  circlefill:     { d: 'M12 12m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0', style: 'fill' },
}

// ---------------------------------------------------------------------------
// Pending async icon resolution tracking
// ---------------------------------------------------------------------------

/** Maps nodeId → normalized icon name for icons that need async resolution */
const pendingIconResolutions = new Map<string, string>()

/**
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 *
 * On local map miss for icon-like names, sets a generic placeholder and
 * records the node for async resolution via the Iconify API.
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return
  const rawName = (node.name ?? node.id ?? '').toLowerCase()
    .replace(/[-_\s]+/g, '')       // normalize separators
    .replace(/(icon|logo)$/, '')   // strip trailing "icon" or "logo"

  const match = ICON_PATH_MAP[rawName]
  if (!match) {
    // Icon-like name but no local match — set placeholder and queue for async
    if (rawName && isIconLikeName(node.name ?? '', rawName)) {
      node.d = GENERIC_ICON_PATH
      if (!node.fill || node.fill.length === 0) {
        node.fill = [{ type: 'solid', color: extractPrimaryColor(node.stroke?.fill) ?? '#64748B' }]
      }
      // Record for async resolution
      pendingIconResolutions.set(node.id, rawName)
    }
    return
  }

  // Replace with verified path data
  node.d = match.d
  applyIconStyle(node, match.style)
}

const EMOJI_REGEX = /[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F]/gu
const GENERIC_ICON_PATH = 'M12 3l2.6 5.27 5.82.84-4.2 4.09.99 5.8L12 16.9l-5.21 2.73.99-5.8-4.2-4.09 5.82-.84L12 3z'

export function applyNoEmojiIconHeuristic(node: PenNode): void {
  if (node.type !== 'text') return
  if (typeof node.content !== 'string' || !node.content) return

  EMOJI_REGEX.lastIndex = 0
  if (!EMOJI_REGEX.test(node.content)) return
  EMOJI_REGEX.lastIndex = 0
  const cleaned = node.content.replace(EMOJI_REGEX, '').replace(/\s{2,}/g, ' ').trim()
  if (cleaned.length > 0) {
    node.content = cleaned
    return
  }

  const iconSize = clamp(toSizeNumber(node.height, toSizeNumber(node.width, node.fontSize ?? 20)), 14, 24)
  const iconFill = extractPrimaryColor('fill' in node ? node.fill : undefined) ?? '#64748B'
  const replacement: PenNode = {
    id: node.id,
    type: 'path',
    name: `${node.name ?? 'Icon'} Path`,
    d: GENERIC_ICON_PATH,
    width: iconSize,
    height: iconSize,
    fill: [{ type: 'solid', color: iconFill }],
  } as PenNode

  if (typeof node.x === 'number') replacement.x = node.x
  if (typeof node.y === 'number') replacement.y = node.y
  if (typeof node.opacity === 'number') replacement.opacity = node.opacity
  if (typeof node.rotation === 'number') replacement.rotation = node.rotation
  replaceNode(node, replacement)
}

// ---------------------------------------------------------------------------
// Async icon resolution via Iconify API proxy
// ---------------------------------------------------------------------------

/**
 * Resolve pending icons asynchronously after streaming completes.
 * Walks the subtree rooted at `rootNodeId`, collects pending entries,
 * fetches from `/api/ai/icon` in parallel, and updates nodes in store.
 */
export async function resolveAsyncIcons(rootNodeId: string): Promise<void> {
  if (pendingIconResolutions.size === 0) return

  const { getNodeById, updateNode } = useDocumentStore.getState()

  // Collect pending entries that belong to this subtree
  const entries: Array<{ nodeId: string; iconName: string }> = []
  collectPendingInSubtree(rootNodeId, getNodeById, entries)
  if (entries.length === 0) return

  // Fetch all in parallel
  const results = await Promise.allSettled(
    entries.map(async ({ nodeId, iconName }) => {
      const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`)
      if (!res.ok) return { nodeId, icon: null }
      const data = (await res.json()) as {
        icon: { d: string; style: 'stroke' | 'fill'; width: number; height: number } | null
      }
      return { nodeId, icon: data.icon }
    }),
  )

  // Apply resolved icons to the store
  for (const result of results) {
    if (result.status !== 'fulfilled') continue
    const { nodeId, icon } = result.value
    pendingIconResolutions.delete(nodeId)

    if (!icon) continue
    const node = getNodeById(nodeId)
    if (!node || node.type !== 'path') continue

    // Build update payload with resolved path + correct styling
    const update: Partial<PenNode> = { d: icon.d }
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'

    if (icon.style === 'stroke') {
      const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
      update.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
      update.fill = []
    } else {
      update.fill = [{ type: 'solid', color: existingColor }]
    }

    updateNode(nodeId, update)
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Check if a name looks like an icon reference (not just any path node). */
function isIconLikeName(originalName: string, normalized: string): boolean {
  // Explicit icon/logo suffix in original name
  if (/icon|logo/i.test(originalName)) return true
  // Short normalized name (likely an icon name, not a complex path description)
  if (normalized.length > 0 && normalized.length <= 30) return true
  return false
}

/** Apply stroke/fill styling to a resolved icon node (caller must ensure path type). */
function applyIconStyle(
  node: PathNode,
  style: 'stroke' | 'fill',
): void {
  if (style === 'stroke') {
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
    const strokeColor = extractPrimaryColor(node.stroke?.fill)
    // Ensure stroke is renderable for line icons
    if (!node.stroke || strokeWidth <= 0 || !strokeColor) {
      node.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
    }
    // Line icons should NOT have opaque fill (transparent to show stroke only)
    if (node.fill && node.fill.length > 0) {
      // Move fill color to stroke if stroke has no color
      const fillColor = extractPrimaryColor(node.fill)
      if (fillColor && node.stroke) {
        node.stroke.fill = [{ type: 'solid', color: fillColor }]
      }
      node.fill = []
    }
  } else {
    // Fill icons must always keep a visible fill.
    const fillColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    node.fill = [{ type: 'solid', color: fillColor }]
    // Remove non-renderable stroke definitions to avoid transparent-only paths.
    if (node.stroke && toStrokeThicknessNumber(node.stroke, 0) <= 0) {
      node.stroke = undefined
    }
  }
}

/** Walk subtree and collect entries from pendingIconResolutions. */
function collectPendingInSubtree(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
  out: Array<{ nodeId: string; iconName: string }>,
): void {
  const iconName = pendingIconResolutions.get(nodeId)
  if (iconName) {
    out.push({ nodeId, iconName })
  }

  const node = getNodeById(nodeId)
  if (!node || !('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    collectPendingInSubtree(child.id, getNodeById, out)
  }
}

function replaceNode(target: PenNode, replacement: PenNode): void {
  const targetRecord = target as unknown as Record<string, unknown>
  for (const key of Object.keys(target)) {
    delete targetRecord[key]
  }
  Object.assign(targetRecord, replacement as unknown as Record<string, unknown>)
}
