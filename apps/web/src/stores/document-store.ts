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
  /** Native 用于就地保存的文件句柄 (File System Access API)。 */
  fileHandle: FileSystemFileHandle | null;
  /** Full 就地保存的 Full 文件路径（绕过 FS Access API）。 */
  filePath: string | null;
  /** Whether “另存为”对话框打开（没有 FS API 的浏览器的回退）。 */
  saveDialogOpen: boolean;

  addNode: (parentId: string | null, node: PenNode, index?: number) => void;
  updateNode: (id: string, updates: Partial<PenNode>) => void;
  removeNode: (id: string) => void;
  moveNode: (
    id: string,
    newParentId: string | null,
    index: number,
    options?: { preserveAbsolutePosition?: boolean },
  ) => void;
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

  // Component 管理
  makeReusable: (nodeId: string) => void;
  detachComponent: (nodeId: string) => string | undefined;

  // Variable 管理
  setVariable: (name: string, definition: VariableDefinition) => void;
  removeVariable: (name: string) => void;
  renameVariable: (oldName: string, newName: string) => void;
  setThemes: (themes: Record<string, string[]>) => void;

  // Page 管理
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

  // --- Save 管道（合并单一入口点） ---
  // save() 保存到现有目标（如果有），否则回退到 saveAs()。
  // saveAs(suggestedName?) 始终显示保存对话框。 The suggestedName，当
  // 提供，覆盖自动派生的建议名称（由旧版本使用）
  // SaveDialog 组件，收集手动文件名输入 - 它 MUST NOT
  // 改变存储状态本身，因此它在此处传递键入的名称，并且仅传递
  // 确认写入后存储更新 fileName/filePath）。
// saveToNewPath(path) writes to a specific given path (Electron-only;
  // 记录错误后在浏览器版本中返回 null）。 Used 由 未来
  // 已经知道目的地的程序流 - 目前没有
  // 树内调用者，但这是设计所需要的，以实现前向兼容。 Like
  // save() 和 saveAs()，任何失败（包括
  // 浏览器构建案例）并仅在成功时发出“已保存”。
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

  // --- Node CRUD（提取到 document-store-node-actions.ts） ---
  ...createNodeActions(set, get),

  // --- Component 管理（提取到 document-store-component-actions.ts） ---
  ...createComponentActions(set, get),

  // --- Variable 管理（提取到 document-store-variable-actions.ts） ---
  ...createVariableActions(set, get),

  // --- Page 管理（提取到 document-store-pages.ts） ---
  ...createPageActions(set, get),

  // --- Lifecycle 操作（保持内联 — 小）---

  applyExternalDocument: (doc) => {
    // Push 当前状态为历史记录，因此 MCP 更改是不可撤销的
    useHistoryStore.getState().pushState(get().document);
    // Normalize 外部文档（填充对象→数组、文本→内容等）
    const normalized = normalizePenDocument(doc);
    const migrated = ensureDocumentNodeIds(migrateToPages(normalized));
    // Preserve activePageId 如果页面仍然存在
    const activePageId = useCanvasStore.getState().activePageId;
    const pageExists = migrated.pages?.some((p) => p.id === activePageId);
    const targetPageId = pageExists ? activePageId : migrated.pages?.[0]?.id;
    // Force ALL 页面上有新的子引用，因此当用户稍后切换到任何页面时，画布同步会检测到更改。
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
    // 最近文件中的 Track
    if (fileName) {
      addRecentFile({ fileName, filePath: filePath ?? null });
    }
    // Set 活动页面到首页
    const firstPageId = migrated.pages?.[0]?.id ?? null;
    useCanvasStore.getState().setActivePageId(firstPageId);
    // Sync design.md 到此文档（延迟导入以避免循环）
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
    // Clear design.md 新文档
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

    // Path 1：Electron 具有已知的 .op 路径 → 就地写入。
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

    // Path 2：Browser 具有有效的 .op 文件句柄 → 就地写入。
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

    // Path 3：No 就地目标 → 委托给 saveAs()，它处理每个后端的对话流。
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
