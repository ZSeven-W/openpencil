import { create } from 'zustand';
import type { DesignMdSpec } from '@/types/design-md';
import { appStorage } from '@/utils/app-storage';
import { useDocumentStore } from '@/stores/document-store';

/**
 * Design.md lives on `PenDocument.designMd` — per-document, serialized with
 * the .pen/.op file, travels with save/load, no cross-document leak.
 *
 * This store is a thin mirror over `document-store.document.designMd` to
 * preserve the legacy hook API (`useDesignMdStore(s => s.designMd)` and
 * `useDesignMdStore(s => s.setDesignMd)`) used throughout the editor UI.
 *
 * Legacy localStorage keys (`openpencil-design-md:<fileKey>`) are migrated
 * into `document.designMd` on first load of a matching file, then deleted.
 * After the migration window closes (a future release), this adapter can
 * be removed entirely and callers can hit document-store directly.
 */

const LEGACY_STORAGE_PREFIX = 'openpencil-design-md:';
const LEGACY_CURRENT_KEY = 'openpencil-design-md-current-key';

interface DesignMdStoreState {
  designMd: DesignMdSpec | undefined;
  setDesignMd: (spec: DesignMdSpec | undefined) => void;
  /**
   * Called on document load — attempts to migrate legacy per-file
   * localStorage design.md into the opened document if the document has
   * no designMd yet. No-op on untitled docs or when nothing to migrate.
   */
  syncToDocument: (fileName: string | null, filePath: string | null) => void;
  /** Called on new document. Kept for call-site back-compat; no-op now. */
  clearForNewDocument: () => void;
  /** Called once at app start. Wipes the obsolete `CURRENT_KEY_STORAGE` entry. */
  hydrate: () => void;
}

function fileKey(fileName: string | null, filePath: string | null): string | null {
  return filePath ?? fileName ?? null;
}

function readLegacySpec(key: string): DesignMdSpec | null {
  try {
    const raw = appStorage.getItem(LEGACY_STORAGE_PREFIX + key);
    if (!raw) return null;
    const data = JSON.parse(raw) as DesignMdSpec;
    if (data && typeof data === 'object' && typeof data.raw === 'string') {
      return data;
    }
  } catch {
    /* ignore */
  }
  return null;
}

function clearLegacyEntry(key: string): void {
  try {
    appStorage.removeItem(LEGACY_STORAGE_PREFIX + key);
  } catch {
    /* ignore */
  }
}

export const useDesignMdStore = create<DesignMdStoreState>((set) => {
  // Seed from document-store's current doc, then subscribe for changes so
  // UI components selecting `designMd` re-render when the underlying
  // document mutates (e.g. after `loadDocument`, undo/redo, MCP sync).
  set({ designMd: useDocumentStore.getState().document.designMd });
  useDocumentStore.subscribe((state, prev) => {
    if (state.document.designMd !== prev.document.designMd) {
      set({ designMd: state.document.designMd });
    }
  });

  return {
    designMd: undefined,

    setDesignMd: (spec) => {
      useDocumentStore.getState().setDesignMd(spec);
    },

    syncToDocument: (fileName, filePath) => {
      const key = fileKey(fileName, filePath);
      if (!key) return;

      // Migrate legacy per-file localStorage entry into the document once.
      const docState = useDocumentStore.getState();
      if (docState.document.designMd) {
        // Document already carries its own designMd — legacy entry is
        // obsolete, just delete it.
        clearLegacyEntry(key);
        return;
      }
      const legacy = readLegacySpec(key);
      if (legacy) {
        docState.setDesignMd(legacy);
        clearLegacyEntry(key);
      }
    },

    clearForNewDocument: () => {
      // No-op: `newDocument()` produces a fresh PenDocument whose
      // `designMd` is already undefined. The zustand subscription will
      // pick that up automatically.
    },

    hydrate: () => {
      // Wipe the orphan "current file" pointer from the legacy scheme so
      // it cannot repopulate designMd across sessions.
      try {
        appStorage.removeItem(LEGACY_CURRENT_KEY);
      } catch {
        /* ignore */
      }
    },
  };
});
