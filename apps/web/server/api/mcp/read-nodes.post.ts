import { defineEventHandler, readBody, createError } from 'h3';
import { getSyncDocument } from '../../utils/mcp-sync-state';
import type { PenDocument, PenNode, NodeSnapshot } from '@zseven-w/pen-types';
import { findNodeInTree, getActivePageChildren } from '@zseven-w/pen-core';
import { openDocument, LIVE_CANVAS_PATH, readNodeWithDepth } from '@zseven-w/pen-mcp';

async function resolveDocument(filePath?: string): Promise<PenDocument> {
  if (filePath && filePath !== LIVE_CANVAS_PATH) {
    return openDocument(filePath);
  }
  const { doc } = getSyncDocument();
  if (!doc) {
    throw createError({ statusCode: 404, statusMessage: 'No document loaded in editor' });
  }
  return doc;
}

interface ReadNodesBody {
  nodeIds?: string[];
  depth?: number;
  pageId?: string;
  filePath?: string;
  includeVariables?: boolean;
}

export default defineEventHandler(async (event) => {
  const body = await readBody<ReadNodesBody>(event);
  const doc = await resolveDocument(body?.filePath);

  const depth = body?.depth ?? -1;
  const pageChildren = getActivePageChildren(doc, body?.pageId ?? null);

  let nodes: NodeSnapshot[];

  if (body?.nodeIds && body.nodeIds.length > 0) {
    nodes = body.nodeIds
      .map((id) => findNodeInTree(pageChildren, id))
      .filter((n): n is PenNode => n !== undefined)
      .map((n) =>
        depth === -1
          ? (n as unknown as NodeSnapshot)
          : (readNodeWithDepth(n, depth) as unknown as NodeSnapshot),
      );
  } else {
    nodes = pageChildren.map((n) =>
      depth === -1
        ? (n as unknown as NodeSnapshot)
        : (readNodeWithDepth(n, depth) as unknown as NodeSnapshot),
    );
  }

  const result: Record<string, unknown> = { nodes };
  if (body?.includeVariables) {
    result.variables = doc.variables ?? {};
    result.themes = (doc as { themes?: unknown[] }).themes ?? [];
  }

  return result;
});
