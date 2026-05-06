import { create } from 'zustand';
import type { DesignMdSpec } from '@/types/design-md';
import { appStorage } from '@/utils/app-storage';

const STORAGE_PREFIX = 'openpencil-design-md:';
const CURRENT_KEY_STORAGE = 'openpencil-design-md-current-key';

/** Derive 来自文件标识符的存储密钥。 Returns 无标题文档为 null。 */
function fileKey(fileName: string | null, filePath: string | null): string | null {
  return filePath ?? fileName ?? null;
}

interface DesignMdStoreState {
  /** Current design.md 规格 */
  designMd: DesignMdSpec | undefined;
  /** Current 用于持久化的文件密钥（null = 无标题，跳过持久化） */
  _fileKey: string | null;

  setDesignMd: (spec: DesignMdSpec | undefined) => void;
  /** Sync 存储到文档 — 恢复持久的 designMd 或清除（如果没有）。 */
  syncToDocument: (fileName: string | null, filePath: string | null) => void;
  /** 新文档上的 Called — 清除 designMd。 */
  clearForNewDocument: () => void;
  hydrate: () => void;
}

export const useDesignMdStore = create<DesignMdStoreState>((set, get) => ({
  designMd: undefined,
  _fileKey: null,

  setDesignMd: (spec) => {
    set({ designMd: spec });
    const key = get()._fileKey;
    if (!key) return; // 无标题 — 跳过持久性
    try {
      if (spec) {
        appStorage.setItem(STORAGE_PREFIX + key, JSON.stringify(spec));
      } else {
        appStorage.removeItem(STORAGE_PREFIX + key);
      }
    } catch {
      /* 忽略 */
    }
  },

  syncToDocument: (fileName, filePath) => {
    const key = fileKey(fileName, filePath);
    set({ _fileKey: key });

    if (!key) {
      set({ designMd: undefined });
      return;
    }

    // Restore 为此文件保留了 designMd
    try {
      const raw = appStorage.getItem(STORAGE_PREFIX + key);
      if (raw) {
        const data = JSON.parse(raw) as DesignMdSpec;
        if (data && typeof data === 'object' && typeof data.raw === 'string') {
          set({ designMd: data });
          return;
        }
      }
    } catch {
      /* 忽略 */
    }

    set({ designMd: undefined });
  },

  clearForNewDocument: () => {
    set({ designMd: undefined, _fileKey: null });
  },

  hydrate: () => {
    try {
      const lastKey = appStorage.getItem(CURRENT_KEY_STORAGE);
      if (!lastKey) return;
      set({ _fileKey: lastKey });
      const raw = appStorage.getItem(STORAGE_PREFIX + lastKey);
      if (!raw) return;
      const data = JSON.parse(raw) as DesignMdSpec;
      if (data && typeof data === 'object' && typeof data.raw === 'string') {
        set({ designMd: data });
      }
    } catch {
      /* 忽略 */
    }
  },
}));

// Persist 状态更改时的当前文件密钥
let _prevFileKey: string | null = null;
useDesignMdStore.subscribe((state) => {
  if (state._fileKey !== _prevFileKey) {
    _prevFileKey = state._fileKey;
    try {
      if (state._fileKey) {
        appStorage.setItem(CURRENT_KEY_STORAGE, state._fileKey);
      } else {
        appStorage.removeItem(CURRENT_KEY_STORAGE);
      }
    } catch {
      /* 忽略 */
    }
  }
});
