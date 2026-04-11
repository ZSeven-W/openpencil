import { create } from 'zustand';
import type { PenDocument, PenNode } from '@/types/pen';
import type { VariableDefinition } from '@/types/variables';

import { normalizePenDocument } from '@/utils/normalize-pen-file';
import { addRecentFile } from '@/utils/recent-files';
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
import {
  isElectron,
  supportsFileSystemAccess,
  writeToFileHandle,
  writeToFilePath,
  saveDocumentAs as fsaSaveDocumentAs,
  downloadDocument,
} from '@/utils/file-operations';
import { documentEvents } from '@/utils/document-events';

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

  // --- Save pipeline (consolidated single entry point) ---
  // save() saves to the existing target if any, falls back to saveAs() otherwise.
  // saveAs(suggestedName?) always shows a save dialog. The suggestedName, when
  // provided, overrides the auto-derived suggested name (used by the legacy
  // SaveDialog component which collects a manual filename input — it MUST NOT
  // mutate store state itself, so it passes the typed name here and only the
  // store updates fileName/filePath after a confirmed write).
  // saveToNewPath(path) writes to a specific given path (Electron-only;
  // returns null in browser builds after logging an error). Used by future
  // programmatic flows that already know the destination — currently has no
  // in-tree callers but is required by the design for forward-compat. Like
  // save() and saveAs(), it returns null on any failure (including the
  // browser-build case) and emits 'saved' only on success.
  save: () => Promise<string | null>;
  saveAs: (suggestedName?: string) => Promise<string | null>;
  saveToNewPath: (filePath: string) => Promise<string | null>;
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
    // Track in recent files
    if (fileName) {
      addRecentFile({ fileName, filePath: filePath ?? null });
    }
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

  save: async () => {
    const state = get();
    const { document: doc, fileName, fileHandle, filePath } = state;
    const isOpFile = fileName ? /\.op$/i.test(fileName) : false;

    // Path 1: Electron with a known .op path → in-place write.
    if (isElectron() && filePath && isOpFile) {
      try {
        await writeToFilePath(filePath, doc);
      } catch (err) {
        console.error('[document-store.save] writeToFilePath failed:', err);
        return null;
      }
      set({ isDirty: false });
      documentEvents.emit('saved', { filePath, fileName: fileName!, document: doc });
      return fileName!;
    }

    // Path 2: Browser with a valid .op file handle → in-place write.
    if (fileHandle && isOpFile) {
      try {
        await writeToFileHandle(fileHandle, doc);
        set({ isDirty: false });
        documentEvents.emit('saved', { filePath: null, fileName: fileName!, document: doc });
        return fileName!;
      } catch (err) {
        console.warn('[document-store.save] writeToFileHandle failed, falling back:', err);
        set({ fileHandle: null });
        return get().saveAs();
      }
    }

    // Path 3: No in-place target → delegate to saveAs() which handles the
    // dialog flow per backend.
    return get().saveAs();
  },

  saveAs: async (explicitSuggestedName) => {
    const state = get();
    const { document: doc, fileName } = state;
    const suggestedName = explicitSuggestedName
      ? explicitSuggestedName.endsWith('.op')
        ? explicitSuggestedName
        : `${explicitSuggestedName}.op`
      : fileName
        ? fileName.replace(/\.(pen|op|json)$/i, '') + '.op'
        : 'untitled.op';

    // Path A: Electron native save dialog.
    if (isElectron()) {
      let savedPath: string | null = null;
      try {
        savedPath = await window.electronAPI!.saveFile(JSON.stringify(doc), suggestedName);
      } catch (err) {
        console.error('[document-store.saveAs] electronAPI.saveFile failed:', err);
        return null;
      }
      if (!savedPath) return null; // user cancelled
      const savedName = savedPath.split(/[/\\]/).pop() || suggestedName;
      set({
        fileName: savedName,
        filePath: savedPath,
        fileHandle: null,
        isDirty: false,
      });
      documentEvents.emit('saved', { filePath: savedPath, fileName: savedName, document: doc });
      return savedName;
    }

    // Path B: Browser File System Access API.
    if (supportsFileSystemAccess()) {
      const result = await fsaSaveDocumentAs(doc, suggestedName);
      if (!result) return null; // user cancelled or API error
      set({
        fileName: result.fileName,
        fileHandle: result.handle,
        filePath: null,
        isDirty: false,
      });
      documentEvents.emit('saved', { filePath: null, fileName: result.fileName, document: doc });
      return result.fileName;
    }

    // Path C: Last-resort browser download. We treat the download as a save
    // because the user got the file out — but filePath stays null.
    try {
      downloadDocument(doc, suggestedName);
    } catch (err) {
      console.error('[document-store.saveAs] downloadDocument failed:', err);
      return null;
    }
    set({ fileName: suggestedName, isDirty: false });
    documentEvents.emit('saved', { filePath: null, fileName: suggestedName, document: doc });
    return suggestedName;
  },

  saveToNewPath: async (filePath) => {
    const { document: doc } = get();
    if (!isElectron()) {
      console.error('[document-store.saveToNewPath] not supported in browser builds');
      return null;
    }
    try {
      await writeToFilePath(filePath, doc);
    } catch (err) {
      console.error('[document-store.saveToNewPath] writeToFilePath failed:', err);
      return null;
    }
    const savedName = filePath.split(/[/\\]/).pop() || 'untitled.op';
    set({
      fileName: savedName,
      filePath,
      fileHandle: null,
      isDirty: false,
    });
    documentEvents.emit('saved', { filePath, fileName: savedName, document: doc });
    return savedName;
  },
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
