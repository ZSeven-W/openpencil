import { openDocument, saveDocument, resolveDocPath, getSyncUrl } from '../document-manager';
import {
  findNodeInTree,
  insertNodeInTree,
  updateNodeInTree,
  removeNodeFromTree,
  cloneNodeWithNewIds,
  flattenNodes,
  getDocChildren,
  setDocChildren,
} from '../utils/node-operations';
import { generateId } from '../utils/id';
import { sanitizeObject } from '../utils/sanitize';
import { getMcpHooks } from '../hooks';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';

export interface BatchDesignParams {
  filePath?: string;
  operations: string;
  postProcess?: boolean;
  canvasWidth?: number;
  pageId?: string;
}

interface OpResult {
  binding: string;
  nodeId: string;
}

/**
 * Parse and execute the batch_design operations DSL.
 *
 * Supported operations (one per line):
 *   binding=I(parent, { ...nodeData })      — Insert
 *   binding=C(nodeId, parent, { ...data })   — Copy
 *   U(path, { ...updates })                 — Update
 *   binding=R(path, { ...nodeData })         — Replace
 *   M(nodeId, parent, index?)               — Move
 *   D(nodeId)                               — Delete
 */
export async function handleBatchDesign(params: BatchDesignParams): Promise<{
  results: OpResult[];
  nodeCount: number;
  postProcessed?: boolean;
  errors?: Array<{ line: string; error: string }>;
}> {
  const filePath = resolveDocPath(params.filePath);
  let doc = await openDocument(filePath);
  doc = structuredClone(doc);

  const pageId = params.pageId;
  const bindings = new Map<string, string>();
  const results: OpResult[] = [];
  const errors: Array<{ line: string; error: string }> = [];
  const lines = splitOperations(params.operations);

  for (const line of lines) {
    try {
      await executeLine(line, doc, bindings, results, pageId);
    } catch (err) {
      // Best-effort: don't abort the entire batch on a single bad operation.
      // Collect errors so the agent can see what failed and retry selectively.
      const preview = line.length > 200 ? `${line.slice(0, 200)}...` : line;
      errors.push({
        line: preview,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  // --- Post-processing ---
  let postProcessed = false;
  if (params.postProcess) {
    const canvasWidth = params.canvasWidth ?? 1200;
    const children = getDocChildren(doc, pageId);

    // Find root nodes that were inserted (first binding is typically the root)
    const insertedIds = new Set(results.map((r) => r.nodeId));
    const rootTargets: PenNode[] = [];
    for (const id of insertedIds) {
      const node = findNodeInTree(children, id);
      if (node) rootTargets.push(node);
    }

    // If no specific roots found, process all top-level children
    const targets = rootTargets.length > 0 ? rootTargets : children;

    const hooks = getMcpHooks();
    for (const target of targets) {
      hooks.resolveTreeRoles?.([target], canvasWidth);
      hooks.resolveTreePostPass?.([target]);

      const flat = flattenNodes([target]);
      for (const node of flat) {
        if (node.type === 'path') hooks.applyIconPathResolution?.([node]);
        if (node.type === 'text') hooks.applyNoEmojiIconHeuristic?.([node]);
      }

      hooks.ensureUniqueNodeIds?.([target]);
      hooks.sanitizeLayoutChildPositions?.([target]);
      hooks.sanitizeScreenFrameBounds?.([target]);
    }

    postProcessed = true;
  }

  await saveDocument(filePath, doc);

  return {
    results,
    nodeCount: countNodes(getDocChildren(doc, pageId)),
    postProcessed: postProcessed || undefined,
    errors: errors.length > 0 ? errors : undefined,
  };
}

/**
 * Split DSL operations handling multi-line JSON bodies. Splits on newlines
 * that are OUTSIDE quoted strings and balanced brackets. This lets agents
 * pretty-print JSON across multiple lines inside a single operation.
 */
function splitOperations(raw: string): string[] {
  const result: string[] = [];
  let buf = '';
  let depth = 0;
  let inString = false;
  let escape = false;

  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i];
    buf += ch;
    if (escape) {
      escape = false;
      continue;
    }
    if (ch === '\\' && inString) {
      escape = true;
      continue;
    }
    if (ch === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;
    if (ch === '(' || ch === '[' || ch === '{') depth++;
    else if (ch === ')' || ch === ']' || ch === '}') depth--;
    else if (ch === '\n' && depth === 0) {
      const trimmed = buf.trim();
      if (trimmed && !trimmed.startsWith('//')) result.push(trimmed);
      buf = '';
    }
  }
  const tail = buf.trim();
  if (tail && !tail.startsWith('//')) result.push(tail);
  return result;
}

async function executeLine(
  line: string,
  doc: PenDocument,
  bindings: Map<string, string>,
  results: OpResult[],
  pageId?: string,
): Promise<void> {
  // Parse: binding=OP(args) or OP(args)
  // Binding is optional for I/C/R/G — auto-generated when omitted (agents
  // sometimes write `I(parent, data)` without the binding prefix).
  const assignMatch = line.match(/^(\w+)\s*=\s*([ICRMG])\((.+)\)$/);
  const bindlessAssignMatch = !assignMatch && line.match(/^([ICRG])\((.+)\)$/);
  const callMatch = line.match(/^([UDM])\((.+)\)$/);

  // Normalize bindless form to an auto-generated binding so the rest of the
  // logic below can treat it uniformly.
  const effectiveAssign =
    assignMatch ??
    (bindlessAssignMatch
      ? ([
          line,
          `_auto_${results.length}_${bindlessAssignMatch[1]}`,
          bindlessAssignMatch[1],
          bindlessAssignMatch[2],
        ] as RegExpMatchArray)
      : null);

  if (effectiveAssign) {
    const [, binding, op, argsStr] = effectiveAssign;
    switch (op) {
      case 'I': {
        const { parent, data } = parseInsertArgs(argsStr, bindings);
        const node = { ...data, id: generateId() } as PenNode;

        // Auto-replace: when inserting a frame at root level and an empty
        // root frame exists, replace it instead of creating a sibling.
        if (parent === null && data.type === 'frame') {
          const children = getDocChildren(doc, pageId);
          const emptyIdx = children.findIndex((n) => isEmptyFrame(n));
          if (emptyIdx !== -1) {
            const emptyFrame = children[emptyIdx];
            // Inherit position from the empty frame so the design lands in the right spot
            if (emptyFrame.x !== undefined) node.x = emptyFrame.x;
            if (emptyFrame.y !== undefined) node.y = emptyFrame.y;
            let updated = removeNodeFromTree(children, emptyFrame.id);
            updated = insertNodeInTree(updated, null, node, emptyIdx);
            setDocChildren(doc, updated, pageId);
            bindings.set(binding, node.id);
            results.push({ binding, nodeId: node.id });
            break;
          }
        }

        setDocChildren(doc, insertNodeInTree(getDocChildren(doc, pageId), parent, node), pageId);
        bindings.set(binding, node.id);
        results.push({ binding, nodeId: node.id });
        break;
      }
      case 'C': {
        const { sourceId, parent, data } = parseCopyArgs(argsStr, bindings);
        const source = findNodeInTree(getDocChildren(doc, pageId), sourceId);
        if (!source) throw new Error(`Copy source not found: ${sourceId}`);
        const cloned = cloneNodeWithNewIds(source, generateId);
        // Apply override properties
        if (data) {
          Object.assign(cloned, data);
          // Don't override the cloned id
          if (data.id) delete (cloned as unknown as Record<string, unknown>).id;
        }
        // Apply descendant overrides
        if (data?.descendants) {
          applyDescendantOverrides(cloned, data.descendants as Record<string, unknown>);
        }
        setDocChildren(doc, insertNodeInTree(getDocChildren(doc, pageId), parent, cloned), pageId);
        bindings.set(binding, cloned.id);
        results.push({ binding, nodeId: cloned.id });
        break;
      }
      case 'R': {
        const { path, data } = parseReplaceArgs(argsStr, bindings);
        const resolvedPath = resolveSlashPath(path, doc, bindings);
        const newNode = { ...data, id: generateId() } as PenNode;
        // Find and replace the node
        const oldNode = findNodeByPath(resolvedPath, doc, pageId);
        if (!oldNode) throw new Error(`Replace target not found: ${path}`);
        // Remove old, insert new at same position
        const parent = findParentByPath(resolvedPath, doc, pageId);
        const parentId = parent ? parent.id : null;
        const siblings = parent
          ? 'children' in parent
            ? (parent.children ?? [])
            : []
          : getDocChildren(doc, pageId);
        const idx = siblings.findIndex((n) => n.id === oldNode.id);
        let children = removeNodeFromTree(getDocChildren(doc, pageId), oldNode.id);
        children = insertNodeInTree(children, parentId, newNode, idx);
        setDocChildren(doc, children, pageId);
        bindings.set(binding, newNode.id);
        results.push({ binding, nodeId: newNode.id });
        break;
      }
      case 'M': {
        const { nodeId, parent, index } = parseMoveArgs(argsStr, bindings);
        const node = findNodeInTree(getDocChildren(doc, pageId), nodeId);
        if (!node) throw new Error(`Move target not found: ${nodeId}`);
        let children = removeNodeFromTree(getDocChildren(doc, pageId), nodeId);
        children = insertNodeInTree(children, parent, node, index);
        setDocChildren(doc, children, pageId);
        bindings.set(binding, nodeId);
        results.push({ binding, nodeId });
        break;
      }
      case 'G': {
        const gArgs = argsStr.match(/^"([^"]+)"\s*,\s*"(search|generate)"\s*,\s*"([^"]+)"$/);
        if (!gArgs) throw new Error(`Invalid G() syntax: ${argsStr}`);
        const [, gParent, gMode, gPrompt] = gArgs;
        const resolvedParent = resolveRef(gParent, bindings);

        const imageNode = {
          id: generateId(),
          type: 'image' as const,
          name: gPrompt.slice(0, 40),
          imagePrompt: gPrompt,
          src: '',
          width: 400,
          height: 300,
        };

        // MCP runs in Node.js — must use absolute URL via getSyncUrl()
        const syncUrl = await getSyncUrl();
        if (gMode === 'search' && syncUrl) {
          try {
            const searchRes = await fetch(`${syncUrl}/api/ai/image-search`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ query: gPrompt, count: 1 }),
            });
            const searchData = (await searchRes.json()) as {
              results?: Array<{ thumbUrl: string }>;
            };
            if (searchData.results && searchData.results.length > 0) {
              imageNode.src = searchData.results[0].thumbUrl;
            }
          } catch {
            /* keep empty src */
          }
        }
        // "generate" mode not supported in MCP (no access to API keys in browser store)

        setDocChildren(
          doc,
          insertNodeInTree(
            getDocChildren(doc, pageId),
            resolvedParent,
            imageNode as unknown as PenNode,
          ),
          pageId,
        );
        bindings.set(binding, imageNode.id);
        results.push({ binding, nodeId: imageNode.id });
        break;
      }
    }
  } else if (callMatch) {
    const [, op, argsStr] = callMatch;
    switch (op) {
      case 'U': {
        const { path, data } = parseUpdateArgs(argsStr, bindings);
        const resolvedPath = resolveSlashPath(path, doc, bindings);
        const targetNode = findNodeByPath(resolvedPath, doc, pageId);
        if (!targetNode) throw new Error(`Update target not found: ${path}`);
        // Update the node in-place
        setDocChildren(
          doc,
          updateNodeInTree(getDocChildren(doc, pageId), targetNode.id, data),
          pageId,
        );
        break;
      }
      case 'D': {
        const nodeId = resolveRef(argsStr.trim().replace(/^"|"$/g, ''), bindings);
        setDocChildren(doc, removeNodeFromTree(getDocChildren(doc, pageId), nodeId), pageId);
        break;
      }
      case 'M': {
        const { nodeId, parent, index } = parseMoveArgs(argsStr, bindings);
        const node = findNodeInTree(getDocChildren(doc, pageId), nodeId);
        if (!node) throw new Error(`Move target not found: ${nodeId}`);
        let children = removeNodeFromTree(getDocChildren(doc, pageId), nodeId);
        children = insertNodeInTree(children, parent, node, index);
        setDocChildren(doc, children, pageId);
        break;
      }
    }
  } else {
    throw new Error(`Cannot parse operation: ${line}`);
  }
}

// --- Argument parsers ---

function parseInsertArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { parent: string | null; data: Record<string, unknown> } {
  const firstComma = findTopLevelComma(argsStr);
  if (firstComma === -1) throw new Error('Insert requires parent and node data');
  const parentRaw = argsStr.slice(0, firstComma).trim();
  const dataStr = argsStr.slice(firstComma + 1).trim();
  const parent = parentRaw === 'null' ? null : resolveRef(parentRaw, bindings);
  const data = parseJsonArg(dataStr);
  return { parent, data };
}

function parseCopyArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { sourceId: string; parent: string | null; data: Record<string, unknown> } {
  const first = findTopLevelComma(argsStr);
  if (first === -1) throw new Error('Copy requires sourceId, parent, and data');
  const sourceRaw = argsStr.slice(0, first).trim();
  const rest = argsStr.slice(first + 1).trim();
  const second = findTopLevelComma(rest);

  let parentRaw: string;
  let dataStr: string;

  if (second === -1) {
    parentRaw = rest;
    dataStr = '{}';
  } else {
    parentRaw = rest.slice(0, second).trim();
    dataStr = rest.slice(second + 1).trim();
  }

  return {
    sourceId: resolveRef(sourceRaw, bindings),
    parent: parentRaw === 'null' ? null : resolveRef(parentRaw, bindings),
    data: parseJsonArg(dataStr),
  };
}

function parseUpdateArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { path: string; data: Record<string, unknown> } {
  const firstComma = findTopLevelComma(argsStr);
  if (firstComma === -1) throw new Error('Update requires path and update data');
  const pathRaw = argsStr.slice(0, firstComma).trim();
  const dataStr = argsStr.slice(firstComma + 1).trim();
  const path = resolvePathExpr(pathRaw, bindings);
  return { path, data: parseJsonArg(dataStr) };
}

function parseReplaceArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { path: string; data: Record<string, unknown> } {
  const firstComma = findTopLevelComma(argsStr);
  if (firstComma === -1) throw new Error('Replace requires path and node data');
  const pathRaw = argsStr.slice(0, firstComma).trim();
  const dataStr = argsStr.slice(firstComma + 1).trim();
  const path = resolvePathExpr(pathRaw, bindings);
  return { path, data: parseJsonArg(dataStr) };
}

function parseMoveArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { nodeId: string; parent: string | null; index?: number } {
  const parts = splitTopLevel(argsStr);
  if (parts.length < 2) throw new Error('Move requires nodeId and parent');
  return {
    nodeId: resolveRef(parts[0].trim(), bindings),
    parent:
      parts[1].trim() === 'null' || parts[1].trim() === 'undefined'
        ? null
        : resolveRef(parts[1].trim(), bindings),
    index: parts[2] ? parseInt(parts[2].trim(), 10) : undefined,
  };
}

// --- Helpers ---

function resolveRef(raw: string, bindings: Map<string, string>): string {
  const cleaned = raw.replace(/^"|"$/g, '');
  return bindings.get(cleaned) ?? cleaned;
}

/** Resolve path expressions like `binding+"/child"` or `"id"` */
function resolvePathExpr(raw: string, bindings: Map<string, string>): string {
  if (raw.includes('+')) {
    return raw
      .split('+')
      .map((p) => {
        const t = p.trim();
        if (t.startsWith('"') || t.startsWith("'")) {
          return t.slice(1, -1);
        }
        return bindings.get(t) ?? t;
      })
      .join('');
  }
  const cleaned = raw.replace(/^"|"$/g, '');
  return bindings.get(cleaned) ?? cleaned;
}

/** Resolve slash-separated path (e.g. "instanceId/childId") to find the actual node. */
function resolveSlashPath(path: string, _doc: PenDocument, _bindings: Map<string, string>): string {
  // The path may be "parentId/childId" — we return as-is for findNodeByPath
  return path;
}

function findNodeByPath(path: string, doc: PenDocument, pageId?: string): PenNode | undefined {
  const children = getDocChildren(doc, pageId);
  const parts = path.split('/');
  if (parts.length === 1) {
    return findNodeInTree(children, parts[0]);
  }
  // For paths like "instanceId/childId", resolve through the tree
  // First find the instance, then look for child
  let current = findNodeInTree(children, parts[0]);
  for (let i = 1; i < parts.length && current; i++) {
    if ('children' in current && current.children) {
      current = current.children.find((c) => c.id === parts[i]);
    } else {
      // For ref nodes, the child might be in the referenced component
      return undefined;
    }
  }
  return current;
}

function findParentByPath(path: string, doc: PenDocument, pageId?: string): PenNode | undefined {
  const parts = path.split('/');
  if (parts.length <= 1) {
    // Top-level or simple ID — find parent in tree
    return findParentInTree(getDocChildren(doc, pageId), parts[0]);
  }
  // Parent is the second-to-last part
  const parentPath = parts.slice(0, -1).join('/');
  return findNodeByPath(parentPath, doc, pageId);
}

function findParentInTree(nodes: PenNode[], id: string): PenNode | undefined {
  for (const node of nodes) {
    if ('children' in node && node.children) {
      for (const child of node.children) {
        if (child.id === id) return node;
      }
      const found = findParentInTree(node.children, id);
      if (found) return found;
    }
  }
  return undefined;
}

function applyDescendantOverrides(node: PenNode, descendants: Record<string, unknown>): void {
  if (!('children' in node) || !node.children) return;
  for (const child of node.children) {
    const override = descendants[child.id];
    if (override) {
      Object.assign(child, override);
    }
    applyDescendantOverrides(child, descendants);
  }
}

/** Parse a JSON-like argument, handling unquoted keys. */
function parseJsonArg(str: string): Record<string, unknown> {
  const trimmed = str.trim();
  // Try strict JSON first (most common case — avoids mangling values like "Don't")
  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    /* fall through to lenient parsing */
  }

  if (parsed === undefined) {
    let normalized = trimmed;
    // Convert JavaScript-style object to JSON: unquoted keys → quoted
    normalized = normalized.replace(/(?<=\{|,)\s*(\w+)\s*:/g, ' "$1":');
    // Replace single-quoted string delimiters with double quotes (not quotes inside strings)
    normalized = replaceSingleQuoteDelimiters(normalized);
    // Remove empty keys like `, ": 50,` (agent truncation artifact) —
    // skip the property entirely so the rest of the object still parses.
    normalized = normalized.replace(/,\s*""\s*:\s*[^,}\]]+/g, '');
    // Remove trailing commas before } or ] (common agent mistake)
    normalized = normalized.replace(/,(\s*[}\]])/g, '$1');
    try {
      parsed = JSON.parse(normalized);
    } catch (err) {
      const snippet = str.slice(0, 300);
      throw new Error(
        `Failed to parse JSON (${err instanceof Error ? err.message : 'unknown'}): ${snippet}${str.length > 300 ? '...' : ''}`,
      );
    }
  }

  return sanitizeObject(normalizeNodeShape(parsed)) as Record<string, unknown>;
}

/**
 * Normalize common agent shorthand into canonical PenNode format.
 * Applied before sanitizeObject so downstream code sees a consistent shape.
 */
function normalizeNodeShape(input: unknown): unknown {
  if (Array.isArray(input)) return input.map(normalizeNodeShape);
  if (!input || typeof input !== 'object') return input;
  const obj = input as Record<string, unknown>;

  // fill: "#hex" | "solid#hex" → [{ type: 'solid', color: '#hex' }]
  // fill: { type, color } → [{ ... }]
  if ('fill' in obj) {
    obj.fill = normalizeFillField(obj.fill);
  }

  // stroke: { thickness, color } | { thickness, color: '#hex' } →
  //   { thickness, fill: [{ type: 'solid', color: '#hex' }] }
  // stroke: '#hex' → { thickness: 1, fill: [{ type: 'solid', color: '#hex' }] }
  if ('stroke' in obj) {
    obj.stroke = normalizeStrokeField(obj.stroke);
  }

  // Recurse into children
  if (Array.isArray(obj.children)) {
    obj.children = obj.children.map((c) => normalizeNodeShape(c));
  }

  return obj;
}

function normalizeFillField(value: unknown): unknown {
  if (value == null) return value;
  // '#hex' shorthand
  if (typeof value === 'string') {
    return [{ type: 'solid', color: value }];
  }
  // Single fill object → wrap in array
  if (typeof value === 'object' && !Array.isArray(value)) {
    return [value];
  }
  // Already an array — leave as-is
  return value;
}

function normalizeStrokeField(value: unknown): unknown {
  if (value == null) return value;
  // '#hex' shorthand → stroke with solid fill
  if (typeof value === 'string') {
    return { thickness: 1, fill: [{ type: 'solid', color: value }] };
  }
  if (typeof value !== 'object') return value;
  const stroke = value as Record<string, unknown>;
  // { thickness, color: '#hex' } → { thickness, fill: [...] }
  if (stroke.color != null && stroke.fill == null) {
    stroke.fill = [{ type: 'solid', color: stroke.color }];
    delete stroke.color;
  }
  // { fill: '#hex' | object } → { fill: [...] }
  if (stroke.fill != null) {
    stroke.fill = normalizeFillField(stroke.fill);
  }
  // { thickness, type, color } variant
  if (stroke.type != null && stroke.color != null && stroke.fill == null) {
    stroke.fill = [{ type: stroke.type, color: stroke.color }];
    delete stroke.type;
    delete stroke.color;
  }
  return stroke;
}

/** Replace single-quote string delimiters with double quotes, leaving apostrophes inside strings. */
function replaceSingleQuoteDelimiters(str: string): string {
  const chars: string[] = [];
  let inDouble = false;
  let inSingle = false;
  for (let i = 0; i < str.length; i++) {
    const ch = str[i];
    if (ch === '\\' && (inDouble || inSingle)) {
      chars.push(ch, str[++i] ?? '');
      continue;
    }
    if (inDouble) {
      if (ch === '"') inDouble = false;
      chars.push(ch);
    } else if (inSingle) {
      if (ch === "'") {
        inSingle = false;
        chars.push('"'); // closing single quote → double quote
      } else {
        chars.push(ch);
      }
    } else {
      if (ch === '"') {
        inDouble = true;
        chars.push(ch);
      } else if (ch === "'") {
        inSingle = true;
        chars.push('"'); // opening single quote → double quote
      } else {
        chars.push(ch);
      }
    }
  }
  return chars.join('');
}

/** Find the index of the first comma not inside braces/brackets/quotes. */
function findTopLevelComma(str: string): number {
  let depth = 0;
  let inString = false;
  let quote = '';
  for (let i = 0; i < str.length; i++) {
    const ch = str[i];
    if (inString) {
      if (ch === '\\') {
        i++;
        continue;
      }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") {
      inString = true;
      quote = ch;
      continue;
    }
    if (ch === '{' || ch === '[' || ch === '(') depth++;
    if (ch === '}' || ch === ']' || ch === ')') depth--;
    if (ch === ',' && depth === 0) return i;
  }
  return -1;
}

function splitTopLevel(str: string): string[] {
  const result: string[] = [];
  let start = 0;
  let depth = 0;
  let inString = false;
  let quote = '';
  for (let i = 0; i < str.length; i++) {
    const ch = str[i];
    if (inString) {
      if (ch === '\\') {
        i++;
        continue;
      }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") {
      inString = true;
      quote = ch;
      continue;
    }
    if (ch === '{' || ch === '[' || ch === '(') depth++;
    if (ch === '}' || ch === ']' || ch === ')') depth--;
    if (ch === ',' && depth === 0) {
      result.push(str.slice(start, i));
      start = i + 1;
    }
  }
  result.push(str.slice(start));
  return result;
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

/** A root frame is "empty" if it has no children. */
function isEmptyFrame(node: PenNode): boolean {
  return (
    node.type === 'frame' && (!('children' in node) || !node.children || node.children.length === 0)
  );
}
