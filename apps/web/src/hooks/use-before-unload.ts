import { useEffect } from 'react';
import { useDocumentStore } from '@/stores/document-store';

/**
 * Prevents
 * 当存在未保存的更改时，在关闭 tab/window 之前警告用户，从而避免意外数据丢失。 In
 * Electron，关闭确认由主进程通过本机对话框处理，因此跳过此钩子。
 */
export function useBeforeUnload() {
  const isDirty = useDocumentStore((s) => s.isDirty);

  useEffect(() => {
    // Electron 在主流程中处理关闭确认
    if (window.electronAPI) return;
    // Skip 处于开发模式，因此 Vite HMR 不会触发“Leave 页面？”对话
    if (import.meta.env.DEV) return;
    if (!isDirty) return;

    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = '';
    };

    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, [isDirty]);
}
