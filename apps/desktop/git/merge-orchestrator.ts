// apps/desktop/git/merge-orchestrator.ts
//
// Single-file mode merge orchestration. Loads three PenDocument blobs from
// git, calls pen-core's mergeDocuments, and produces the wire-format
// ConflictBag with stable ids. Also applies user resolutions to a merged
// document during applyMerge.

import {
  mergeDocuments,
  type MergeResult,
  type NodeConflict,
  type DocFieldConflict,
} from '@zseven-w/pen-core';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';

import { GitError } from './error';
import { readBlobAtCommit, type IsoRepoHandle } from './git-iso';
import {
  buildConflictBag,
  parseConflictId,
  type ConflictBag,
  type ConflictResolution,
} from './merge-session';

/**
 * Load the tracked file's content at three commits, JSON.parse, and run the
 * pen-core merge. Returns BOTH the raw MergeResult (so the caller can stash
 * it for later applyResolutions) AND the wire-format ConflictBag (so the
 * caller can return it across IPC immediately).
 */
export async function runMerge(opts: {
  handle: IsoRepoHandle;
  filepath: string;
  oursCommit: string;
  theirsCommit: string;
  baseCommit: string;
}): Promise<{
  result: MergeResult;
  bag: ConflictBag;
  conflictMap: Map<string, NodeConflict | DocFieldConflict>;
}> {
  const { handle, filepath, oursCommit, theirsCommit, baseCommit } = opts;

  const [oursStr, theirsStr, baseStr] = await Promise.all([
    readBlobAtCommit({ handle, filepath, commitHash: oursCommit }),
    readBlobAtCommit({ handle, filepath, commitHash: theirsCommit }),
    readBlobAtCommit({ handle, filepath, commitHash: baseCommit }),
  ]);

  let ours: PenDocument;
  let theirs: PenDocument;
  let base: PenDocument;
  try {
    ours = JSON.parse(oursStr) as PenDocument;
    theirs = JSON.parse(theirsStr) as PenDocument;
    base = JSON.parse(baseStr) as PenDocument;
  } catch (err) {
    throw new GitError('engine-crash', `Failed to parse PenDocument blobs for merge`, {
      cause: err,
      detail: { filepath, oursCommit, theirsCommit, baseCommit },
    });
  }

  const result = mergeDocuments({ base, ours, theirs });
  const { bag, conflictMap } = buildConflictBag(result);
  return { result, bag, conflictMap };
}

/**
 * Apply the user's conflict resolutions to a merged document. Returns a new
 * PenDocument with the chosen versions substituted in. Does NOT mutate the
 * input merged document.
 *
 * Resolution semantics:
 *   - 'ours' → leave the merged tree unchanged at that node/field (pen-core's
 *     mergeDocuments already places ours as the placeholder for unresolved
 *     conflicts, so 'ours' is a no-op).
 *   - 'theirs' → replace the conflicted node/field with the theirs version.
 *   - 'manual-node' → replace the conflicted node with the user's edited version.
 *   - 'manual-field' → set the doc-field value to the user's choice.
 *
 * If a conflict has no resolution in the map, we default to 'ours' (the
 * pen-core placeholder). The applyMerge engine fn enforces "all conflicts
 * must be resolved" before calling this — but defaulting here makes the
 * function safe to call in tests with partial resolution maps.
 */
export function applyResolutions(opts: {
  merged: PenDocument;
  conflictMap: Map<string, NodeConflict | DocFieldConflict>;
  resolutions: Map<string, ConflictResolution>;
}): PenDocument {
  const { conflictMap, resolutions } = opts;
  // We build a new document by deep-cloning via JSON round-trip. The merge
  // result is already a fresh object from pen-core, but we don't want to
  // mutate it in case the caller still holds a reference for diagnostics.
  let doc = JSON.parse(JSON.stringify(opts.merged)) as PenDocument;

  for (const [id, resolution] of resolutions) {
    const parsed = parseConflictId(id);
    const conflict = conflictMap.get(id);
    if (!conflict) {
      throw new GitError('engine-crash', `resolveConflict: unknown id ${id}`);
    }

    if (parsed.kind === 'node') {
      const nodeConflict = conflict as NodeConflict;
      const target = pickNodeForResolution(resolution, nodeConflict);
      if (target === null) {
        // 'ours' (with ours being null = deleted) — drop the node from the tree.
        doc = removeNodeById(doc, parsed.pageId, parsed.nodeId);
      } else {
        doc = replaceNodeById(doc, parsed.pageId, parsed.nodeId, target);
      }
    } else {
      // doc-field conflict
      const fieldConflict = conflict as DocFieldConflict;
      const value = pickFieldForResolution(resolution, fieldConflict);
      doc = setDocFieldByPath(doc, parsed.field, parsed.path, value);
    }
  }

  return doc;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function pickNodeForResolution(
  resolution: ConflictResolution,
  conflict: NodeConflict,
): PenNode | null {
  if (resolution.kind === 'ours') return conflict.ours;
  if (resolution.kind === 'theirs') return conflict.theirs;
  if (resolution.kind === 'manual-node') return resolution.node;
  // manual-field is a programming error for node conflicts; treat as ours.
  return conflict.ours;
}

function pickFieldForResolution(
  resolution: ConflictResolution,
  conflict: DocFieldConflict,
): unknown {
  if (resolution.kind === 'ours') return conflict.ours;
  if (resolution.kind === 'theirs') return conflict.theirs;
  if (resolution.kind === 'manual-field') return resolution.value;
  // manual-node is a programming error for doc-field conflicts; treat as ours.
  return conflict.ours;
}

/**
 * Replace a node in the document tree by id. Walks both the legacy
 * single-page `children` and the multi-page `pages` shape.
 *
 * NOTE: we do NOT use pen-core's `updateNodeInTree` here because its
 * semantics are shallow-merge (`{...oldNode, ...updates}`), which would
 * leave stale fields from the old node if the replacement changes type or
 * omits properties. Conflict resolution requires wholesale replacement —
 * the user's chosen node (from theirs or manual edit) must fully supplant
 * the old one with no residual fields leaking through.
 */
function replaceNodeById(
  doc: PenDocument,
  pageId: string | null,
  nodeId: string,
  replacement: PenNode,
): PenDocument {
  if (doc.pages && pageId !== null) {
    return {
      ...doc,
      pages: doc.pages.map((page) =>
        page.id === pageId
          ? { ...page, children: replaceNodeInArray(page.children, nodeId, replacement) }
          : page,
      ),
    };
  }
  // Legacy single-page or null pageId.
  return {
    ...doc,
    children: replaceNodeInArray(doc.children ?? [], nodeId, replacement),
  };
}

/**
 * Recursive tree walker that swaps a node by id with a wholesale replacement.
 * Returns a new array — does not mutate the input.
 */
function replaceNodeInArray(nodes: PenNode[], id: string, replacement: PenNode): PenNode[] {
  return nodes.map((n) => {
    if (n.id === id) return replacement;
    if ('children' in n && n.children) {
      return {
        ...n,
        children: replaceNodeInArray(n.children, id, replacement),
      } as PenNode;
    }
    return n;
  });
}

/**
 * Remove a node from the document tree by id. Returns a new document.
 */
function removeNodeById(doc: PenDocument, pageId: string | null, nodeId: string): PenDocument {
  if (doc.pages && pageId !== null) {
    return {
      ...doc,
      pages: doc.pages.map((page) =>
        page.id === pageId
          ? { ...page, children: removeNodeFromArray(page.children, nodeId) }
          : page,
      ),
    };
  }
  return {
    ...doc,
    children: removeNodeFromArray(doc.children ?? [], nodeId),
  };
}

function removeNodeFromArray(nodes: PenNode[], id: string): PenNode[] {
  const out: PenNode[] = [];
  for (const n of nodes) {
    if (n.id === id) continue;
    if ('children' in n && n.children) {
      out.push({ ...n, children: removeNodeFromArray(n.children, id) } as PenNode);
    } else {
      out.push(n);
    }
  }
  return out;
}

/**
 * Set a doc-field value by its dotted path. Used for variables/themes/etc.
 * The path format matches DocFieldConflict.path (e.g.
 * 'variables.color-1.value' or 'pages').
 */
function setDocFieldByPath(
  doc: PenDocument,
  field: string,
  path: string,
  value: unknown,
): PenDocument {
  // Split the path into segments. The first segment matches `field`; the
  // remaining segments navigate into nested object properties.
  const segments = path.split('.');
  if (segments.length === 0 || segments[0] !== field) {
    // Top-level field, no nesting (e.g. field === 'name', path === 'name').
    return { ...doc, [field]: value } as unknown as PenDocument;
  }

  if (segments.length === 1) {
    return { ...doc, [field]: value } as unknown as PenDocument;
  }

  // Clone the field value and navigate to the parent of the leaf.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const docAny = doc as any;
  const fieldValue = docAny[field];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cloned: any = JSON.parse(JSON.stringify(fieldValue ?? {}));
  let cursor = cloned;
  for (let i = 1; i < segments.length - 1; i++) {
    const seg = segments[i];
    if (cursor[seg] == null) cursor[seg] = {};
    cursor = cursor[seg];
  }
  cursor[segments[segments.length - 1]] = value;

  return { ...doc, [field]: cloned } as unknown as PenDocument;
}
