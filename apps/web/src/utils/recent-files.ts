import { appStorage } from '@/utils/app-storage';

const STORAGE_KEY = 'openpencil-recent-files';
const MAX_ENTRIES = 10;

export interface RecentFile {
  fileName: string;
  filePath: string | null;
  lastOpened: number;
}

export function getRecentFiles(): RecentFile[] {
  try {
    const raw = appStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function syncToElectron(files: RecentFile[]): void {
  if (typeof window !== 'undefined' && window.electronAPI?.syncRecentFiles) {
    const forMenu = files
      .filter((f) => f.filePath)
      .map((f) => ({ fileName: f.fileName, filePath: f.filePath! }));
    window.electronAPI.syncRecentFiles(forMenu);
  }
}

export function addRecentFile(entry: Omit<RecentFile, 'lastOpened'>): void {
  const files = getRecentFiles();
  const filtered = files.filter(
    (f) => !(f.fileName === entry.fileName && f.filePath === entry.filePath),
  );
  const newEntry: RecentFile = { ...entry, lastOpened: Date.now() };
  const updated = [newEntry, ...filtered].slice(0, MAX_ENTRIES);
  appStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  syncToElectron(updated);
}

export function clearRecentFiles(): void {
  appStorage.removeItem(STORAGE_KEY);
  syncToElectron([]);
}

/**
 * Format a timestamp as a relative time string.
 * Returns an i18n key + interpolation params.
 */
export function relativeTime(timestamp: number): { key: string; params?: Record<string, number> } {
  const diff = Date.now() - timestamp;
  const minutes = Math.floor(diff / 60_000);
  const hours = Math.floor(diff / 3_600_000);
  const days = Math.floor(diff / 86_400_000);

  if (minutes < 1) return { key: 'fileMenu.justNow' };
  if (minutes < 60) return { key: 'fileMenu.minutesAgo', params: { count: minutes } };
  if (hours < 24) return { key: 'fileMenu.hoursAgo', params: { count: hours } };
  if (days < 2) return { key: 'fileMenu.yesterday' };
  return { key: 'fileMenu.daysAgo', params: { count: days } };
}
