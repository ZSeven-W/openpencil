import { detectAllIssues, type Issue, type IssueCategory } from '@zseven-w/pen-ai-skills';
import { findNodeInTree } from '@zseven-w/pen-core';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import { openDocument, resolveDocPath } from '../document-manager';
import { getDocChildren } from '../utils/node-operations';

export interface DebugValidationReportParams {
  filePath?: string;
  pageId?: string;
  rootNodeId?: string;
  categories?: IssueCategory[];
  maxIssues?: number;
}

interface ReportJson {
  rootId: string;
  totalScanned: number;
  totalIssues: number;
  byCategory: Partial<Record<IssueCategory, number>>;
  issues: Issue[];
}

function countNodes(node: PenNode): number {
  let count = 1;
  if ('children' in node && node.children) {
    for (const c of node.children) count += countNodes(c);
  }
  return count;
}

export async function handleValidationReport(
  params: DebugValidationReportParams,
  docOverride?: PenDocument,
): Promise<string> {
  const doc = docOverride ?? (await openDocument(resolveDocPath(params.filePath)));
  const pageChildren = getDocChildren(doc, params.pageId);

  // Resolve root: explicit nodeId, else use a synthetic container over page children.
  let root: PenNode;
  if (params.rootNodeId) {
    const found = findNodeInTree(pageChildren, params.rootNodeId);
    if (!found) {
      const empty: ReportJson = {
        rootId: params.rootNodeId,
        totalScanned: 0,
        totalIssues: 0,
        byCategory: {},
        issues: [],
      };
      return JSON.stringify(empty, null, 2);
    }
    root = found;
  } else {
    // Wrap page children in a synthetic frame — detectors walk the subtree
    // from a PenNode, and this synthetic root never appears in the output.
    root = {
      id: '__synthetic_root__',
      type: 'frame',
      name: 'root',
      children: pageChildren,
    } as unknown as PenNode;
  }

  let issues = detectAllIssues(root, doc);

  if (params.categories && params.categories.length > 0) {
    const allowed = new Set(params.categories);
    issues = issues.filter((i) => allowed.has(i.category));
  }

  const cap = params.maxIssues ?? 200;
  if (issues.length > cap) issues = issues.slice(0, cap);

  const byCategory: Partial<Record<IssueCategory, number>> = {};
  for (const i of issues) {
    byCategory[i.category] = (byCategory[i.category] ?? 0) + 1;
  }

  const report: ReportJson = {
    rootId: params.rootNodeId ?? '__page__',
    totalScanned: countNodes(root),
    totalIssues: issues.length,
    byCategory,
    issues,
  };

  return JSON.stringify(report, null, 2);
}
