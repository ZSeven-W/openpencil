import { useDocumentStore } from '@/stores/document-store';
import {
  supportsFileSystemAccess,
  isElectron,
  writeToFileHandle,
  writeToFilePath,
  saveDocumentAs,
  downloadDocument,
} from '@/utils/file-operations';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';

export async function saveCurrentDocument(): Promise<boolean> {
  try {
    syncCanvasPositionsToStore();
  } catch (err) {
    console.error('[Save] syncCanvasPositionsToStore failed:', err);
  }

  const store = useDocumentStore.getState();
  const { document: doc, fileName: fn, fileHandle, filePath } = store;

  const isOpFile = fn ? /\.op$/i.test(fn) : false;
  const suggestedName = fn ? fn.replace(/\.(pen|op|json)$/i, '') + '.op' : 'untitled.op';

  try {
    if (isElectron() && filePath && isOpFile) {
      await writeToFilePath(filePath, doc);
      store.markClean();
      return true;
    }

    if (fileHandle && isOpFile) {
      try {
        await writeToFileHandle(fileHandle, doc);
        store.markClean();
        return true;
      } catch (err) {
        console.warn('[Save] File handle write failed, falling back:', err);
        useDocumentStore.setState({ fileHandle: null });
      }
    }

    if (isElectron()) {
      const savedPath = await window.electronAPI!.saveFile(JSON.stringify(doc), suggestedName);
      if (!savedPath) return false;
      useDocumentStore.setState({
        fileName: savedPath.split(/[/\\]/).pop() || suggestedName,
        filePath: savedPath,
        fileHandle: null,
        isDirty: false,
      });
      return true;
    }

    if (supportsFileSystemAccess()) {
      const result = await saveDocumentAs(doc, suggestedName);
      if (!result) return false;
      useDocumentStore.setState({
        fileName: result.fileName,
        fileHandle: result.handle,
        isDirty: false,
      });
      return true;
    }

    downloadDocument(doc, suggestedName);
    store.markClean();
    return true;
  } catch (err) {
    console.error('[Save] Failed to save document:', err);
    try {
      downloadDocument(doc, suggestedName);
      store.markClean();
      return true;
    } catch (dlErr) {
      console.error('[Save] Download fallback also failed:', dlErr);
      return false;
    }
  }
}
