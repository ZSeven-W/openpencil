import type { PenDocument } from '@/types/pen';
import { parseAndPrepareImportedDocument } from './import-pen-document';

// ---------------------------------------------------------------------------
// Feature 检测
// ---------------------------------------------------------------------------

export function supportsFileSystemAccess(): boolean {
  return 'showSaveFilePicker' in window;
}

export function isElectron(): boolean {
  return !!window.electronAPI?.isElectron;
}

// ---------------------------------------------------------------------------
// File System Access API (Chrome / Edge)
// ---------------------------------------------------------------------------

/** Serialize 文档到 JSON 字符串。 Throws 失败。 */
function serializeDocument(doc: PenDocument): string {
  return JSON.stringify(doc);
}

/** Write 将 JSON 文件转换为 FileSystemFileHandle。 */
export async function writeToFileHandle(
  handle: FileSystemFileHandle,
  doc: PenDocument,
): Promise<void> {
  const json = serializeDocument(doc);
  const writable = await handle.createWritable();
  await writable.write(json);
  await writable.close();
}

/** 通过 Electron IPC 将 Write 文档保存到已知文件路径。 */
export async function writeToFilePath(filePath: string, doc: PenDocument): Promise<void> {
  const api = window.electronAPI;
  if (!api?.saveToPath) throw new Error('Electron saveToPath not available');
  const json = serializeDocument(doc);
  await api.saveToPath(filePath, json);
}

/** Show 本机保存文件选择器，写入并返回句柄+名称。 */
export async function saveDocumentAs(
  doc: PenDocument,
  suggestedName?: string,
): Promise<{ handle: FileSystemFileHandle; fileName: string } | null> {
  try {
    const handle: FileSystemFileHandle = await (
      window as unknown as {
        showSaveFilePicker: (opts: unknown) => Promise<FileSystemFileHandle>;
      }
    ).showSaveFilePicker({
      suggestedName: suggestedName || 'untitled.op',
      types: [
        {
          description: 'OpenPencil File',
          accept: { 'application/json': ['.op'] },
        },
      ],
    });
    await writeToFileHandle(handle, doc);
    return { handle, fileName: handle.name };
  } catch {
    // User 已取消或 API 错误
    return null;
  }
}

/** 通过本机选择器获取 Open 文件，返回 doc + 句柄。 */
export async function openDocumentFS(): Promise<{
  doc: PenDocument;
  fileName: string;
  handle: FileSystemFileHandle;
} | null> {
  try {
    const [handle]: FileSystemFileHandle[] = await (
      window as unknown as {
        showOpenFilePicker: (opts: unknown) => Promise<FileSystemFileHandle[]>;
      }
    ).showOpenFilePicker({
      types: [
        {
          description: 'OpenPencil File',
          accept: { 'application/json': ['.op', '.pen', '.json'] },
        },
      ],
    });
    const file = await handle.getFile();
    const text = await file.text();
    const prepared = parseAndPrepareImportedDocument(text, {
      fileName: file.name,
    });
    if (!prepared) throw new Error('Invalid PenDocument format');
    const { doc } = prepared;
    return { doc, fileName: file.name, handle };
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Fallback：下载/文件输入（Firefox、Safari）
// ---------------------------------------------------------------------------

/** Download 文档作为文件（浏览器下载）。 */
export function downloadDocument(doc: PenDocument, fileName: string): void {
  const json = JSON.stringify(doc);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.click();
  URL.revokeObjectURL(url);
}

/** Open 文件通过 <input type="file"> （后备）。 */
export function openDocument(): Promise<{
  doc: PenDocument;
  fileName: string;
} | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.op,.pen,.json';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      try {
        const text = await file.text();
        const prepared = parseAndPrepareImportedDocument(text, {
          fileName: file.name,
        });
        if (!prepared) throw new Error('Invalid PenDocument format');
        const { doc } = prepared;
        resolve({ doc, fileName: file.name });
      } catch {
        resolve(null);
      }
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}
