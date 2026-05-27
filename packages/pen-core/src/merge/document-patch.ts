import type { PenDocument, PenNode, PenPage } from '@zseven-w/pen-types';
import { diffDocuments, type NodePatch } from './node-diff.js';
import { indexNodesById, jsonEqual, type IndexedNode } from './merge-helpers.js';

export type DocumentFieldName = 'name' | 'version' | 'variables' | 'themes';

export type DocumentPatch =
  | NodePatch
  | {
      op: 'set-doc-field';
      field: DocumentFieldName;
      value: unknown;
      before?: unknown;
    }
  | {
      op: 'set-pages-meta';
      pages: Array<{ id: string; name: string }>;
      before?: Array<{ id: string; name: string }>;
    };

const DOC_FIELDS: DocumentFieldName[] = ['name', 'version', 'variables', 'themes'];

export function diffPenDocuments(base: PenDocument, next: PenDocument): DocumentPatch[] {
  const patches: DocumentPatch[] = [];

  for (const field of DOC_FIELDS) {
    if (!jsonEqual(base[field], next[field])) {
      patches.push({
        op: 'set-doc-field',
        field,
        value: deepCloneValue(next[field]),
        before: deepCloneValue(base[field]),
      });
    }
  }

  const basePagesMeta = getPagesMeta(base);
  const nextPagesMeta = getPagesMeta(next);
  if (!jsonEqual(basePagesMeta, nextPagesMeta)) {
    patches.push({
      op: 'set-pages-meta',
      pages: deepCloneValue(nextPagesMeta),
      before: deepCloneValue(basePagesMeta),
    });
  }

  patches.push(...collapseAddedSubtrees(base, next, diffDocuments(base, next)));
  return patches;
}

export function applyDocumentPatches(
  base: PenDocument,
  patches: readonly DocumentPatch[],
): PenDocument {
  let document = cloneDocument(base);
  const nodePatches: NodePatch[] = [];

  for (const patch of patches) {
    if (patch.op === 'set-doc-field') {
      document = applyDocFieldPatch(document, patch);
    } else if (patch.op === 'set-pages-meta') {
      document = applyPagesMetaPatch(document, patch.pages);
    } else {
      nodePatches.push(patch);
    }
  }

  document = applyNodePatches(document, nodePatches);
  assertValidDocumentTree(document);
  return document;
}

function collapseAddedSubtrees(
  base: PenDocument,
  next: PenDocument,
  nodePatches: NodePatch[],
): NodePatch[] {
  const baseIndex = indexNodesById(base);
  const nextIndex = indexNodesById(next);
  const addedIds = new Set(
    nodePatches.filter((patch) => patch.op === 'add').map((patch) => patch.nodeId),
  );

  return nodePatches.flatMap((patch) => {
    if (patch.op !== 'add') return [patch];
    if (patch.parentId && addedIds.has(patch.parentId) && !baseIndex.has(patch.parentId)) {
      return [];
    }

    const nextNode = nextIndex.get(patch.nodeId)?.node;
    if (!nextNode) return [patch];
    const parentWasAdded = patch.parentId ? !baseIndex.has(patch.parentId) : false;
    return [
      {
        ...patch,
        fields: parentWasAdded
          ? cloneNode(nextNode)
          : (cloneAddedNode(nextNode, baseIndex) as Partial<PenNode>),
      },
    ];
  });
}

function applyDocFieldPatch(
  document: PenDocument,
  patch: Extract<DocumentPatch, { op: 'set-doc-field' }>,
): PenDocument {
  const next = { ...document } as Record<string, unknown>;
  if (patch.value === undefined) {
    delete next[patch.field];
  } else {
    next[patch.field] = deepCloneValue(patch.value);
  }
  return next as unknown as PenDocument;
}

function applyPagesMetaPatch(
  document: PenDocument,
  pagesMeta: Array<{ id: string; name: string }>,
): PenDocument {
  const seen = new Set<string>();
  for (const page of pagesMeta) {
    if (!page.id) throw new Error('Page id is required');
    if (seen.has(page.id)) throw new Error(`Duplicate page id: ${page.id}`);
    seen.add(page.id);
  }

  const existingPages = new Map((document.pages ?? []).map((page) => [page.id, page]));
  const nextPages = pagesMeta.map<PenPage>((page) => ({
    id: page.id,
    name: page.name,
    children: cloneNodes(existingPages.get(page.id)?.children ?? []),
  }));

  return {
    ...document,
    pages: nextPages,
    children: nextPages.length > 0 ? [] : cloneNodes(document.children ?? []),
  };
}

function applyNodePatches(document: PenDocument, patches: readonly NodePatch[]): PenDocument {
  let next = document;

  const addPatches = orderAddPatches(patches.filter((patch) => patch.op === 'add'));
  for (const patch of addPatches) {
    next = addNode(next, patch);
  }

  const movePatches = patches
    .filter((patch) => patch.op === 'move')
    .sort((a, b) => (a.index ?? Number.MAX_SAFE_INTEGER) - (b.index ?? Number.MAX_SAFE_INTEGER));
  for (const patch of movePatches) {
    next = moveNode(next, patch);
  }

  for (const patch of patches.filter((patch) => patch.op === 'modify')) {
    next = modifyNode(next, patch);
  }

  for (const patch of patches.filter((patch) => patch.op === 'remove')) {
    next = removeNode(next, patch.nodeId);
  }

  return next;
}

function orderAddPatches(addPatches: NodePatch[]): NodePatch[] {
  const byId = new Map(addPatches.map((patch) => [patch.nodeId, patch]));
  const ordered: NodePatch[] = [];
  const visiting = new Set<string>();
  const visited = new Set<string>();

  const visit = (patch: NodePatch) => {
    if (visited.has(patch.nodeId)) return;
    if (visiting.has(patch.nodeId)) {
      throw new Error(`Add patch cycle detected for node ${patch.nodeId}`);
    }
    visiting.add(patch.nodeId);
    if (patch.parentId) {
      const parentPatch = byId.get(patch.parentId);
      if (parentPatch) visit(parentPatch);
    }
    visiting.delete(patch.nodeId);
    visited.add(patch.nodeId);
    ordered.push(patch);
  };

  for (const patch of addPatches) visit(patch);
  return ordered;
}

function addNode(document: PenDocument, patch: NodePatch): PenDocument {
  if (!patch.fields || typeof patch.fields !== 'object') {
    throw new Error(`Add patch for ${patch.nodeId} is missing node fields`);
  }
  const index = indexNodesById(document);
  if (index.has(patch.nodeId)) return document;

  const node = cloneNode({
    ...(patch.fields as PenNode),
    id: patch.nodeId,
  });
  if (!node.type) {
    throw new Error(`Add patch for ${patch.nodeId} is missing node type`);
  }

  return insertNode(document, patch.pageId ?? null, patch.parentId ?? null, node, patch.index);
}

function moveNode(document: PenDocument, patch: NodePatch): PenDocument {
  const index = indexNodesById(document);
  const existing = index.get(patch.nodeId);
  if (!existing) {
    throw new Error(`Cannot move missing node ${patch.nodeId}`);
  }
  if (findNodeInChildren(childrenOf(existing.node), patch.parentId)) {
    throw new Error(`Cannot move node ${patch.nodeId} into its descendant ${patch.parentId}`);
  }
  if (patch.parentId && (!index.has(patch.parentId) || isDescendant(index, patch.nodeId, patch.parentId))) {
    return document;
  }
  if (
    existing.parentId === (patch.parentId ?? null) &&
    existing.pageId === (patch.pageId ?? null) &&
    existing.index === (patch.index ?? existing.index)
  ) {
    return document;
  }
  if (patch.parentId === patch.nodeId) {
    throw new Error(`Cannot move node ${patch.nodeId} into itself`);
  }
  if (patch.parentId && isDescendant(index, patch.parentId, patch.nodeId)) {
    throw new Error(`Cannot move node ${patch.nodeId} into its descendant ${patch.parentId}`);
  }

  const removed = removeNode(document, patch.nodeId);
  return insertNode(
    removed,
    patch.pageId ?? null,
    patch.parentId ?? null,
    cloneNode(existing.node),
    patch.index,
  );
}

function modifyNode(document: PenDocument, patch: NodePatch): PenDocument {
  if (!patch.fields || Object.keys(patch.fields).length === 0) return document;
  const fields = { ...patch.fields } as Record<string, unknown>;
  if ('id' in fields && fields.id !== patch.nodeId) {
    throw new Error(`Modify patch cannot change node id ${patch.nodeId}`);
  }
  if ('children' in fields) {
    throw new Error(`Modify patch cannot replace children for node ${patch.nodeId}`);
  }
  delete fields.id;

  const updated = updateNode(document, patch.nodeId, fields as Partial<PenNode>);
  if (!indexNodesById(document).has(patch.nodeId)) {
    throw new Error(`Cannot modify missing node ${patch.nodeId}`);
  }
  return updated;
}

function insertNode(
  document: PenDocument,
  pageId: string | null,
  parentId: string | null,
  node: PenNode,
  index?: number,
): PenDocument {
  if (document.pages && document.pages.length > 0) {
    const pageIndex = pageId
      ? document.pages.findIndex((page) => page.id === pageId)
      : document.pages.length === 1
        ? 0
        : -1;
    if (pageIndex < 0) throw new Error(`Target page not found: ${pageId ?? 'default'}`);

    const page = document.pages[pageIndex];
    const children = insertNodeIntoChildren(page.children, parentId, node, index);
    return {
      ...document,
      pages: document.pages.map((item, itemIndex) =>
        itemIndex === pageIndex ? { ...item, children } : item,
      ),
    };
  }

  if (pageId !== null) throw new Error(`Target page not found: ${pageId}`);
  return {
    ...document,
    children: insertNodeIntoChildren(document.children ?? [], parentId, node, index),
  };
}

function insertNodeIntoChildren(
  nodes: PenNode[],
  parentId: string | null,
  node: PenNode,
  index?: number,
): PenNode[] {
  if (parentId === null) return insertAt(nodes, node, index);

  const { nodes: nextNodes, inserted } = insertNodeIntoChildrenInternal(
    nodes,
    parentId,
    node,
    index,
  );
  if (!inserted) throw new Error(`Parent node not found: ${parentId}`);
  return nextNodes;
}

function insertNodeIntoChildrenInternal(
  nodes: PenNode[],
  parentId: string,
  node: PenNode,
  index?: number,
): { nodes: PenNode[]; inserted: boolean } {
  let inserted = false;
  const nextNodes = nodes.map((current) => {
    if (current.id === parentId) {
      if (!canContainChildren(current)) {
        throw new Error(`Node ${parentId} cannot contain children`);
      }
      inserted = true;
      return {
        ...current,
        children: insertAt(childrenOf(current), node, index),
      } as PenNode;
    }
    const children = childrenOf(current);
    if (children.length === 0) return current;
    const result = insertNodeIntoChildrenInternal(children, parentId, node, index);
    if (!result.inserted) return current;
    inserted = true;
    return { ...current, children: result.nodes } as PenNode;
  });

  return { nodes: inserted ? nextNodes : nodes, inserted };
}

function insertAt(nodes: PenNode[], node: PenNode, index?: number): PenNode[] {
  const next = [...nodes];
  const targetIndex =
    index === undefined ? next.length : Math.max(0, Math.min(index, next.length));
  next.splice(targetIndex, 0, node);
  return next;
}

function removeNode(document: PenDocument, nodeId: string): PenDocument {
  if (document.pages && document.pages.length > 0) {
    return {
      ...document,
      pages: document.pages.map((page) => ({
        ...page,
        children: removeNodeFromChildren(page.children, nodeId),
      })),
    };
  }
  return { ...document, children: removeNodeFromChildren(document.children ?? [], nodeId) };
}

function removeNodeFromChildren(nodes: PenNode[], nodeId: string): PenNode[] {
  let changed = false;
  const nextNodes: PenNode[] = [];
  for (const node of nodes) {
    if (node.id === nodeId) {
      changed = true;
      continue;
    }
    const children = childrenOf(node);
    if (children.length === 0) {
      nextNodes.push(node);
      continue;
    }
    const nextChildren = removeNodeFromChildren(children, nodeId);
    if (nextChildren !== children) {
      changed = true;
      nextNodes.push({ ...node, children: nextChildren } as PenNode);
    } else {
      nextNodes.push(node);
    }
  }
  return changed ? nextNodes : nodes;
}

function updateNode(
  document: PenDocument,
  nodeId: string,
  fields: Partial<PenNode>,
): PenDocument {
  if (document.pages && document.pages.length > 0) {
    return {
      ...document,
      pages: document.pages.map((page) => ({
        ...page,
        children: updateNodeInChildren(page.children, nodeId, fields),
      })),
    };
  }
  return { ...document, children: updateNodeInChildren(document.children ?? [], nodeId, fields) };
}

function updateNodeInChildren(
  nodes: PenNode[],
  nodeId: string,
  fields: Partial<PenNode>,
): PenNode[] {
  let changed = false;
  const nextNodes = nodes.map((node) => {
    if (node.id === nodeId) {
      changed = true;
      return { ...node, ...fields } as PenNode;
    }
    const children = childrenOf(node);
    if (children.length === 0) return node;
    const nextChildren = updateNodeInChildren(children, nodeId, fields);
    if (nextChildren === children) return node;
    changed = true;
    return { ...node, children: nextChildren } as PenNode;
  });
  return changed ? nextNodes : nodes;
}

function childrenOf(node: PenNode): PenNode[] {
  return (node as { children?: PenNode[] }).children ?? [];
}

function findNodeInChildren(nodes: PenNode[], nodeId: string | null | undefined): PenNode | null {
  if (!nodeId) return null;
  for (const node of nodes) {
    if (node.id === nodeId) return node;
    const found = findNodeInChildren(childrenOf(node), nodeId);
    if (found) return found;
  }
  return null;
}

function canContainChildren(node: PenNode): boolean {
  return ['frame', 'group', 'rectangle', 'ref'].includes(node.type) || 'children' in node;
}

function isDescendant(index: Map<string, IndexedNode>, nodeId: string, ancestorId: string): boolean {
  let current = index.get(nodeId);
  while (current?.parentId) {
    if (current.parentId === ancestorId) return true;
    current = index.get(current.parentId);
  }
  return false;
}

function assertValidDocumentTree(document: PenDocument): void {
  const seen = new Set<string>();
  const visit = (nodes: PenNode[], ancestors: Set<string>) => {
    for (const node of nodes) {
      if (!node.id) throw new Error('Node id is required');
      if (seen.has(node.id)) throw new Error(`Duplicate node id: ${node.id}`);
      if (ancestors.has(node.id)) throw new Error(`Cycle detected at node ${node.id}`);
      seen.add(node.id);
      const nextAncestors = new Set(ancestors);
      nextAncestors.add(node.id);
      visit(childrenOf(node), nextAncestors);
    }
  };

  if (document.pages && document.pages.length > 0) {
    const pageIds = new Set<string>();
    for (const page of document.pages) {
      if (pageIds.has(page.id)) throw new Error(`Duplicate page id: ${page.id}`);
      pageIds.add(page.id);
      visit(page.children, new Set());
    }
    return;
  }
  visit(document.children ?? [], new Set());
}

function getPagesMeta(document: PenDocument): Array<{ id: string; name: string }> {
  return (document.pages ?? []).map((page) => ({ id: page.id, name: page.name }));
}

function cloneDocument(document: PenDocument): PenDocument {
  return deepCloneValue(document) as PenDocument;
}

function cloneNode(node: PenNode): PenNode {
  return deepCloneValue(node) as PenNode;
}

function cloneAddedNode(node: PenNode, baseIndex: Map<string, IndexedNode>): PenNode {
  const cloned = cloneNode(node);
  const children = childrenOf(cloned);
  if (children.length > 0) {
    (cloned as { children?: PenNode[] }).children = children
      .filter((child) => !baseIndex.has(child.id))
      .map((child) => cloneAddedNode(child, baseIndex));
  }
  return cloned;
}

function cloneNodes(nodes: PenNode[]): PenNode[] {
  return deepCloneValue(nodes) as PenNode[];
}

function deepCloneValue<T>(value: T): T {
  if (value === undefined) return value;
  return structuredClone(value);
}
