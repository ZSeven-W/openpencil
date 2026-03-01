import { readFile, writeFile, access } from 'node:fs/promises'
import { constants } from 'node:fs'
import type { PenDocument } from '../types/pen'

const cache = new Map<string, { doc: PenDocument; mtime: number }>()

/** Validate that a parsed object looks like a PenDocument. */
function validate(doc: unknown): doc is PenDocument {
  if (!doc || typeof doc !== 'object') return false
  const d = doc as Record<string, unknown>
  // Accept docs with children array or pages array
  return typeof d.version === 'string' && (Array.isArray(d.children) || Array.isArray(d.pages))
}

/** Read and parse a .op / .pen file, returning a PenDocument. Uses cache. */
export async function openDocument(filePath: string): Promise<PenDocument> {
  const cached = cache.get(filePath)
  if (cached) return cached.doc

  await access(filePath, constants.R_OK)
  const text = await readFile(filePath, 'utf-8')
  const raw = JSON.parse(text)
  if (!validate(raw)) {
    throw new Error(`Invalid document format: ${filePath}`)
  }
  cache.set(filePath, { doc: raw, mtime: Date.now() })
  return raw
}

/** Create a new empty document (not saved to disk yet). */
export function createEmptyDocument(): PenDocument {
  return {
    version: '1.0.0',
    children: [],
  }
}

/** Write a PenDocument to disk and update cache. */
export async function saveDocument(
  filePath: string,
  doc: PenDocument,
): Promise<void> {
  const json = JSON.stringify(doc, null, 2)
  await writeFile(filePath, json, 'utf-8')
  cache.set(filePath, { doc, mtime: Date.now() })
}

/** Get document from cache (for tools that operate on the active doc). */
export function getCachedDocument(
  filePath: string,
): PenDocument | undefined {
  return cache.get(filePath)?.doc
}

/** Update the cached document in-memory (call saveDocument to persist). */
export function setCachedDocument(
  filePath: string,
  doc: PenDocument,
): void {
  cache.set(filePath, { doc, mtime: Date.now() })
}

/** Check if a file exists. */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, constants.F_OK)
    return true
  } catch {
    return false
  }
}

/** Invalidate cache for a file. */
export function invalidateCache(filePath: string): void {
  cache.delete(filePath)
}
