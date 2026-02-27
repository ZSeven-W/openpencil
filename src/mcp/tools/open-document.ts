import { resolve } from 'node:path'
import {
  openDocument,
  createEmptyDocument,
  saveDocument,
  fileExists,
} from '../document-manager'
import type { PenDocument } from '../../types/pen'

export interface OpenDocumentParams {
  filePath?: string
}

export interface OpenDocumentResult {
  filePath: string
  document: {
    version: string
    name?: string
    childCount: number
    pageCount: number
    pages?: { id: string; name: string; childCount: number }[]
    hasVariables: boolean
    hasThemes: boolean
  }
}

export async function handleOpenDocument(
  params: OpenDocumentParams,
): Promise<OpenDocumentResult> {
  let filePath: string
  let doc: PenDocument

  if (params.filePath) {
    filePath = resolve(params.filePath)
    const exists = await fileExists(filePath)
    if (exists) {
      doc = await openDocument(filePath)
    } else {
      // Create new file at specified path
      doc = createEmptyDocument()
      await saveDocument(filePath, doc)
    }
  } else {
    throw new Error(
      'filePath is required. Provide a path to an existing .op file or a new file to create.',
    )
  }

  const pages = doc.pages?.map((p) => ({
    id: p.id,
    name: p.name,
    childCount: p.children.length,
  }))
  const totalChildren = doc.pages
    ? doc.pages.reduce((sum, p) => sum + p.children.length, 0)
    : doc.children.length

  return {
    filePath,
    document: {
      version: doc.version,
      name: doc.name,
      childCount: totalChildren,
      pageCount: doc.pages?.length ?? 1,
      pages,
      hasVariables: !!doc.variables && Object.keys(doc.variables).length > 0,
      hasThemes: !!doc.themes && Object.keys(doc.themes).length > 0,
    },
  }
}
