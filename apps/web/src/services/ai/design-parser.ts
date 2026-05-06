import type { PenNode } from '@/types/pen';

// ---------------------------------------------------------------------------
// Streaming JSONL 解析器结果
// ---------------------------------------------------------------------------

export interface StreamingNodeResult {
  node: PenNode;
  parentId: string | null;
}

// ---------------------------------------------------------------------------
// 从 AI 响应文本中提取 JSON
// ---------------------------------------------------------------------------

/**
 * Extract
 * PenNode JSON 来自 AI 响应文本。 Handles ```json
 块、JSONL 格式、原始数组和后备解析。
 */
/**
 * Strip 第三方模型可
 * 能注入的非标准 XML 类标签（例如 `<minimax:tool_call>`、`<tool_call>`、`<|im_start|
 * >`）。 Preserves 标签之间的内容，因此任何嵌入的 JSON 仍然可以被解析。
 */
function stripNonStandardTags(text: string): string {
  return text
    .replace(/<\/?[\w:.]+:[\w]+[^>]*>/g, '') // 命名空间标签：<minimax:tool_call>
    .replace(/<\/?tool_call[^>]*>/g, '') // <tool_call>，</tool_call>
    .replace(/<\|[\w_]+\|>/g, '') // 聊天模板标记：<|im_start|>
    .replace(/\[TOOL_CALL\]/gi, ''); // 括号式工具调用标记
}

/**
 * Strip 基本层模型（MiniMax 等）可能发出的虚假工具调用块。
 * These look like `{tool => "Write", args => { ... }}` and are not valid JSON.
 * Must 运行 BEFORE JSON 提取，因此大括号扫描不会拾取它们。
 */
function stripToolCallBlocks(text: string): string {
  // Remove `{tool => "...", args => { ... }}` blocks (arrow-syntax pseudo-calls)
// These use `=>` instead of `:` for key-value pairs
  return text.replace(/\{tool\s*=>\s*"[^"]*"\s*,\s*args\s*=>[\s\S]*$/gi, '');
}

export function extractJsonFromResponse(text: string): PenNode[] | null {
  // Clean 解析前的非标准模型工件
  const cleaned = stripToolCallBlocks(stripNonStandardTags(text));

  const parsedBlocks = extractAllJsonBlocks(cleaned)
    .map((block) => tryParseNodes(block))
    .filter(Boolean) as PenNode[][];

  if (parsedBlocks.length > 0) {
    return selectBestNodeSet(parsedBlocks);
  }

  // Try JSONL 格式（带有 _parent 字段的平面节点）
  const jsonlTree = parseJsonlToTree(cleaned);
  if (jsonlTree) return jsonlTree;

  // Fallback：如果没有找到块，尝试查找单个 JSON 数组
  const arrayMatch = cleaned.match(/\[\s*\{[\s\S]*\}\s*\]/);
  if (arrayMatch) {
    const nodes = tryParseNodes(arrayMatch[0]);
    return nodes;
  }

  // Fallback：尝试解析具有嵌套子节点的单个根节点（较弱的模型可能会输出一个根对象而不是数组）
  const singleRoot = tryParseSingleRootNode(cleaned);
  if (singleRoot) return singleRoot;

  // Fallback：删除 <step> 标签后尝试解析原始文本。
  const stripped = cleaned.replace(/<step[\s\S]*?<\/step>/g, '').trim();
  const directNodes = tryParseNodes(stripped);
  if (directNodes) {
    return directNodes;
  }

  return null;
}

// ---------------------------------------------------------------------------
// Streaming JSONL 解析器 — 从内部提取完整的 JSON 对象
// 当它们流入时，会出现一个 ```json 块，从而实现逐元素渲染。
// ---------------------------------------------------------------------------

/**
 * Extract
 * 从流文本中完成了 JSON 对象（在用于树插入的“`json block). Uses brace-counting to detect
 * complete objects before the block closes. Each object is expected to
 have a `_parent”字段内）。
 */
export function extractStreamingNodes(
  text: string,
  processedOffset: number,
): { results: StreamingNodeResult[]; newOffset: number } {
  // Primary 模式：在 ```json 围栏块内解析。 Fallback
  // 模式：当模型省略栅栏时解析原始 JSONL/object 文本。
  const jsonBlockStart = text.indexOf('```json');

  let contentStart = -1;
  let searchEnd = text.length;
  if (jsonBlockStart !== -1) {
    const firstNewline = text.indexOf('\n', jsonBlockStart);
    if (firstNewline === -1) return { results: [], newOffset: processedOffset };
    contentStart = firstNewline + 1;
    const blockEnd = text.indexOf('\n```', contentStart);
    searchEnd = blockEnd > 0 ? blockEnd : text.length;
  } else {
    const firstBrace = text.indexOf('{');
    if (firstBrace === -1) return { results: [], newOffset: processedOffset };
    contentStart = firstBrace;
  }

  const startPos = Math.max(processedOffset, contentStart);

  const results: StreamingNodeResult[] = [];
  let i = startPos;

  while (i < searchEnd) {
    // Skip 到下一个“{”字符
    while (i < searchEnd && text[i] !== '{') i++;
    if (i >= searchEnd) break;

    // Brace-计数以查找匹配的“}”
    const objStart = i;
    let depth = 0;
    let inString = false;
    let escaped = false;
    let j = i;

    while (j < searchEnd) {
      const ch = text[j];
      if (escaped) {
        escaped = false;
        j++;
        continue;
      }
      if (ch === '\\' && inString) {
        escaped = true;
        j++;
        continue;
      }
      if (ch === '"') {
        inString = !inString;
        j++;
        continue;
      }
      if (inString) {
        j++;
        continue;
      }
      if (ch === '{') depth++;
      else if (ch === '}') {
        depth--;
        if (depth === 0) {
          // 找到 Complete 对象
          const objStr = text.slice(objStart, j + 1);
          try {
            const obj = JSON.parse(objStr) as Record<string, unknown>;
            if (obj.id && obj.type) {
              const parentId = (obj._parent as string | null) ?? null;
              delete obj._parent;
              results.push({ node: obj as unknown as PenNode, parentId });
            }
          } catch {
            /* 格式错误 JSON，跳过 */
          }
          i = j + 1;
          break;
        }
      }
      j++;
    }

    if (depth > 0) break; // Incomplete 对象，等待更多数据
  }

  return { results, newOffset: i };
}

// ---------------------------------------------------------------------------
// Internal 帮助者
// ---------------------------------------------------------------------------

/**
 * Helper 查找文本中所有完整的 JSON 块（```json or ``` 块）。
 */
function extractAllJsonBlocks(text: string): string[] {
  const blocks: string[] = [];
  // Matches ```json or ``` 块
  const regex = /```(?:json)?\s*\n?([\s\S]*?)\n?\s*```/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    // Basic 启发式：在添加之前检查它是否看起来像 JSON array/object
    const content = match[1].trim();
    if (content.startsWith('[') || content.startsWith('{')) {
      blocks.push(content);
    }
  }
  return blocks;
}

/**
 * Parse JSONL
 * 格式响应（带有 _parent 字段的平面节点）到树中。 Used by extractAndApplyDesign 用于批量应用 JSONL
 内容。
 */
function parseJsonlToTree(text: string): PenNode[] | null {
  const { results } = extractStreamingNodes(text, 0);
  if (results.length === 0) return null;

  const nodeMap = new Map<string, PenNode>();
  const roots: PenNode[] = [];

  for (const { node, parentId } of results) {
    nodeMap.set(node.id, node);

    if (parentId === null) {
      roots.push(node);
    } else {
      const parent = nodeMap.get(parentId);
      if (parent) {
        if (
          !('children' in parent) ||
          !Array.isArray((parent as PenNode & { children?: PenNode[] }).children)
        ) {
          (parent as PenNode & { children?: PenNode[] }).children = [];
        }
        (parent as PenNode & { children: PenNode[] }).children.push(node);
      } else {
        roots.push(node); // Parent 未找到，视为 root
      }
    }
  }

  return roots.length > 0 ? roots : null;
}

/**
 * Try 从原始文本中解析
 * 带有嵌套子项的单个根 PenNode。 Handles 较弱的模型输出单个 JSON 对象而不是数组或 JSONL 格式的情况。
 *
 */
function tryParseSingleRootNode(text: string): PenNode[] | null {
  const first = text.indexOf('{');
  const last = text.lastIndexOf('}');
  if (first < 0 || last <= first) return null;
  try {
    const obj = JSON.parse(text.slice(first, last + 1)) as Record<string, unknown>;
    if (typeof obj.id === 'string' && typeof obj.type === 'string' && Array.isArray(obj.children)) {
      return [obj as unknown as PenNode];
    }
  } catch {
    /* 忽略解析错误 */
  }
  return null;
}

function tryParseNodes(json: string): PenNode[] | null {
  try {
    const parsed = JSON.parse(json.trim());
    const nodes = Array.isArray(parsed) ? parsed : [parsed];
    return validateNodes(nodes) ? nodes : null;
  } catch {
    return null;
  }
}

function validateNodes(nodes: unknown[]): nodes is PenNode[] {
  return nodes.every(
    (node) =>
      typeof node === 'object' &&
      node !== null &&
      'id' in node &&
      'type' in node &&
      typeof (node as PenNode).id === 'string' &&
      typeof (node as PenNode).type === 'string',
  );
}

function selectBestNodeSet(candidates: PenNode[][]): PenNode[] {
  let best = candidates[candidates.length - 1];
  let bestScore = scoreNodeSet(best);

  for (const candidate of candidates) {
    const score = scoreNodeSet(candidate);
    // Favor 稍后会阻止连接以保留最新的完整输出。
    if (score >= bestScore) {
      best = candidate;
      bestScore = score;
    }
  }

  return best;
}

function scoreNodeSet(nodes: PenNode[]): number {
  let score = nodes.length;

  if (nodes.length === 1 && nodes[0].type === 'frame') {
    score += 1000;
    const root = nodes[0];
    if ((root.x ?? 0) === 0 && (root.y ?? 0) === 0) score += 50;
    if ('children' in root && Array.isArray(root.children)) {
      score += root.children.length * 10;
    }
  }

  if (nodes.length > 1) {
    score -= 200;
  }

  for (const node of nodes) {
    if ('children' in node && Array.isArray(node.children)) {
      score += node.children.length * 2;
    }
  }

  return score;
}
