import type { PenNode, PenDocument, NodeSnapshot } from '@zseven-w/pen-types';
import { openDocument, resolveDocPath } from '../document-manager';
import { findNodeInTree, getDocChildren, readNodeWithDepth } from '../utils/node-operations';

export interface ReadNodesParams {
  nodeIds?: string[];
  depth?: number;
  pageId?: string;
  filePath?: string;
  includeVariables?: boolean;
}

export interface ReadNodesResult {
  nodes: NodeSnapshot[];
  variables?: Record<string, unknown>;
  themes?: unknown[];
}

function toNodeSnapshot(node: PenNode, depth: number): NodeSnapshot {
  return readNodeWithDepth(node, depth) as unknown as NodeSnapshot;
}

export async function handleReadNodes(
  params: ReadNodesParams,
  docOverride?: PenDocument,
): Promise<ReadNodesResult> {
  const depth = params.depth ?? -1;
  const doc = docOverride ?? (await openDocument(resolveDocPath(params.filePath)));
  const pageChildren = getDocChildren(doc, params.pageId);

  let nodes: NodeSnapshot[];

  if (params.nodeIds && params.nodeIds.length > 0) {
    nodes = params.nodeIds
      .map((id) => findNodeInTree(pageChildren, id))
      .filter((n): n is PenNode => n !== undefined)
      .map((n) => (depth === -1 ? (n as unknown as NodeSnapshot) : toNodeSnapshot(n, depth)));
  } else {
    nodes = pageChildren.map((n) =>
      depth === -1 ? (n as unknown as NodeSnapshot) : toNodeSnapshot(n, depth),
    );
  }

  const result: ReadNodesResult = { nodes };

  if (params.includeVariables) {
    result.variables = doc.variables ?? {};
    result.themes = (doc as { themes?: unknown[] }).themes ?? [];
  }

  return result;
}
