import type { PenDocument } from '@/types/pen'
import { normalizePenDocument } from './normalize-pen-file'

// ---------------------------------------------------------------------------
// Feature detection
// ---------------------------------------------------------------------------

export function supportsFileSystemAccess(): boolean {
  return 'showSaveFilePicker' in window
}

// ---------------------------------------------------------------------------
// File System Access API (Chrome / Edge)
// ---------------------------------------------------------------------------

/** Write document JSON to a FileSystemFileHandle. */
export async function writeToFileHandle(
  handle: FileSystemFileHandle,
  doc: PenDocument,
): Promise<void> {
  const writable = await handle.createWritable()
  await writable.write(JSON.stringify(doc, null, 2))
  await writable.close()
}

/** Show native save-file picker, write, and return the handle + name. */
export async function saveDocumentAs(
  doc: PenDocument,
  suggestedName?: string,
): Promise<{ handle: FileSystemFileHandle; fileName: string } | null> {
  try {
    const handle: FileSystemFileHandle = await (
      window as unknown as {
        showSaveFilePicker: (opts: unknown) => Promise<FileSystemFileHandle>
      }
    ).showSaveFilePicker({
      suggestedName: suggestedName || 'untitled.op',
      types: [
        {
          description: 'OpenPencil File',
          accept: { 'application/json': ['.op'] },
        },
      ],
    })
    await writeToFileHandle(handle, doc)
    return { handle, fileName: handle.name }
  } catch {
    // User cancelled or API error
    return null
  }
}

/** Open file via native picker, return doc + handle. */
export async function openDocumentFS(): Promise<{
  doc: PenDocument
  fileName: string
  handle: FileSystemFileHandle
} | null> {
  try {
    const [handle]: FileSystemFileHandle[] = await (
      window as unknown as {
        showOpenFilePicker: (
          opts: unknown,
        ) => Promise<FileSystemFileHandle[]>
      }
    ).showOpenFilePicker({
      types: [
        {
          description: 'OpenPencil File',
          accept: { 'application/json': ['.op', '.pen', '.json'] },
        },
      ],
    })
    const file = await handle.getFile()
    const text = await file.text()
    const raw = JSON.parse(text) as PenDocument
    if (!raw.version || !Array.isArray(raw.children)) {
      throw new Error('Invalid PenDocument format')
    }
    const doc = normalizePenDocument(raw)
    return { doc, fileName: file.name, handle }
  } catch {
    return null
  }
}

// ---------------------------------------------------------------------------
// Fallback: download / file-input (Firefox, Safari)
// ---------------------------------------------------------------------------

/** Download document as a file (browser download). */
export function downloadDocument(doc: PenDocument, fileName: string): void {
  const json = JSON.stringify(doc, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = fileName
  a.click()
  URL.revokeObjectURL(url)
}

/** Open file via <input type="file"> (fallback). */
export function openDocument(): Promise<{
  doc: PenDocument
  fileName: string
} | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.op,.pen,.json'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) {
        resolve(null)
        return
      }
      try {
        const text = await file.text()
        const raw = JSON.parse(text) as PenDocument
        if (!raw.version || !Array.isArray(raw.children)) {
          throw new Error('Invalid PenDocument format')
        }
        const doc = normalizePenDocument(raw)
        resolve({ doc, fileName: file.name })
      } catch {
        resolve(null)
      }
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}
