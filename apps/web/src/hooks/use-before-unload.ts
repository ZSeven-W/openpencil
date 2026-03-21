import { useEffect } from 'react'
import { useDocumentStore } from '@/stores/document-store'

/**
 * Prevents accidental data loss by warning the user before closing the tab/window
 * when there are unsaved changes. In Electron, close confirmation is handled by
 * the main process via a native dialog, so this hook is skipped.
 */
export function useBeforeUnload() {
  const isDirty = useDocumentStore((s) => s.isDirty)

  useEffect(() => {
    // Electron handles close confirmation in the main process
    if (window.electronAPI) return
    if (!isDirty) return

    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault()
      e.returnValue = ''
    }

    window.addEventListener('beforeunload', handler)
    return () => window.removeEventListener('beforeunload', handler)
  }, [isDirty])
}
