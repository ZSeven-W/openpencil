/**
 * Cross-环境存储抽象
 *
 * 。 In Electron Nitro 服务器每次启动都会在随机端口上启动，
 * 这会更改源并擦除
 * `localStorage`。 This 模块提供同步 `getItem` / `setItem` / `removeItem` API： -
 *
 * **Electron**：从启动时通过 IPC 从 JSON 首选项文件预加载的内存缓存中读取。 Writes
 * 立即更新缓存（同步）并异步保存到磁盘。 - **Web**：直接委托给 `localStorage`。 Call
 * `initAppStor
 * age()` 在应用程序启动时（在任何商店水合之前）一次，然后 `await` 结果。 In 网络模式它立即解决。
 *
 *
 *
 */

/** In - Electron 模式的内存缓存。 */
let cache: Record<string, string> | null = null;
let initPromise: Promise<void> | null = null;

/** Whether 我们正在 Electron 中运行，并且 IPC 桥可用。 */
function isElectron(): boolean {
  return typeof window !== 'undefined' && !!window.electronAPI?.getPreferences;
}

/**
 * Initialise
 * 存储层。 Must 在任何存储水合作用之前被调用（并等待），以便填充缓存。 Idempotent — 多个调用返回相同的承诺。
 *
 */
export async function initAppStorage(): Promise<void> {
  if (typeof window === 'undefined') return;
  if (!isElectron()) return;
  if (initPromise) return initPromise;
  initPromise = (async () => {
    try {
      const prefs: Record<string, string> = await window.electronAPI!.getPreferences();
      cache = prefs ?? {};
    } catch {
      cache = {};
    }
  })();
  return initPromise;
}

/** Synchronous get — 从缓存 (Electron) 或 localStorage (web) 读取。 */
export function getItem(key: string): string | null {
  if (cache !== null) {
    return cache[key] ?? null;
  }
  if (typeof window === 'undefined') return null;
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Synchronous set — 更新缓存+触发 Electron 中的异步 IPC 写入。 */
export function setItem(key: string, value: string): void {
  if (cache !== null) {
    cache[key] = value;
    window.electronAPI?.setPreference(key, value)?.catch(() => {});
    return;
  }
  try {
    localStorage.setItem(key, value);
  } catch {
    // 超出配额或私人模式
  }
}

/** Synchronous 删除 — 更新缓存+触发 Electron 中的异步 IPC 写入。 */
export function removeItem(key: string): void {
  if (cache !== null) {
    delete cache[key];
    window.electronAPI?.removePreference(key)?.catch(() => {});
    return;
  }
  try {
    localStorage.removeItem(key);
  } catch {
    // 忽略
  }
}

/**
 * Convenience 重新导出，以便商店可以执行以下操作：
 *   import { appStorage } from '@/utils/app-storage'
 * appStorage.getItem(...)
 */
export const appStorage = { getItem, setItem, removeItem };
