/**
 * Template instantiation — creates a new page from a template definition.
 *
 * Reuses existing UIKit utilities (deepCloneNode) and page store actions.
 */

import { nanoid } from 'nanoid'
import type { PenPage } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { getTemplateById, type TemplateDefinition } from './content-templates'

/**
 * Instantiate a template as a new page in the document.
 *
 * Creates a fresh PenNode tree from the template factory (unique IDs each time),
 * wraps it in a PenPage, and appends it to the document's pages.
 */
export function instantiateTemplate(templateId: string): string | null {
  const template = getTemplateById(templateId)
  if (!template) return null

  return instantiateFromDefinition(template)
}

/**
 * Instantiate directly from a TemplateDefinition (for programmatic use).
 */
export function instantiateFromDefinition(template: TemplateDefinition): string {
  const doc = useDocumentStore.getState().document
  useHistoryStore.getState().pushState(doc)

  // Create a fresh node tree with unique IDs
  const rootNode = template.create()

  const pageId = nanoid()
  const newPage: PenPage = {
    id: pageId,
    name: template.name,
    children: [rootNode],
  }

  // Append page to document via direct state update
  const pages = doc.pages ?? []
  useDocumentStore.setState({
    document: { ...doc, pages: [...pages, newPage] },
    isDirty: true,
  })

  // Navigate to the new page
  useCanvasStore.getState().setActivePageId(pageId)

  return pageId
}
