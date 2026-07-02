import { openDocument, saveDocument, resolveDocPath, getSyncUrl } from '../document-manager';
import {
  findNodeInTree,
  flattenNodes,
  getDocChildren,
  setDocChildren,
  computeLayoutTree,
} from '../utils/node-operations';
import { getMcpHooks } from '../hooks';
import type { PenNode } from '@zseven-w/pen-types';
import type { LayoutEntry } from '../utils/node-operations';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DesignRefineParams {
  filePath?: string;
  rootId: string;
  canvasWidth?: number;
  pageId?: string;
}

interface RefineFix {
  nodeId: string;
  nodeName?: string;
  fix: string;
}

interface DesignRefineResult {
  rootId: string;
  totalNodeCount: number;
  fixes: RefineFix[];
  layoutSnapshot: LayoutEntry[];
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

export async function handleDesignRefine(params: DesignRefineParams): Promise<DesignRefineResult> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);
  const pageId = params.pageId;
  const canvasWidth = params.canvasWidth ?? 1200;

  const allChildren = getDocChildren(doc, pageId);
  const root = findNodeInTree(allChildren, params.rootId);
  if (!root) {
    throw new Error(`Root node not found: ${params.rootId}`);
  }

  const fixes: RefineFix[] = [];

  // Snapshot state before processing for diff
  const beforeSnapshot = captureNodeState(root);

  const hooks = getMcpHooks();
  hooks.resolveTreeRoles?.([root], canvasWidth);
  hooks.resolveTreePostPass?.([root]);

  const flat = flattenNodes([root]);
  for (const node of flat) {
    if (node.type === 'path') hooks.applyIconPathResolution?.([node]);
    if (node.type === 'text') hooks.applyNoEmojiIconHeuristic?.([node]);
  }

  hooks.ensureUniqueNodeIds?.([root]);
  hooks.sanitizeLayoutChildPositions?.([root]);
  hooks.sanitizeScreenFrameBounds?.([root]);

  // Diff to find what was changed
  const afterSnapshot = captureNodeState(root);
  diffSnapshots(beforeSnapshot, afterSnapshot, fixes);

  // Persist
  setDocChildren(doc, allChildren, pageId);
  await saveDocument(filePath, doc);

  // Auto-fill image nodes with placeholder/empty src via image-search API
  const syncUrl = await getSyncUrl();
  if (syncUrl) {
    const walkAndFill = async (node: any) => {
      if (
        node.type === 'image' &&
        (!node.src || node.src.startsWith('data:image/svg+xml;charset=utf-8,%3Csvg'))
      ) {
        const query = node.imageSearchQuery ?? node.name ?? 'placeholder';
        try {
          const res = await fetch(`${syncUrl}/api/ai/image-search`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ query, count: 1 }),
          });
          const data = (await res.json()) as { results?: { thumbUrl: string }[] };
          if (data.results && data.results.length > 0) {
            node.src = data.results[0].thumbUrl;
          }
        } catch {
          // non-fatal: image search unavailable
        }
        await new Promise<void>((r) => setTimeout(r, 3000));
      }
      if (node.children) {
        for (const child of node.children) await walkAndFill(child);
      }
    };
    const children = getDocChildren(doc, pageId);
    for (const child of children) await walkAndFill(child);
    // Persist again with filled image srcs
    setDocChildren(doc, children, pageId);
    await saveDocument(filePath, doc);
  }

  // Build layout snapshot
  const layoutSnapshot = computeLayoutTree([root], allChildren, 3);

  return {
    rootId: params.rootId,
    totalNodeCount: flat.length,
    fixes,
    layoutSnapshot,
  };
}

// ---------------------------------------------------------------------------
// State capture and diffing for fix reporting
// ---------------------------------------------------------------------------

interface NodeState {
  id: string;
  name?: string;
  props: Record<string, unknown>;
}

/** Capture a flat snapshot of all node properties for diffing. */
function captureNodeState(root: PenNode): Map<string, NodeState> {
  const states = new Map<string, NodeState>();
  const flat = flattenNodes([root]);
  for (const node of flat) {
    const props: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(node)) {
      if (key === 'children' || key === 'id') continue;
      props[key] = typeof value === 'object' ? JSON.stringify(value) : value;
    }
    states.set(node.id, { id: node.id, name: node.name, props });
  }
  return states;
}

/** Compare before/after snapshots and report fixes. */
function diffSnapshots(
  before: Map<string, NodeState>,
  after: Map<string, NodeState>,
  fixes: RefineFix[],
): void {
  for (const [id, afterState] of after) {
    const beforeState = before.get(id);
    if (!beforeState) continue;

    const changedProps: string[] = [];
    for (const [key, afterVal] of Object.entries(afterState.props)) {
      const beforeVal = beforeState.props[key];
      if (beforeVal !== afterVal) {
        const displayVal =
          typeof afterVal === 'string' && afterVal.startsWith('{')
            ? key
            : `${key}=${String(afterVal)}`;
        changedProps.push(displayVal);
      }
    }

    if (changedProps.length > 0) {
      fixes.push({
        nodeId: id,
        nodeName: afterState.name,
        fix: `Updated: ${changedProps.join(', ')}`,
      });
    }
  }
}
