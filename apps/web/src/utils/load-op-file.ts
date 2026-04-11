// apps/web/src/utils/load-op-file.ts
//
// Standalone helper to load a .op file from an absolute path via the
// Electron readFile IPC. Lives in its own module (NOT inside
// file-operations.ts) to avoid a cycle: document-store.ts imports
// writeToFileHandle/etc. from file-operations.ts, so file-operations.ts
// cannot import useDocumentStore without creating a cycle. This file
// imports document-store but is NOT imported by document-store or
// file-operations, so the dependency graph stays acyclic.
//
// Used by:
//   - apps/web/src/hooks/use-electron-menu.ts (file association open events)
//   - apps/web/src/components/panels/git-panel/git-panel-tracked-picker.tsx
//     (the [跟踪并打开] button — Phase 4b)
//   - apps/web/src/stores/git-store.ts acknowledgeAutoBindAndOpen action
//     (the auto-bind banner [打开] button — Phase 4b)

import { useDocumentStore } from '@/stores/document-store';
import { normalizePenDocument } from '@/utils/normalize-pen-file';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';

/**
 * Load a .op file from an absolute path via the Electron readFile IPC,
 * parse + normalize it, and dispatch into useDocumentStore. Returns true
 * on success, false on any failure (no throw — failures are silent
 * because the caller is usually a UI button that should not crash).
 */
export async function loadOpFileFromPath(filePath: string): Promise<boolean> {
  const api = typeof window !== 'undefined' ? window.electronAPI : undefined;
  if (!api?.readFile) return false;
  try {
    const result = await api.readFile(filePath);
    if (!result) return false;
    const raw = JSON.parse(result.content);
    if (!raw.version || (!Array.isArray(raw.children) && !Array.isArray(raw.pages))) {
      return false;
    }
    const doc = normalizePenDocument(raw);
    const name = filePath.split(/[/\\]/).pop() || 'untitled.op';
    useDocumentStore.getState().loadDocument(doc, name, null, filePath);
    // zoomToFitContent is dispatched on next frame so React has time
    // to commit the document update before the canvas reads it.
    requestAnimationFrame(() => zoomToFitContent());
    return true;
  } catch {
    return false;
  }
}
