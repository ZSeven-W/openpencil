import type { PenDocument, PenNode } from '@/types/pen';
import { normalizePenDocument } from './normalize-pen-file';

interface ImportSourceOptions {
  fileName?: string | null;
  filePath?: string | null;
}

export interface PreparedImportedDocument {
  doc: PenDocument;
  appliedLegacyPenCompatibility: boolean;
}

export function isPenDocumentLike(raw: unknown): raw is PenDocument {
  if (!raw || typeof raw !== 'object') return false;
  const candidate = raw as Record<string, unknown>;
  return (
    typeof candidate.version === 'string' &&
    (Array.isArray(candidate.children) || Array.isArray(candidate.pages))
  );
}

export function prepareImportedDocument(
  raw: unknown,
  options: ImportSourceOptions = {},
): PreparedImportedDocument | null {
  if (!isPenDocumentLike(raw)) return null;

  const normalized = normalizePenDocument(raw);
  const appliedLegacyPenCompatibility = shouldApplyLegacyPenCompatibility(raw, options);

  return {
    doc: appliedLegacyPenCompatibility ? applyLegacyPenCompatibility(normalized) : normalized,
    appliedLegacyPenCompatibility,
  };
}

export function parseAndPrepareImportedDocument(
  text: string,
  options: ImportSourceOptions = {},
): PreparedImportedDocument | null {
  try {
    const raw = JSON.parse(text) as unknown;
    return prepareImportedDocument(raw, options);
  } catch {
    return null;
  }
}

function shouldApplyLegacyPenCompatibility(
  doc: PenDocument,
  options: ImportSourceOptions,
): boolean {
  if (!hasPenExtension(options.fileName, options.filePath)) return false;
  if (Array.isArray(doc.pages) && doc.pages.length > 0) return false;

  const topLevel = Array.isArray(doc.children) ? doc.children : [];
  if (topLevel.length === 0) return false;

  const pageLikeFrames = topLevel.filter(isLegacyPageFrame);
  if (pageLikeFrames.length === 0) return false;

  const legacyVersion = /^2(?:\.\d+)*$/i.test(doc.version);
  const hasLegacyShell = pageLikeFrames.some((frame) =>
    frame.children?.some(
      (child) =>
        child?.type === 'frame' &&
        typeof child.name === 'string' &&
        /^(content|qa|reference)$/i.test(child.name),
    ),
  );
  const pageLikeDominates = pageLikeFrames.length >= Math.max(2, Math.floor(topLevel.length / 2));

  return legacyVersion || hasLegacyShell || pageLikeDominates;
}

function hasPenExtension(fileName?: string | null, filePath?: string | null): boolean {
  return [fileName, filePath].some((value) => /\.pen$/i.test(value ?? ''));
}

function isLegacyPageFrame(node: PenNode): node is PenNode & { children?: PenNode[] } {
  const rawNode = node as unknown as Record<string, unknown>;
  return (
    node.type === 'frame' &&
    rawNode.clip === true &&
    typeof node.name === 'string' &&
    /^Page\s+\d+/i.test(node.name)
  );
}

function applyLegacyPenCompatibility(doc: PenDocument): PenDocument {
  return {
    ...doc,
    children: doc.children.map(reverseDescendantOrder),
    pages: doc.pages?.map((page) => ({
      ...page,
      children: page.children.map(reverseDescendantOrder),
    })),
  };
}

function reverseDescendantOrder(node: PenNode): PenNode {
  if (!('children' in node) || !Array.isArray(node.children) || node.children.length === 0) {
    return node;
  }

  return {
    ...node,
    children: node.children.map(reverseDescendantOrder).reverse(),
  } as PenNode;
}
