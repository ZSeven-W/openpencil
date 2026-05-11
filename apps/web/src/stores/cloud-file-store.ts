import { create } from 'zustand';
import type { CloudFileSummary } from '@/types/cloud';
import { createCloudFile, deleteCloudFile, listCloudFiles } from '@/services/cloud/cloud-files';
import type { PenDocument } from '@/types/pen';

interface CloudFileState {
  files: CloudFileSummary[];
  loading: boolean;
  error: string | null;
  loadFiles: () => Promise<void>;
  createFile: (name: string, document: PenDocument, source?: 'import' | 'manual_save') => Promise<string | null>;
  deleteFile: (id: string) => Promise<boolean>;
  reset: () => void;
}

export const useCloudFileStore = create<CloudFileState>((set, get) => ({
  files: [],
  loading: false,
  error: null,

  loadFiles: async () => {
    set({ loading: true, error: null });
    try {
      const files = await listCloudFiles();
      set({ files, loading: false });
    } catch (err) {
      set({
        loading: false,
        error: err instanceof Error ? err.message : 'Failed to load cloud files',
      });
    }
  },

  createFile: async (name, document, source = 'manual_save') => {
    set({ loading: true, error: null });
    try {
      const file = await createCloudFile({ name, document, source });
      set({ files: [file, ...get().files], loading: false });
      return file.id;
    } catch (err) {
      set({
        loading: false,
        error: err instanceof Error ? err.message : 'Failed to create cloud file',
      });
      return null;
    }
  },

  deleteFile: async (id) => {
    set({ error: null });
    try {
      await deleteCloudFile(id);
      set({ files: get().files.filter((file) => file.id !== id) });
      return true;
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to delete cloud file' });
      return false;
    }
  },

  reset: () => set({ files: [], loading: false, error: null }),
}));

