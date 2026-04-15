import { describe, it, expect } from 'vitest';
import { detectAppendIntent } from '../append-intent-detector';
import type { PenDocument, FrameNode } from '@/types/pen';

function frame(id: string, name: string, width = 375, children: any[] = []): FrameNode {
  return {
    id,
    type: 'frame',
    name,
    x: 0,
    y: 0,
    width,
    height: 812,
    children,
  } as FrameNode;
}

function doc(children: any[], pageId?: string): PenDocument {
  if (pageId) {
    return {
      children: [],
      pages: [{ id: pageId, name: 'Page', children }],
    } as unknown as PenDocument;
  }
  return { children } as unknown as PenDocument;
}

describe('detectAppendIntent', () => {
  it('returns null when canvas is empty', () => {
    expect(detectAppendIntent('continue the workouts interface', doc([]), null)).toBeNull();
  });

  it('returns null when prompt has no append keyword', () => {
    const d = doc([frame('page-1', 'Page', 375, [frame('section-1', 'Hero')])]);
    expect(detectAppendIntent('generate a new landing page', d, null)).toBeNull();
  });

  it('returns null when prompt explicitly asks for a new page/screen', () => {
    const d = doc([frame('page-1', 'Page', 375, [frame('section-1', 'Hero')])]);
    expect(detectAppendIntent('add another screen for workouts', d, null)).toBeNull();
    expect(detectAppendIntent('也加一个新页面', d, null)).toBeNull();
  });

  it('detects "continue" with a non-empty page', () => {
    const content = frame('content-root', 'Page Content Root', 375, [frame('s-1', 'Hero')]);
    const page = frame('page-1', 'Page', 375, [frame('status-bar', 'Status bar'), content]);
    const d = doc([page]);
    const result = detectAppendIntent('Continue to generate the workouts interface', d, null);
    expect(result).not.toBeNull();
    expect(result!.targetParentId).toBe('content-root');
    expect(result!.isMobile).toBe(true);
    expect(result!.existingSectionLabels).toContain('Hero');
  });

  it('detects Chinese append keywords', () => {
    const page = frame('page-1', 'Page', 1200, [frame('hero', 'Hero')]);
    const d = doc([page]);
    const result = detectAppendIntent('再加一个功能 section', d, null);
    expect(result).not.toBeNull();
    expect(result!.isMobile).toBe(false);
  });

  it('falls back to the page frame when no content-root child exists', () => {
    const page = frame('page-1', 'Page', 375, [frame('hero', 'Hero')]);
    const d = doc([page]);
    const result = detectAppendIntent('continue', d, null);
    expect(result!.targetParentId).toBe('page-1');
  });

  it('honors the active page when pages[] is used', () => {
    const pageA = frame('page-a', 'Page', 375, [frame('hero', 'Hero')]);
    const pageB = frame('page-b', 'Page 2', 375, [frame('intro', 'Intro')]);
    const d = doc([pageA, pageB], 'page-b');
    const result = detectAppendIntent('continue', d, 'page-b');
    expect(result!.targetParentId).toBe('page-b');
    expect(result!.existingSectionLabels).toContain('Intro');
    expect(result!.existingSectionLabels).not.toContain('Hero');
  });

  it('returns null when the page has only a status bar child', () => {
    const page = frame('page-1', 'Page', 375, [frame('status-bar', 'Status Bar')]);
    const d = doc([page]);
    expect(detectAppendIntent('continue', d, null)).toBeNull();
  });

  it('excludes status bar from existingSectionLabels when real content also exists', () => {
    const page = frame('page-1', 'Page', 375, [
      frame('status-bar', 'Status Bar'),
      frame('hero', 'Hero'),
    ]);
    const d = doc([page]);
    const result = detectAppendIntent('continue', d, null);
    expect(result).not.toBeNull();
    expect(result!.existingSectionLabels).toContain('Hero');
    expect(result!.existingSectionLabels).not.toContain('Status Bar');
  });
});
