import type { PenDocument, PenNode, PenPage } from '@zseven-w/pen-types';
import { generateId } from '@zseven-w/pen-core';

export interface PageManagerOptions {
  getDocument: () => PenDocument;
  setDocument: (doc: PenDocument) => void;
  onPageChange?: (pageId: string) => void;
}

/**
 * Manages multi-page state.
 * Extracted from apps/web/src/stores/document-store-pages.ts.
 */
export class PageManager {
  private activePageId: string | null = null;
  private getDocument: () => PenDocument;
  private setDocument: (doc: PenDocument) => void;
  private onPageChangeCb?: (pageId: string) => void;

  constructor(options: PageManagerOptions) {
    this.getDocument = options.getDocument;
    this.setDocument = options.setDocument;
    this.onPageChangeCb = options.onPageChange;
    // Initialize active page to the first page
    const doc = this.getDocument();
    this.activePageId = doc.pages?.[0]?.id ?? null;
  }

  getActivePage(): string {
    return this.activePageId ?? this.getDocument().pages?.[0]?.id ?? '';
  }

  setActivePage(pageId: string): void {
    this.activePageId = pageId;
    this.onPageChangeCb?.(pageId);
  }

  addPage(): string {
    const doc = this.getDocument();
    const pages = doc.pages ?? [];
    const pageNum = pages.length + 1;
    const newPageId = generateId();
    const newPage: PenPage = {
      id: newPageId,
      name: `Page ${pageNum}`,
      children: [
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
        } as PenNode,
      ],
    };
    this.setDocument({ ...doc, pages: [...pages, newPage] });
    this.setActivePage(newPageId);
    return newPageId;
  }

  removePage(pageId: string): void {
    const doc = this.getDocument();
    if (!doc.pages || doc.pages.length <= 1) return;
    const newPages = doc.pages.filter((p) => p.id !== pageId);
    this.setDocument({ ...doc, pages: newPages });
    if (this.activePageId === pageId) {
      this.setActivePage(newPages[0].id);
    }
  }

  renamePage(pageId: string, name: string): void {
    const doc = this.getDocument();
    if (!doc.pages) return;
    this.setDocument({
      ...doc,
      pages: doc.pages.map((p) => (p.id === pageId ? { ...p, name } : p)),
    });
  }

  reorderPage(pageId: string, direction: 'left' | 'right'): void {
    const doc = this.getDocument();
    if (!doc.pages) return;
    const idx = doc.pages.findIndex((p) => p.id === pageId);
    if (idx === -1) return;
    const newIdx = direction === 'left' ? idx - 1 : idx + 1;
    if (newIdx < 0 || newIdx >= doc.pages.length) return;
    const newPages = [...doc.pages];
    const [moved] = newPages.splice(idx, 1);
    newPages.splice(newIdx, 0, moved);
    this.setDocument({ ...doc, pages: newPages });
  }

  duplicatePage(pageId: string): string | null {
    const doc = this.getDocument();
    if (!doc.pages) return null;
    const page = doc.pages.find((p) => p.id === pageId);
    if (!page) return null;
    const newPageId = generateId();
    const cloneWithNewIds = (n: PenNode): PenNode => {
      const cloned = { ...n, id: generateId() } as PenNode;
      if ('children' in cloned && cloned.children) {
        cloned.children = cloned.children.map(cloneWithNewIds);
      }
      return cloned;
    };
    const newPage: PenPage = {
      id: newPageId,
      name: `${page.name} copy`,
      children: page.children.map(cloneWithNewIds),
    };
    const idx = doc.pages.findIndex((p) => p.id === pageId);
    const newPages = [...doc.pages];
    newPages.splice(idx + 1, 0, newPage);
    this.setDocument({ ...doc, pages: newPages });
    this.setActivePage(newPageId);
    return newPageId;
  }
}
