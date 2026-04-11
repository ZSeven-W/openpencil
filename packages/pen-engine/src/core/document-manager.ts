import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import {
  createEmptyDocument,
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  insertNodeInTree,
  flattenNodes,
  isDescendantOf,
  getNodeBounds,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
  migrateToPages,
  ensureDocumentNodeIds,
  cloneNodeWithNewIds,
} from '@zseven-w/pen-core';
import { generateId } from '@zseven-w/pen-core';
import type { HistoryManager } from './history-manager.js';

export interface DocumentManagerOptions {
  historyManager: HistoryManager;
  onChange?: (doc: PenDocument) => void;
  getActivePageId?: () => string | null;
}

/**
 * Manages the PenDocument tree with immutable snapshot semantics.
 * Each mutation creates a new document reference (structural sharing).
 * Extracted from apps/web/src/stores/document-store.ts + action slices.
 */
export class DocumentManager {
  private document: PenDocument;
  private history: HistoryManager;
  private onChangeCb?: (doc: PenDocument) => void;
  private getActivePageId: () => string | null;

  constructor(options: DocumentManagerOptions) {
    this.document = ensureDocumentNodeIds(migrateToPages(createEmptyDocument()));
    this.history = options.historyManager;
    this.onChangeCb = options.onChange;
    this.getActivePageId = options.getActivePageId ?? (() => this.document.pages?.[0]?.id ?? null);
  }

  // ── Snapshot ──

  /** Returns the current document (immutable reference). */
  getDocument(): PenDocument {
    return this.document;
  }

  /** Create a fresh empty document. */
  createDocument(): PenDocument {
    return ensureDocumentNodeIds(migrateToPages(createEmptyDocument()));
  }

  /**
   * Load a document, replacing current state. Resets history.
   */
  loadDocument(doc: PenDocument): void {
    this.history.clear();
    this.document = ensureDocumentNodeIds(migrateToPages(doc));
    this.onChangeCb?.(this.document);
  }

  // ── Internal helpers ──

  private getPageChildren(): PenNode[] {
    return getActivePageChildren(this.document, this.getActivePageId());
  }

  private setPageChildren(children: PenNode[]): PenDocument {
    return setActivePageChildren(this.document, this.getActivePageId(), children);
  }

  private mutate(fn: (children: PenNode[]) => PenNode[]): void {
    this.history.push(this.document);
    const children = this.getPageChildren();
    this.document = this.setPageChildren(fn(children));
    this.onChangeCb?.(this.document);
  }

  // ── Node CRUD ──

  addNode(parentId: string | null, node: PenNode, index?: number): void {
    this.mutate((children) => insertNodeInTree(children, parentId, node, index ?? 0));
  }

  updateNode(id: string, updates: Partial<PenNode>): void {
    this.mutate((children) => updateNodeInTree(children, id, updates));
  }

  removeNode(id: string): void {
    this.mutate((children) => removeNodeFromTree(children, id));
  }

  moveNode(id: string, newParentId: string | null, index: number): void {
    const children = this.getPageChildren();
    const node = findNodeInTree(children, id);
    if (!node) return;
    this.history.push(this.document);
    const withoutNode = removeNodeFromTree(children, id);
    const withNode = insertNodeInTree(withoutNode, newParentId, node, index);
    this.document = this.setPageChildren(withNode);
    this.onChangeCb?.(this.document);
  }

  duplicateNode(id: string): string | null {
    const children = this.getPageChildren();
    const node = findNodeInTree(children, id);
    if (!node) return null;
    const cloned = cloneNodeWithNewIds(node);
    const parent = findParentInTree(children, id);
    const parentId = parent ? parent.id : null;
    const siblings = parent && 'children' in parent ? (parent.children ?? []) : children;
    const idx = siblings.findIndex((n) => n.id === id);
    this.mutate((c) => insertNodeInTree(c, parentId, cloned, idx >= 0 ? idx + 1 : 0));
    return cloned.id;
  }

  groupNodes(ids: string[]): string | null {
    if (ids.length < 2) return null;
    const children = this.getPageChildren();
    const nodes = ids.map((id) => findNodeInTree(children, id)).filter(Boolean) as PenNode[];
    if (nodes.length < 2) return null;

    const groupId = generateId();
    const allChildren = this.getPageChildren();
    const nodeBounds = nodes.map((n) => getNodeBounds(n, allChildren));
    const minX = Math.min(...nodeBounds.map((b) => b.x));
    const minY = Math.min(...nodeBounds.map((b) => b.y));
    const maxX = Math.max(...nodeBounds.map((b) => b.x + b.w));
    const maxY = Math.max(...nodeBounds.map((b) => b.y + b.h));
    const groupChildren = nodes.map((n) => ({
      ...n,
      x: (n.x ?? 0) - minX,
      y: (n.y ?? 0) - minY,
    }));

    this.history.push(this.document);
    let newChildren = children;
    for (const id of ids) {
      newChildren = removeNodeFromTree(newChildren, id);
    }
    const group: PenNode = {
      id: groupId,
      type: 'group',
      name: 'Group',
      x: minX,
      y: minY,
      width: maxX - minX,
      height: maxY - minY,
      children: groupChildren,
    } as PenNode;
    newChildren = insertNodeInTree(newChildren, null, group, 0);
    this.document = this.setPageChildren(newChildren);
    this.onChangeCb?.(this.document);
    return groupId;
  }

  ungroupNode(groupId: string): void {
    const children = this.getPageChildren();
    const group = findNodeInTree(children, groupId);
    if (!group || !('children' in group) || !group.children?.length) return;
    const parent = findParentInTree(children, groupId);
    const parentId = parent ? parent.id : null;

    this.history.push(this.document);
    let newChildren = removeNodeFromTree(children, groupId);
    const ungrouped = group.children.map((child: PenNode) => ({
      ...child,
      x: (child.x ?? 0) + (group.x ?? 0),
      y: (child.y ?? 0) + (group.y ?? 0),
    }));
    for (let i = ungrouped.length - 1; i >= 0; i--) {
      newChildren = insertNodeInTree(newChildren, parentId, ungrouped[i], 0);
    }
    this.document = this.setPageChildren(newChildren);
    this.onChangeCb?.(this.document);
  }

  /** Returns the children of the active page (public accessor for DesignEngine). */
  getActivePageChildren(): PenNode[] {
    return this.getPageChildren();
  }

  getNodeById(id: string): PenNode | undefined {
    const allChildren = getAllChildren(this.document);
    return findNodeInTree(allChildren, id) ?? undefined;
  }

  getParentOf(id: string): PenNode | undefined {
    const children = this.getPageChildren();
    return findParentInTree(children, id) ?? undefined;
  }

  getFlatNodes(): PenNode[] {
    return flattenNodes(this.getPageChildren());
  }

  isDescendantOf(nodeId: string, ancestorId: string): boolean {
    return isDescendantOf(this.getPageChildren(), nodeId, ancestorId);
  }
}
