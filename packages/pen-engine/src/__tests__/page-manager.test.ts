import { describe, it, expect, vi, beforeEach } from 'vitest';
import { PageManager } from '../core/page-manager';
import type { PenDocument } from '@zseven-w/pen-types';
import { createEmptyDocument, migrateToPages, ensureDocumentNodeIds } from '@zseven-w/pen-core';

describe('PageManager', () => {
  let pm: PageManager;
  let doc: PenDocument;
  let onChange: ReturnType<typeof vi.fn>;
  let onPageChange: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onChange = vi.fn();
    onPageChange = vi.fn();
    doc = ensureDocumentNodeIds(migrateToPages(createEmptyDocument()));
    pm = new PageManager({
      getDocument: () => doc,
      setDocument: (d) => {
        doc = d;
        onChange(d);
      },
      onPageChange,
    });
  });

  it('should return the first page as active by default', () => {
    const pageId = pm.getActivePage();
    expect(pageId).toBe(doc.pages![0].id);
  });

  it('addPage should create a new page', () => {
    const pageId = pm.addPage();
    expect(pageId).toBeTruthy();
    expect(doc.pages!.length).toBe(2);
    expect(onPageChange).toHaveBeenCalledWith(pageId);
  });

  it('removePage should remove a page (but not the last one)', () => {
    const pageId = pm.addPage();
    const originalFirst = doc.pages![0].id;
    pm.removePage(pageId);
    expect(doc.pages!.length).toBe(1);
    expect(doc.pages![0].id).toBe(originalFirst);
  });

  it('removePage should not remove the last page', () => {
    const onlyPageId = doc.pages![0].id;
    pm.removePage(onlyPageId);
    expect(doc.pages!.length).toBe(1);
  });

  it('renamePage should update page name', () => {
    const pageId = doc.pages![0].id;
    pm.renamePage(pageId, 'My Custom Page');
    expect(doc.pages![0].name).toBe('My Custom Page');
  });

  it('reorderPage should swap page positions', () => {
    const firstId = doc.pages![0].id;
    pm.addPage();
    const secondId = doc.pages![1].id;
    pm.reorderPage(firstId, 'right');
    expect(doc.pages![0].id).toBe(secondId);
    expect(doc.pages![1].id).toBe(firstId);
  });

  it('setActivePage should update active page and emit', () => {
    pm.addPage();
    const secondId = doc.pages![1].id;
    pm.setActivePage(secondId);
    expect(pm.getActivePage()).toBe(secondId);
    expect(onPageChange).toHaveBeenCalledWith(secondId);
  });

  it('duplicatePage should create a copy with new IDs', () => {
    const pageId = doc.pages![0].id;
    const newId = pm.duplicatePage(pageId);
    expect(newId).not.toBeNull();
    expect(doc.pages!.length).toBe(2);
    expect(doc.pages![1].name).toContain('copy');
  });
});
