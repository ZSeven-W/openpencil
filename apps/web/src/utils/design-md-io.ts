import type { DesignMdSpec } from '@/types/design-md'
import { parseDesignMd, generateDesignMd } from './design-md-parser'
import { supportsFileSystemAccess } from './file-operations'

/** Import a .md design file via file picker. */
export async function importDesignMd(): Promise<DesignMdSpec | null> {
  if (supportsFileSystemAccess()) {
    try {
      const [handle]: FileSystemFileHandle[] = await (
        window as unknown as {
          showOpenFilePicker: (opts: unknown) => Promise<FileSystemFileHandle[]>
        }
      ).showOpenFilePicker({
        types: [
          {
            description: 'Design Markdown',
            accept: { 'text/markdown': ['.md'] },
          },
        ],
      })
      const file = await handle.getFile()
      const text = await file.text()
      return parseDesignMd(text)
    } catch {
      return null
    }
  }

  // Fallback: <input type="file">
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.md'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) { resolve(null); return }
      try {
        const text = await file.text()
        resolve(parseDesignMd(text))
      } catch {
        resolve(null)
      }
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}

/** Export a DesignMdSpec as a .md file. */
export async function exportDesignMd(
  spec: DesignMdSpec,
  fileName?: string,
): Promise<void> {
  const markdown = generateDesignMd(spec)
  const name = fileName ?? `${(spec.projectName ?? 'design').replace(/[^a-zA-Z0-9_-]/g, '_')}.md`

  if (supportsFileSystemAccess()) {
    try {
      const handle: FileSystemFileHandle = await (
        window as unknown as {
          showSaveFilePicker: (opts: unknown) => Promise<FileSystemFileHandle>
        }
      ).showSaveFilePicker({
        suggestedName: name,
        types: [
          {
            description: 'Design Markdown',
            accept: { 'text/markdown': ['.md'] },
          },
        ],
      })
      const writable = await handle.createWritable()
      await writable.write(markdown)
      await writable.close()
      return
    } catch {
      return
    }
  }

  // Fallback: browser download
  const blob = new Blob([markdown], { type: 'text/markdown' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = name
  a.click()
  URL.revokeObjectURL(url)
}
