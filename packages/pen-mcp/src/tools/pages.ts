import { openDocument, saveDocument, resolveDocPath } from '../document-manager';
import { cloneNodeWithNewIds } from '../utils/node-operations';
import { generateId } from '../utils/id';
import type { PenPage, PenNode } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// add_page
// ---------------------------------------------------------------------------

export interface AddPageParams {
  filePath?: string;
  name?: string;
  children?: Record<string, any>[];
}

export async function handleAddPage(
  params: AddPageParams,
): Promise<{ pageId: string; pageCount: number }> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  // Ensure pages array exists; migrate doc.children to first page if needed
  if (!doc.pages || doc.pages.length === 0) {
    doc.pages = [
      {
        id: generateId(),
        name: 'Page 1',
        children: doc.children ?? [],
      },
    ];
    doc.children = [];
  }

  const pageNum = doc.pages.length + 1;
  const newPage: PenPage = {
    id: generateId(),
    name: params.name ?? `Page ${pageNum}`,
    children: (params.children as PenNode[]) ?? [
      {
        id: generateId(),
        type: 'frame',
        name: 'Frame',
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        children: [],
      },
    ],
  };

  doc.pages.push(newPage);
  await saveDocument(filePath, doc);

  return { pageId: newPage.id, pageCount: doc.pages.length };
}

// ---------------------------------------------------------------------------
// remove_page
// ---------------------------------------------------------------------------

export interface RemovePageParams {
  filePath?: string;
  pageId: string;
}

export async function handleRemovePage(
  params: RemovePageParams,
): Promise<{ ok: true; pageCount: number }> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  if (!doc.pages || doc.pages.length <= 1) {
    throw new Error('Cannot remove the last page');
  }

  const idx = doc.pages.findIndex((p) => p.id === params.pageId);
  if (idx === -1) throw new Error(`Page not found: ${params.pageId}`);

  doc.pages.splice(idx, 1);
  await saveDocument(filePath, doc);

  return { ok: true, pageCount: doc.pages.length };
}

// ---------------------------------------------------------------------------
// rename_page
// ---------------------------------------------------------------------------

export interface RenamePageParams {
  filePath?: string;
  pageId: string;
  name: string;
}

export async function handleRenamePage(params: RenamePageParams): Promise<{ ok: true }> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  if (!doc.pages) throw new Error('Document has no pages');
  const page = doc.pages.find((p) => p.id === params.pageId);
  if (!page) throw new Error(`Page not found: ${params.pageId}`);

  page.name = params.name;
  await saveDocument(filePath, doc);

  return { ok: true };
}

// ---------------------------------------------------------------------------
// reorder_page
// ---------------------------------------------------------------------------

export interface ReorderPageParams {
  filePath?: string;
  pageId: string;
  index: number;
}

export async function handleReorderPage(params: ReorderPageParams): Promise<{ ok: true }> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  if (!doc.pages) throw new Error('Document has no pages');
  const idx = doc.pages.findIndex((p) => p.id === params.pageId);
  if (idx === -1) throw new Error(`Page not found: ${params.pageId}`);

  const newIdx = Math.max(0, Math.min(params.index, doc.pages.length - 1));
  const [moved] = doc.pages.splice(idx, 1);
  doc.pages.splice(newIdx, 0, moved);
  await saveDocument(filePath, doc);

  return { ok: true };
}

// ---------------------------------------------------------------------------
// duplicate_page
// ---------------------------------------------------------------------------

export interface DuplicatePageParams {
  filePath?: string;
  pageId: string;
  name?: string;
}

export async function handleDuplicatePage(
  params: DuplicatePageParams,
): Promise<{ pageId: string; pageCount: number }> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  if (!doc.pages) throw new Error('Document has no pages');
  const page = doc.pages.find((p) => p.id === params.pageId);
  if (!page) throw new Error(`Page not found: ${params.pageId}`);

  const newPageId = generateId();
  const newPage: PenPage = {
    id: newPageId,
    name: params.name ?? `${page.name} copy`,
    children: page.children.map((n) => cloneNodeWithNewIds(n, generateId)),
  };

  const idx = doc.pages.findIndex((p) => p.id === params.pageId);
  doc.pages.splice(idx + 1, 0, newPage);
  await saveDocument(filePath, doc);

  return { pageId: newPageId, pageCount: doc.pages.length };
}
