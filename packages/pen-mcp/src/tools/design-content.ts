import { openDocument, saveDocument, resolveDocPath } from '../document-manager';
import {
  findNodeInTree,
  flattenNodes,
  getDocChildren,
  setDocChildren,
  readNodeWithDepth,
} from '../utils/node-operations';
import { generateId } from '../utils/id';
import { sanitizeObject } from '../utils/sanitize';
import { getMcpHooks } from '../hooks';
import type { PenNode } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DesignContentParams {
  filePath?: string;
  sectionId: string;
  children: Record<string, unknown>[];
  postProcess?: boolean;
  canvasWidth?: number;
  pageId?: string;
}

interface DesignContentResult {
  sectionId: string;
  insertedCount: number;
  totalNodeCount: number;
  warnings: string[];
  snapshot: Record<string, unknown>;
  postProcessed: boolean;
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

export async function handleDesignContent(
  params: DesignContentParams,
): Promise<DesignContentResult> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);
  const pageId = params.pageId;
  const canvasWidth = params.canvasWidth ?? 1200;

  // Find the section frame
  const allChildren = getDocChildren(doc, pageId);
  const section = findNodeInTree(allChildren, params.sectionId);
  if (!section) {
    throw new Error(`Section not found: ${params.sectionId}`);
  }
  if (section.type !== 'frame') {
    throw new Error(`Section must be a frame node, got: ${section.type}`);
  }

  // Sanitize and assign IDs to incoming children
  const warnings: string[] = [];
  const sanitized = params.children.map((child) => sanitizeObject(child));
  const nodes: PenNode[] = sanitized.map((child) => assignIds(child, warnings));

  // Insert nodes as children of the section
  if (!section.children) {
    section.children = [];
  }
  section.children.push(...nodes);

  // Post-processing
  let postProcessed = false;
  if (params.postProcess !== false) {
    const hooks = getMcpHooks();
    hooks.resolveTreeRoles?.([section], canvasWidth);
    hooks.resolveTreePostPass?.([section]);

    const flat = flattenNodes([section]);
    for (const node of flat) {
      if (node.type === 'path') hooks.applyIconPathResolution?.([node]);
      if (node.type === 'text') hooks.applyNoEmojiIconHeuristic?.([node]);
    }

    hooks.ensureUniqueNodeIds?.([section]);
    hooks.sanitizeLayoutChildPositions?.([section]);
    hooks.sanitizeScreenFrameBounds?.([section]);

    postProcessed = true;
  }

  // Persist
  setDocChildren(doc, allChildren, pageId);
  await saveDocument(filePath, doc);

  // Build response with depth-limited snapshot
  const insertedCount = countNodes(nodes);
  const totalNodeCount = countNodes(section.children);
  const snapshot = readNodeWithDepth(section, 2);

  return {
    sectionId: params.sectionId,
    insertedCount,
    totalNodeCount,
    warnings,
    snapshot,
    postProcessed,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Recursively assign generated IDs to a node tree, collecting warnings. */
function assignIds(data: Record<string, unknown>, warnings: string[]): PenNode {
  const node = { ...data, id: (data.id as string) || generateId() } as unknown as PenNode;

  // Validate required fields
  const nodeRec = node as unknown as Record<string, unknown>;
  if (!nodeRec.type) {
    warnings.push(
      `Node "${(nodeRec.name as string) ?? (nodeRec.id as string)}" missing type — defaulting to "frame"`,
    );
    nodeRec.type = 'frame';
  }

  // Check for common issues
  if (node.type === 'text') {
    const tNode = node as import('@zseven-w/pen-types').TextNode;
    const content = typeof tNode.content === 'string' ? tNode.content : '';
    if (content.length > 15 && !tNode.textGrowth) {
      warnings.push(
        `Text "${content.slice(0, 20)}..." is >15 chars without textGrowth="fixed-width" — may overflow`,
      );
    }
    if (typeof tNode.height === 'number') {
      warnings.push(
        `Text "${node.name ?? node.id}" has explicit pixel height — will be removed by post-processing`,
      );
    }
  }

  if (node.type === 'frame') {
    const fNode = node as import('@zseven-w/pen-types').FrameNode;
    if (typeof fNode.cornerRadius === 'number' && fNode.cornerRadius > 0) {
      const hasImageChild =
        Array.isArray(fNode.children) && fNode.children.some((c) => c.type === 'image');
      if (hasImageChild && !fNode.clipContent) {
        warnings.push(
          `Frame "${node.name ?? node.id}" has cornerRadius + image child but no clipContent — will be auto-added`,
        );
      }
    }
  }

  // Recurse into children
  if ('children' in node && Array.isArray(node.children)) {
    const container = node as PenNode & import('@zseven-w/pen-types').ContainerProps;
    container.children = container.children!.map((child) =>
      assignIds(child as unknown as Record<string, unknown>, warnings),
    );
  }

  return node;
}

function countNodes(nodes: PenNode[]): number {
  let count = 0;
  for (const node of nodes) {
    count++;
    if ('children' in node && node.children) {
      count += countNodes(node.children);
    }
  }
  return count;
}
