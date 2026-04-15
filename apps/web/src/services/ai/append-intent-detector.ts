import type { PenDocument, PenNode, FrameNode } from '@/types/pen';
import type { AppendContext } from './ai-types';

const APPEND_EN =
  /\b(continue|continuing|append|also add|add (?:another|more|a new)\s+section|one more|next section|add to the)\b/i;
const APPEND_CJK =
  /(继续|接着|再加|再加一个|再添加|再生成|再来一个|补充|追加|加一个.{0,6}(区块|栏|模块|section|段))/;

const NEW_SCREEN_EN =
  /\b(new (?:page|screen|design|mockup)|another (?:page|screen|design|mockup)|from scratch|brand new)\b/i;
const NEW_SCREEN_CJK = /(新页面|新屏|新设计|从零|全新|另起|另外一页)/;

const STATUS_BAR_RE = /(status[\s_-]*bar|system[\s_-]*chrome|状态栏|系统栏)/i;

function isFrame(node: PenNode): node is FrameNode {
  return node.type === 'frame';
}

function isStatusBarLike(frame: FrameNode): boolean {
  return STATUS_BAR_RE.test(`${frame.name ?? ''} ${frame.id ?? ''}`);
}

function pickContentRoot(page: FrameNode): { target: FrameNode; sectionLabels: string[] } {
  const children = (
    'children' in page && Array.isArray(page.children) ? page.children : []
  ) as PenNode[];
  const frames = children.filter(isFrame);
  const contentFrames = frames.filter((f) => !isStatusBarLike(f));

  const CONTENT_NAME = /\b(content|main|body|root)\b/i;
  const contentCandidate = contentFrames.find((f) => CONTENT_NAME.test(f.name ?? ''));
  if (contentCandidate) {
    const grand = (
      'children' in contentCandidate && Array.isArray(contentCandidate.children)
        ? contentCandidate.children
        : []
    ) as PenNode[];
    return {
      target: contentCandidate,
      sectionLabels: grand
        .filter(isFrame)
        .filter((f) => !isStatusBarLike(f))
        .map((c) => c.name ?? c.id),
    };
  }

  return {
    target: page,
    sectionLabels: contentFrames.map((c) => c.name ?? c.id),
  };
}

function pickActivePageFrame(doc: PenDocument, activePageId: string | null): FrameNode | null {
  const pages = (doc as unknown as { pages?: Array<{ id: string; children: PenNode[] }> }).pages;
  if (pages && pages.length > 0) {
    // Each page entry has an id and children (the top-level frames on that page).
    // If activePageId matches a frame inside any page's children, use that frame directly.
    if (activePageId) {
      for (const page of pages) {
        const matchingFrame = page.children.find((n) => isFrame(n) && n.id === activePageId);
        if (matchingFrame && isFrame(matchingFrame)) return matchingFrame;
      }
    }
    // Fall back: use active page entry's first frame
    const active = (activePageId && pages.find((p) => p.id === activePageId)) || pages[0];
    const firstFrame = active.children.find(isFrame);
    if (firstFrame) return firstFrame;
    return null;
  }
  // No pages array: look among top-level children frames
  const topFrames = (doc.children ?? []).filter(isFrame);
  if (activePageId) {
    const match = topFrames.find((f) => f.id === activePageId);
    if (match) return match;
  }
  return topFrames[0] ?? null;
}

export function detectAppendIntent(
  prompt: string,
  doc: PenDocument,
  activePageId: string | null,
): AppendContext | null {
  if (!prompt || prompt.trim().length === 0) return null;

  const hasAppendKeyword = APPEND_EN.test(prompt) || APPEND_CJK.test(prompt);
  if (!hasAppendKeyword) return null;

  if (NEW_SCREEN_EN.test(prompt) || NEW_SCREEN_CJK.test(prompt)) return null;

  const pageFrame = pickActivePageFrame(doc, activePageId);
  if (!pageFrame) return null;

  const pageHasContent =
    'children' in pageFrame &&
    Array.isArray(pageFrame.children) &&
    pageFrame.children.some((c) => isFrame(c) && !isStatusBarLike(c));
  if (!pageHasContent) return null;

  const { target, sectionLabels } = pickContentRoot(pageFrame);
  const width = typeof pageFrame.width === 'number' ? pageFrame.width : 375;

  return {
    targetParentId: target.id,
    targetWidth: typeof target.width === 'number' ? target.width : width,
    existingSectionLabels: sectionLabels,
    isMobile: width <= 480,
  };
}
