import { nanoid } from 'nanoid'
import type { PenDocument, PenNode } from '@/types/pen'
import type { UIKit } from '@/types/uikit'
import { normalizePenDocument } from '@/utils/normalize-pen-file'
import {
  supportsFileSystemAccess,
  saveDocumentAs,
  downloadDocument,
} from '@/utils/file-operations'
import { extractComponentsFromDocument } from './kit-utils'

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/**
 * Import a .pen file as a UIKit via the native file picker.
 * Returns null if the user cancels or the file has no reusable components.
 */
export async function importKitFromFile(): Promise<UIKit | null> {
  const doc = await pickAndParsePenFile()
  if (!doc) return null

  const components = extractComponentsFromDocument(doc)
  if (components.length === 0) return null

  return {
    id: nanoid(),
    name: doc.name ?? 'Imported Kit',
    version: doc.version,
    builtIn: false,
    document: doc,
    components,
  }
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/**
 * Export reusable components from the current document as a .pen kit file.
 * If componentIds is empty, exports all reusable components.
 */
export async function exportKit(
  sourceDoc: PenDocument,
  componentIds: string[],
  kitName: string,
): Promise<boolean> {
  const reusableNodes = collectReusableNodes(sourceDoc.children)
  const selected =
    componentIds.length > 0
      ? reusableNodes.filter((n) => componentIds.includes(n.id))
      : reusableNodes

  if (selected.length === 0) return false

  const kitDoc: PenDocument = {
    version: '1.0.0',
    name: kitName,
    children: selected.map((n) => structuredClone(n)),
  }

  // Copy referenced variables
  if (sourceDoc.variables) {
    const refs = collectAllVariableRefs(selected)
    const vars: Record<string, unknown> = {}
    for (const ref of refs) {
      const name = ref.startsWith('$') ? ref.slice(1) : ref
      if (sourceDoc.variables[name]) {
        vars[name] = structuredClone(sourceDoc.variables[name])
      }
    }
    if (Object.keys(vars).length > 0) {
      kitDoc.variables = vars as PenDocument['variables']
    }
  }

  // Copy themes if variables were included
  if (kitDoc.variables && sourceDoc.themes) {
    kitDoc.themes = structuredClone(sourceDoc.themes)
  }

  const fileName = `${kitName.replace(/[^a-zA-Z0-9-_ ]/g, '').trim() || 'kit'}.op`

  if (supportsFileSystemAccess()) {
    const result = await saveDocumentAs(kitDoc, fileName)
    return result !== null
  }
  downloadDocument(kitDoc, fileName)
  return true
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async function pickAndParsePenFile(): Promise<PenDocument | null> {
  if (supportsFileSystemAccess()) {
    return pickFileSystemAccess()
  }
  return pickFallback()
}

async function pickFileSystemAccess(): Promise<PenDocument | null> {
  try {
    const [handle] = await (
      window as unknown as {
        showOpenFilePicker: (opts: unknown) => Promise<FileSystemFileHandle[]>
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
    return parsePenJson(text)
  } catch {
    return null
  }
}

function pickFallback(): Promise<PenDocument | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.op,.pen,.json'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) { resolve(null); return }
      try {
        const text = await file.text()
        resolve(parsePenJson(text))
      } catch {
        resolve(null)
      }
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}

function parsePenJson(text: string): PenDocument | null {
  try {
    const raw = JSON.parse(text) as PenDocument
    if (!raw.version || !Array.isArray(raw.children)) return null
    return normalizePenDocument(raw)
  } catch {
    return null
  }
}

function collectReusableNodes(nodes: PenNode[]): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
    if ('reusable' in node && node.reusable) {
      result.push(node)
    }
    if ('children' in node && node.children) {
      result.push(...collectReusableNodes(node.children))
    }
  }
  return result
}

function isVarRef(v: unknown): v is string {
  return typeof v === 'string' && v.startsWith('$')
}

function collectAllVariableRefs(nodes: PenNode[]): Set<string> {
  const refs = new Set<string>()
  for (const node of nodes) {
    collectNodeRefs(node, refs)
  }
  return refs
}

function collectNodeRefs(node: PenNode, refs: Set<string>): void {
  if (isVarRef(node.opacity)) refs.add(node.opacity)
  if (isVarRef(node.enabled)) refs.add(node.enabled as string)
  if ('gap' in node && isVarRef(node.gap)) refs.add(node.gap as string)
  if ('padding' in node && isVarRef(node.padding)) refs.add(node.padding as string)
  if ('fill' in node && Array.isArray(node.fill)) {
    for (const f of node.fill) {
      if (f.type === 'solid' && isVarRef(f.color)) refs.add(f.color)
    }
  }
  if ('children' in node && node.children) {
    for (const child of node.children) collectNodeRefs(child, refs)
  }
}
