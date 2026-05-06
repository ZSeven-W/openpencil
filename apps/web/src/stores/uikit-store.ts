import { create } from 'zustand';
import type { UIKit, ComponentCategory } from '@/types/uikit';
import { getBuiltInKits } from '@/uikit/built-in-registry';
import { appStorage } from '@/utils/app-storage';

const STORAGE_KEY = 'openpencil-uikits';

interface PersistedState {
  importedKits: UIKit[];
  browserOpen?: boolean;
}

interface UIKitStoreState {
  /** All 加载套件（内置+进口） */
  kits: UIKit[];
  /** Whether 浏览器面板已打开 */
  browserOpen: boolean;
  /** Current 搜索查询 */
  searchQuery: string;
  /** Active 类别过滤器（空 = 全部） */
  activeCategory: ComponentCategory | null;
  /** Active 套件过滤器（空 = 全部） */
  activeKitId: string | null;

  toggleBrowser: () => void;
  setBrowserOpen: (open: boolean) => void;
  setSearchQuery: (query: string) => void;
  setActiveCategory: (category: ComponentCategory | null) => void;
  setActiveKitId: (kitId: string | null) => void;
  importKit: (kit: UIKit) => void;
  removeKit: (kitId: string) => void;
  persist: () => void;
  hydrate: () => void;
}

export const useUIKitStore = create<UIKitStoreState>((set, get) => ({
  kits: getBuiltInKits(),
  browserOpen: false,
  searchQuery: '',
  activeCategory: null,
  activeKitId: null,

  toggleBrowser: () => {
    const next = !get().browserOpen;
    set({ browserOpen: next });
    get().persist();
  },
  setBrowserOpen: (open) => {
    set({ browserOpen: open });
    get().persist();
  },
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setActiveCategory: (activeCategory) => set({ activeCategory }),
  setActiveKitId: (activeKitId) => set({ activeKitId }),

  importKit: (kit) => {
    set((s) => ({ kits: [...s.kits, kit] }));
    get().persist();
  },

  removeKit: (kitId) => {
    const { activeKitId } = get();
    set((s) => ({
      kits: s.kits.filter((k) => k.id !== kitId || k.builtIn),
      // Reset 过滤器（如果选择了已删除的套件）
      activeKitId: activeKitId === kitId ? null : activeKitId,
    }));
    get().persist();
  },

  persist: () => {
    try {
      const { kits, browserOpen } = get();
      const imported = kits.filter((k) => !k.builtIn);
      appStorage.setItem(STORAGE_KEY, JSON.stringify({ importedKits: imported, browserOpen }));
    } catch {
      // 忽略 — localStorage 可能已满
    }
  },

  hydrate: () => {
    try {
      const raw = appStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as Partial<PersistedState>;
      if (data.importedKits && Array.isArray(data.importedKits)) {
        const builtIn = getBuiltInKits();
        const builtInIds = new Set(builtIn.map((k) => k.id));
        // Filter 排除任何与内置 IDs 冲突的导入套件
        const imported = data.importedKits.filter((k) => !builtInIds.has(k.id));
        set({ kits: [...builtIn, ...imported] });
      }
      if (typeof data.browserOpen === 'boolean') set({ browserOpen: data.browserOpen });
    } catch {
      // 忽略
    }
  },
}));
