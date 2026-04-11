import type { PenNode, PathNode } from '@/types/pen';
import { useDocumentStore } from '@/stores/document-store';
import { toStrokeThicknessNumber, extractPrimaryColor } from './generation-utils';
import { ICON_PATH_MAP, type IconEntry } from './icon-dictionary';

// ---------------------------------------------------------------------------
// Pending async icon resolution tracking
// ---------------------------------------------------------------------------

/** Maps nodeId -> normalized icon name for icons that need async resolution */
export const pendingIconResolutions = new Map<string, string>();

/**
 * Fire an immediate icon fetch during streaming with a short timeout.
 * If the server responds in time, update the node right away and remove it
 * from pendingIconResolutions so post-streaming resolution can skip it.
 * On timeout or failure, the node stays in pendingIconResolutions as a fallback.
 */
export function tryImmediateIconResolution(nodeId: string, iconName: string): void {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 600);

  fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`, { signal: controller.signal })
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      clearTimeout(timer);
      const icon = data?.icon as {
        d: string;
        style: 'stroke' | 'fill';
        width: number;
        height: number;
        iconId?: string;
      } | null;
      if (!icon) return;

      // Still pending (post-streaming resolution hasn't claimed it yet)?
      if (!pendingIconResolutions.has(nodeId)) return;
      pendingIconResolutions.delete(nodeId);

      const { getNodeById, updateNode } = useDocumentStore.getState();
      const node = getNodeById(nodeId);
      if (!node || node.type !== 'path') return;

      const update: Partial<PenNode> = { d: icon.d };
      if (icon.iconId) (update as Partial<PathNode>).iconId = icon.iconId;

      const existingColor =
        extractPrimaryColor('fill' in node ? node.fill : undefined) ??
        extractPrimaryColor(node.stroke?.fill) ??
        '#64748B';

      if (icon.style === 'stroke') {
        const sw = toStrokeThicknessNumber(node.stroke, 0);
        update.stroke = {
          thickness: sw > 0 ? sw : 2,
          fill: [{ type: 'solid', color: existingColor }],
        };
        update.fill = [];
      } else {
        update.fill = [{ type: 'solid', color: existingColor }];
        (update as Partial<PathNode>).stroke = undefined;
      }

      updateNode(nodeId, update);
    })
    .catch(() => clearTimeout(timer));
}

/**
 * Queue an icon_font node for async resolution when lookupIconByName fails.
 * Fetches from /api/ai/icon, caches in ICON_PATH_MAP for future lookups,
 * and triggers node recreation by touching the store node.
 */
export function tryAsyncIconFontResolution(nodeId: string, iconName: string): void {
  const normalized = iconName
    .replace(/[-_\s/]+/g, '')
    .replace(/icon$/i, '')
    .toLowerCase();
  if (!normalized || pendingIconResolutions.has(nodeId)) return;
  pendingIconResolutions.set(nodeId, normalized);

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 1500);

  fetch(`/api/ai/icon?name=${encodeURIComponent(normalized)}`, { signal: controller.signal })
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      clearTimeout(timer);
      if (!pendingIconResolutions.has(nodeId)) return;
      pendingIconResolutions.delete(nodeId);

      const icon = data?.icon as {
        d: string;
        style: 'stroke' | 'fill';
        iconId?: string;
      } | null;
      if (!icon) return;

      // Cache in ICON_PATH_MAP so future lookups resolve instantly
      const entry: IconEntry = {
        d: icon.d,
        style: icon.style,
        iconId: icon.iconId ?? `resolved:${normalized}`,
      };
      if (!ICON_PATH_MAP[normalized]) ICON_PATH_MAP[normalized] = entry;

      // Touch the node in store to trigger canvas recreation
      const { getNodeById, updateNode } = useDocumentStore.getState();
      const node = getNodeById(nodeId);
      if (!node || node.type !== 'icon_font') return;
      // Update iconFontName to the resolved short name (strip "lucide:" / "feather:" prefix)
      // to trigger __needsRecreation and ensure lookupIconByName resolves on next render.
      const resolvedName = (icon.iconId ?? normalized).replace(/^[a-z]+:/, '');
      updateNode(nodeId, { iconFontName: resolvedName } as Partial<PenNode>);
    })
    .catch(() => {
      clearTimeout(timer);
      pendingIconResolutions.delete(nodeId);
    });
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
  if (pendingIconResolutions.size === 0) return;

  const { getNodeById, updateNode } = useDocumentStore.getState();

  // Collect pending entries that belong to this subtree
  const entries: Array<{ nodeId: string; iconName: string }> = [];
  collectPendingInSubtree(rootNodeId, getNodeById, entries);
  if (entries.length === 0) return;

  await fetchAndApplyIconResults(entries, getNodeById, updateNode);
}

/**
 * Resolve ALL pending icons regardless of which subtree they belong to.
 * Use this after non-streaming apply paths (animateNodesToCanvas, applyNodesToCanvas).
 */
export async function resolveAllPendingIcons(): Promise<void> {
  if (pendingIconResolutions.size === 0) return;

  const { getNodeById, updateNode } = useDocumentStore.getState();
  const entries = Array.from(pendingIconResolutions.entries()).map(([nodeId, iconName]) => ({
    nodeId,
    iconName,
  }));

  await fetchAndApplyIconResults(entries, getNodeById, updateNode);
}

async function fetchAndApplyIconResults(
  entries: Array<{ nodeId: string; iconName: string }>,
  getNodeById: ReturnType<typeof useDocumentStore.getState>['getNodeById'],
  updateNode: ReturnType<typeof useDocumentStore.getState>['updateNode'],
): Promise<void> {
  // Fetch all in parallel
  const results = await Promise.allSettled(
    entries.map(async ({ nodeId, iconName }) => {
      const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`);
      if (!res.ok) return { nodeId, icon: null };
      const data = (await res.json()) as {
        icon: {
          d: string;
          style: 'stroke' | 'fill';
          width: number;
          height: number;
          iconId?: string;
        } | null;
      };
      return { nodeId, icon: data.icon };
    }),
  );

  // Apply resolved icons to the store
  for (const result of results) {
    if (result.status !== 'fulfilled') continue;
    const { nodeId, icon } = result.value;
    pendingIconResolutions.delete(nodeId);

    if (!icon) continue;
    const node = getNodeById(nodeId);
    if (!node || node.type !== 'path') continue;

    // Build update payload with resolved path + correct styling
    const update: Partial<PenNode> = { d: icon.d };
    if (icon.iconId) (update as Partial<PathNode>).iconId = icon.iconId;
    const existingColor =
      extractPrimaryColor('fill' in node ? node.fill : undefined) ??
      extractPrimaryColor(node.stroke?.fill) ??
      '#64748B';

    if (icon.style === 'stroke') {
      const strokeWidth = toStrokeThicknessNumber(node.stroke, 0);
      update.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      };
      update.fill = [];
    } else {
      update.fill = [{ type: 'solid', color: existingColor }];
      // Clear any stroke left over from the placeholder (brand icons are fill-only)
      (update as Partial<PathNode>).stroke = undefined;
    }

    updateNode(nodeId, update);
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Walk subtree and collect entries from pendingIconResolutions. */
function collectPendingInSubtree(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
  out: Array<{ nodeId: string; iconName: string }>,
): void {
  const iconName = pendingIconResolutions.get(nodeId);
  if (iconName) {
    out.push({ nodeId, iconName });
  }

  const node = getNodeById(nodeId);
  if (!node || !('children' in node) || !Array.isArray(node.children)) return;
  for (const child of node.children) {
    collectPendingInSubtree(child.id, getNodeById, out);
  }
}
