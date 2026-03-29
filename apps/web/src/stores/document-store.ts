import { create } from 'zustand';
import type { PenDocument, PenNode } from '@/types/pen';
import type { VariableDefinition } from '@/types/variables';

import { normalizePenDocument } from '@/utils/normalize-pen-file';
import { useHistoryStore } from '@/stores/history-store';
import { useCanvasStore } from '@/stores/canvas-store';
import {
  createEmptyDocument,
  migrateToPages,
  ensureDocumentNodeIds,
  DEFAULT_PAGE_ID,
} from './document-tree-utils';
import { createNodeActions } from './document-store-node-actions';
import { createComponentActions } from './document-store-component-actions';
import { createVariableActions } from './document-store-variable-actions';
import { createPageActions } from './document-store-pages';

interface DocumentStoreState {
  document: PenDocument;
  fileName: string | null;
  isDirty: boolean;
  /** Native file handle for save-in-place (File System Access API). */
  fileHandle: FileSystemFileHandle | null;
  /** Full file path for Electron save-in-place (bypasses FS Access API). */
  filePath: string | null;
  /** Whether the "save as" dialog is open (fallback for browsers without FS API). */
  saveDialogOpen: boolean;

  addNode: (parentId: string | null, node: PenNode, index?: number) => void;
  updateNode: (id: string, updates: Partial<PenNode>) => void;
  removeNode: (id: string) => void;
  moveNode: (id: string, newParentId: string | null, index: number) => void;
  reorderNode: (id: string, direction: 'up' | 'down') => void;
  toggleVisibility: (id: string) => void;
  toggleLock: (id: string) => void;
  duplicateNode: (id: string) => string | null;
  groupNodes: (nodeIds: string[]) => string | null;
  ungroupNode: (groupId: string) => void;
  scaleDescendantsInStore: (parentId: string, scaleX: number, scaleY: number) => void;
  rotateDescendantsInStore: (parentId: string, angleDeltaDeg: number) => void;
  getNodeById: (id: string) => PenNode | undefined;
  getParentOf: (id: string) => PenNode | undefined;
  getFlatNodes: () => PenNode[];
  isDescendantOf: (nodeId: string, ancestorId: string) => boolean;

  // Component management
  makeReusable: (nodeId: string) => void;
  detachComponent: (nodeId: string) => string | undefined;

  // Variable management
  setVariable: (name: string, definition: VariableDefinition) => void;
  removeVariable: (name: string) => void;
  renameVariable: (oldName: string, newName: string) => void;
  setThemes: (themes: Record<string, string[]>) => void;

  // Page management
  addPage: () => string;
  removePage: (pageId: string) => void;
  renamePage: (pageId: string, name: string) => void;
  reorderPage: (pageId: string, direction: 'left' | 'right') => void;
  duplicatePage: (pageId: string) => string | null;

  applyExternalDocument: (doc: PenDocument) => void;
  applyHistoryState: (doc: PenDocument) => void;
  loadDocument: (
    doc: PenDocument,
    fileName?: string,
    fileHandle?: FileSystemFileHandle | null,
    filePath?: string | null,
  ) => void;
  newDocument: () => void;
  markClean: () => void;
  setFileHandle: (handle: FileSystemFileHandle | null) => void;
  setSaveDialogOpen: (open: boolean) => void;
}

export const useDocumentStore = create<DocumentStoreState>((set, get) => ({
  document: createEmptyDocument(),
  fileName: null,
  isDirty: false,
  fileHandle: null,
  filePath: null,
  saveDialogOpen: false,

  // --- Node CRUD (extracted to document-store-node-actions.ts) ---
  ...createNodeActions(set, get),

  // --- Component management (extracted to document-store-component-actions.ts) ---
  ...createComponentActions(set, get),

  // --- Variable management (extracted to document-store-variable-actions.ts) ---
  ...createVariableActions(set, get),

  // --- Page management (extracted to document-store-pages.ts) ---
  ...createPageActions(set, get),

  // --- Lifecycle actions (remain inline — small) ---

  applyExternalDocument: (doc) => {
    // Push current state to history so MCP changes are undoable
    useHistoryStore.getState().pushState(get().document);
    // Normalize external document (fill object→array, text→content, etc.)
    const normalized = normalizePenDocument(doc);
    const migrated = ensureDocumentNodeIds(migrateToPages(normalized));
    // Preserve activePageId if page still exists
    const activePageId = useCanvasStore.getState().activePageId;
    const pageExists = migrated.pages?.some((p) => p.id === activePageId);
    const targetPageId = pageExists ? activePageId : migrated.pages?.[0]?.id;
    // Force new children references on ALL pages so canvas sync detects
    // changes when the user later switches to any page.
    if (migrated.pages) {
      for (const page of migrated.pages) {
        page.children = [...page.children];
      }
    }
    set({ document: migrated, isDirty: true });
    if (!pageExists && targetPageId) {
      useCanvasStore.getState().setActivePageId(targetPageId);
    }
  },

  applyHistoryState: (doc) => set({ document: doc, isDirty: true }),

  loadDocument: (doc, fileName, fileHandle, filePath) => {
    useHistoryStore.getState().clear();
    const migrated = ensureDocumentNodeIds(migrateToPages(doc));
    set({
      document: migrated,
      fileName: fileName ?? null,
      fileHandle: fileHandle ?? null,
      filePath: filePath ?? null,
      isDirty: false,
    });
    // Set active page to the first page
    const firstPageId = migrated.pages?.[0]?.id ?? null;
    useCanvasStore.getState().setActivePageId(firstPageId);
    // Sync design.md to this document (lazy import to avoid circular)
    import('@/stores/design-md-store').then(({ useDesignMdStore }) => {
      useDesignMdStore.getState().syncToDocument(fileName ?? null, filePath ?? null);
    });
  },

  newDocument: () => {
    useHistoryStore.getState().clear();
    const doc = createEmptyDocument();
    set({
      document: doc,
      fileName: null,
      fileHandle: null,
      filePath: null,
      isDirty: false,
    });
    useCanvasStore.getState().setActivePageId(doc.pages?.[0]?.id ?? DEFAULT_PAGE_ID);
    // Clear design.md for new document
    import('@/stores/design-md-store').then(({ useDesignMdStore }) => {
      useDesignMdStore.getState().clearForNewDocument();
    });
  },

  markClean: () => set({ isDirty: false }),
  setFileHandle: (fileHandle) => set({ fileHandle }),
  setSaveDialogOpen: (saveDialogOpen) => set({ saveDialogOpen }),
}));

export {
  createEmptyDocument,
  findNodeInTree,
  DEFAULT_FRAME_ID,
  DEFAULT_PAGE_ID,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
  migrateToPages,
} from './document-tree-utils';
export { generateId } from '@/utils/id';

// Sync isDirty to a global so the Electron main process can query it
// via webContents.executeJavaScript for close confirmation.
if (typeof window !== 'undefined') {
  useDocumentStore.subscribe((state) => {
    (window as unknown as Record<string, unknown>).__documentIsDirty = state.isDirty;
  });
}

// Expose stores on window in dev mode for testing/debugging
if (import.meta.env.DEV && typeof window !== 'undefined') {
  (window as unknown as Record<string, unknown>).__documentStore = useDocumentStore;
  (window as unknown as Record<string, unknown>).__canvasStore = useCanvasStore;
}
