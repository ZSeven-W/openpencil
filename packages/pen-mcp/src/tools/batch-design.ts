import { openDocument, saveDocument, resolveDocPath, getSyncUrl } from '../document-manager';
import { findNodeInTree, flattenNodes, getDocChildren } from '../utils/node-operations';
import { getMcpHooks } from '../hooks';
import type { PenNode } from '@zseven-w/pen-types';
import { runBatchDesignDsl, countNodes, type OpResult } from './batch-design-dsl';

export interface BatchDesignParams {
  filePath?: string;
  operations: string;
  postProcess?: boolean;
  canvasWidth?: number;
  pageId?: string;
}

/**
 * Thin server-side wrapper around `runBatchDesignDsl`:
 *   1. Resolve the .op file path (or live canvas sentinel)
 *   2. Load the doc via document-manager (node:fs)
 *   3. Run the pure DSL executor, passing a server-flavored image
 *      search fetcher that resolves `getSyncUrl()`
 *   4. Optionally apply post-processing hooks (role resolution,
 *      icon paths, sanitize) to newly-inserted roots
 *   5. Save the mutated doc back to the file
 *
 * `runBatchDesignDsl` itself is pure and lives in `batch-design-dsl.ts`
 * so browser callers can import it without pulling in node:fs via
 * this file's `document-manager` import chain. See that file's
 * docstring for the full executor contract.
 */
export async function handleBatchDesign(params: BatchDesignParams): Promise<{
  results: OpResult[];
  nodeCount: number;
  postProcessed?: boolean;
  errors?: Array<{ line: string; error: string }>;
}> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  const pageId = params.pageId;
  const imageSearchFetcher = makeServerImageSearchFetcher();
  const { results, errors } = await runBatchDesignDsl(doc, params.operations, {
    pageId,
    imageSearchFetcher,
  });

  // --- Post-processing ---
  let postProcessed = false;
  if (params.postProcess) {
    const canvasWidth = params.canvasWidth ?? 1200;
    const children = getDocChildren(doc, pageId);

    // Find root nodes that were inserted (first binding is typically the root)
    const insertedIds = new Set(results.map((r) => r.nodeId));
    const rootTargets: PenNode[] = [];
    for (const id of insertedIds) {
      const node = findNodeInTree(children, id);
      if (node) rootTargets.push(node);
    }

    // If no specific roots found, process all top-level children
    const targets = rootTargets.length > 0 ? rootTargets : children;

    const hooks = getMcpHooks();
    for (const target of targets) {
      hooks.resolveTreeRoles?.([target], canvasWidth);
      hooks.resolveTreePostPass?.([target]);

      const flat = flattenNodes([target]);
      for (const node of flat) {
        if (node.type === 'path') hooks.applyIconPathResolution?.([node]);
        if (node.type === 'text') hooks.applyNoEmojiIconHeuristic?.([node]);
      }

      hooks.ensureUniqueNodeIds?.([target]);
      hooks.sanitizeLayoutChildPositions?.([target]);
      hooks.sanitizeScreenFrameBounds?.([target]);
    }

    postProcessed = true;
  }

  await saveDocument(filePath, doc);

  return {
    results,
    nodeCount: countNodes(getDocChildren(doc, pageId)),
    postProcessed: postProcessed || undefined,
    errors: errors.length > 0 ? errors : undefined,
  };
}

/**
 * Build the server-side image search fetcher. MCP runs in Node, so
 * it can't use a relative URL — must resolve the live canvas sync
 * URL first, then POST to its /api/ai/image-search endpoint.
 *
 * Browser callers of `runBatchDesignDsl` should NOT use this —
 * they can either build a fetcher with a relative URL, or omit the
 * fetcher entirely (image nodes emit with empty src, a downstream
 * pass enriches).
 */
function makeServerImageSearchFetcher() {
  return async (prompt: string): Promise<string | null> => {
    const syncUrl = await getSyncUrl();
    if (!syncUrl) return null;
    try {
      const res = await fetch(`${syncUrl}/api/ai/image-search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query: prompt, count: 1 }),
      });
      const data = (await res.json()) as { results?: Array<{ thumbUrl: string }> };
      const first = data.results?.[0]?.thumbUrl;
      return first ?? null;
    } catch {
      return null;
    }
  };
}

// Re-export the pure executor so existing imports (apps/web tests, etc.)
// keep working. New browser callers should import directly from
// './batch-design-dsl' to avoid the node:fs chain pulled in by this file.
export {
  runBatchDesignDsl,
  countNodes,
  type OpResult,
  type ImageSearchFetcher,
} from './batch-design-dsl';
