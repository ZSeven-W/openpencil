import type { PenNode } from '@/types/pen';
import { useDocumentStore, DEFAULT_FRAME_ID, getActivePageChildren } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { useHistoryStore } from '@/stores/history-store';
import {
  pendingAnimationNodes,
  markNodesForAnimation,
  startNewAnimationBatch,
  resetAnimationState,
} from './design-animation';
import {
  toSizeNumber,
  createPhonePlaceholderDataUri,
  estimateNodeIntrinsicHeight,
} from './generation-utils';
import { defaultLineHeight } from '@/canvas/canvas-text-measure';
import {
  normalizeTreeLayout,
  unwrapFakePhoneMockups,
  stripRedundantSectionFills,
  normalizeStrokeFillSchema,
} from '@/canvas/canvas-layout-engine';
import { forcePageResync } from '@/canvas/canvas-sync-utils';
import {
  applyIconPathResolution,
  applyNoEmojiIconHeuristic,
  resolveAsyncIcons,
  resolveAllPendingIcons,
} from './icon-resolver';
import {
  resolveNodeRole,
  resolveTreeRoles,
  resolveTreePostPass,
  detectThemeFromNode,
} from './role-resolver';
import type { RoleContext } from './role-resolver';
import { rewriteLlmAntiPatterns } from './sanitize-llm-anti-patterns';
// 触发所有 role definition 的副作用注册
import './role-definitions';
import { extractJsonFromResponse } from './design-parser';
import {
  scanAndFillImages,
  enqueueImageForSearch,
  resetImageSearchQueue,
} from './image-search-pipeline';
import {
  deepCloneNode,
  mergeNodeForProgressiveUpsert,
  ensureUniqueNodeIds,
  sanitizeLayoutChildPositions,
  sanitizeScreenFrameBounds,
  hasActiveLayout,
  isOverlayNode,
} from './design-node-sanitization';

// ---------------------------------------------------------------------------
// 跨阶段 ID 重映射：
// 记录 `replaceEmptyFrame()` 做过的映射，
// 让后续阶段知道模型生成的根帧 ID 已经换成了真实画布根帧 ID。
// ---------------------------------------------------------------------------

const generationRemappedIds = new Map<string, string>();
let generationContextHint = '';
/** 当前这一轮生成的根框架宽度（桌面通常是 1200，移动通常是 375）。 */
let generationCanvasWidth = 1200;
/** 当前这一轮生成所使用的根框架 ID。 */
let generationRootFrameId: string = DEFAULT_FRAME_ID;
/** 当前这一轮开始前，画布里已经存在的节点 ID。 */
let preExistingNodeIds = new Set<string>();

/**
 * 返回当前活动页面上的第一个顶级 frame ID。
 *
 * 在多页文档里，真正的“当前页根框架”不能简单写死成 `DEFAULT_FRAME_ID`，
 * 因为只有第一页会使用它；后续页面都会由 `addPage()` 生成新的 nanoid。
 * 所以凡是想定位“当前页面根框架”的地方，都应该走这个辅助函数。
 */
function getActivePagePrimaryFrameId(): string | null {
  const doc = useDocumentStore.getState().document;
  const activePageId = useCanvasStore.getState().activePageId;
  const children = getActivePageChildren(doc, activePageId);
  for (const child of children) {
    if (child.type === 'frame') return child.id;
  }
  return null;
}

export function resetGenerationRemapping(): void {
  generationRemappedIds.clear();
  // 只有当前页还没有 frame（例如首次加载或遗留文档）时，
  // 才退回 `DEFAULT_FRAME_ID`；否则优先使用真实的活动页根框架。
  generationRootFrameId = getActivePagePrimaryFrameId() ?? DEFAULT_FRAME_ID;
  // 快照当前所有节点 ID，后续 upsert 会用它避免冲突
  preExistingNodeIds = new Set(
    useDocumentStore
      .getState()
      .getFlatNodes()
      .map((n) => n.id),
  );
  // 重置这一轮生成的增量图片搜索队列
  resetImageSearchQueue();
}

export function setGenerationContextHint(hint?: string): void {
  generationContextHint = hint?.trim() ?? '';
}

export function setGenerationCanvasWidth(width: number): void {
  generationCanvasWidth = width > 0 ? width : 1200;
}

/** 对外暴露当前生成宽度（只读）。 */
export function getGenerationCanvasWidth(): number {
  return generationCanvasWidth;
}

/** 对外暴露当前生成根框架 ID（只读）。 */
export function getGenerationRootFrameId(): string {
  return generationRootFrameId;
}

/** 覆盖当前根框架 ID，供追加模式复用已有页面框架。 */
export function setGenerationRootFrameId(id: string): void {
  generationRootFrameId = id;
}

/** 对外暴露当前 ID 重映射表（只读）。 */
export function getGenerationRemappedIds(): Map<string, string> {
  return generationRemappedIds;
}

// ---------------------------------------------------------------------------
// 将单个流式节点插入画布
// ---------------------------------------------------------------------------

/**
 * 立即把一个流式节点插入画布。
 *
 * 这里会顺带处理：
 * - 根框架替换
 * - 父节点 ID 重映射
 *
 * 注意：依赖完整子树的启发式（例如按钮宽度、框架高度、clipContent）
 * 在这里还跑不了，因为流式阶段节点到达时通常还没有子节点。
 * 等整棵子树插完后，再调用 `applyPostStreamingTreeHeuristics()` 补跑。
 */
/**
 * 就地规范化节点填充里的渐变 stop 偏移。
 * 没有 offset 时自动均匀分布；
 * 百分比形式的 offset（>1）会被归一到 0~1。
 */
function normalizeNodeFills(node: PenNode): void {
  const fills = 'fill' in node ? (node as { fill?: unknown }).fill : undefined;

  // 把字符串简写（例如 "#000000"）转换成标准 PenFill 数组
  if (typeof fills === 'string') {
    (node as unknown as Record<string, unknown>).fill = [{ type: 'solid', color: fills }];
    return;
  }

  if (!Array.isArray(fills)) return;

  // 把数组里残留的字符串填充项也补成 solid fill 对象
  for (let i = 0; i < fills.length; i++) {
    if (typeof fills[i] === 'string') {
      fills[i] = { type: 'solid', color: fills[i] };
    }
  }

  for (const fill of fills) {
    if (!fill || typeof fill !== 'object') continue;
    const f = fill as { type?: string; stops?: unknown[] };
    if ((f.type === 'linear_gradient' || f.type === 'radial_gradient') && Array.isArray(f.stops)) {
      const n = f.stops.length;
      f.stops = f.stops.map((s: unknown, i: number) => {
        const stop = s as Record<string, unknown>;
        let offset =
          typeof stop.offset === 'number' && Number.isFinite(stop.offset)
            ? stop.offset
            : typeof stop.position === 'number' && Number.isFinite(stop.position)
              ? (stop.position as number)
              : null;
        if (offset !== null && offset > 1) offset = offset / 100;
        return {
          color: typeof stop.color === 'string' ? stop.color : '#000000',
          offset: offset !== null ? Math.max(0, Math.min(1, offset)) : i / Math.max(n - 1, 1),
        };
      });
    }
  }
}

export function insertStreamingNode(node: PenNode, parentId: string | null): void {
  const { addNode, getNodeById } = useDocumentStore.getState();
  normalizeNodeFills(node);

  // 保证节点 ID 唯一，避免和画布里已有内容冲突。
  // upsert 路径本来就会做这件事，但流式路径以前没有，
  // 所以两轮生成如果碰巧用了相同 ID，就会出现重复对象。
  const streamCounters = new Map<string, number>();
  const streamRemaps = new Map<string, string>();
  ensureUniqueNodeIds(node, preExistingNodeIds, streamCounters, streamRemaps);
  // 把新插入的 ID 也登记进去，避免后续流节点再次撞 ID
  const trackNewIds = (n: PenNode) => {
    preExistingNodeIds.add(n.id);
    if ('children' in n && Array.isArray(n.children)) {
      for (const child of n.children) trackNewIds(child);
    }
  };
  trackNewIds(node);
  // 把这次产生的重映射并入整轮生成的全局映射表
  for (const [from, to] of streamRemaps) {
    generationRemappedIds.set(from, to);
  }

  // 保证容器节点一定有 `children` 数组，方便后续继续往里插
  if ((node.type === 'frame' || node.type === 'group') && !('children' in node)) {
    (node as PenNode & { children: PenNode[] }).children = [];
  }

  // 解析父节点 ID 的重映射结果（例如模型根框架 -> 真实根框架）
  const resolvedParent = parentId ? (generationRemappedIds.get(parentId) ?? parentId) : null;

  const parentNode = resolvedParent ? getNodeById(resolvedParent) : null;

  if (parentNode && hasActiveLayout(parentNode) && !isOverlayNode(node)) {
    if ('x' in node) delete (node as { x?: number }).x;
    if ('y' in node) delete (node as { y?: number }).y;
    // 布局容器里的文本默认遵循一套经验规则：
    // - 垂直布局中，正文更适合拉伸宽度后换行
    // - 水平布局中，短标签更适合收缩包裹内容，避免挤压兄弟节点
    if (node.type === 'text') {
      const parentLayout = 'layout' in parentNode ? parentNode.layout : undefined;
      const content = 'content' in node ? ((node.content as string) ?? '') : '';
      const isLongText = content.length > 15;

      if (parentLayout === 'vertical') {
        // 只有真正需要换行的长文本，才强制 `fill_container + fixed-width`。
        // 短标签 / 标题 / 数字更适合自然包裹宽度。
        if (isLongText) {
          if (typeof node.width === 'number') node.width = 'fill_container';
          if (!node.textGrowth) node.textGrowth = 'fixed-width';
        } else {
          // 垂直布局里的短文本：可以拉伸宽度，但不强制换行
          if (typeof node.width === 'number') node.width = 'fill_container';
        }
      } else if (parentLayout === 'horizontal') {
        if (
          typeof node.width === 'string' &&
          node.width.startsWith('fill_container') &&
          !isLongText
        ) {
          node.width = 'fit_content';
        }
        if (
          !isLongText &&
          (!node.textGrowth ||
            node.textGrowth === 'fixed-width' ||
            node.textGrowth === 'fixed-width-height')
        ) {
          node.textGrowth = 'auto';
        }
      }
      // 尊重 AI 已经显式给出的 `textGrowth`。
      // 但固定像素高度通常不可靠，容易造成文本裁切或重叠，
      // 所以这里优先删掉，让引擎自己计算高度。
      if (typeof node.height === 'number' && node.textGrowth !== 'fixed-width-height') {
        delete (node as { height?: unknown }).height;
      }
      // 默认行高按文本角色推断（标题 / 正文）
      if (!node.lineHeight) {
        node.lineHeight = defaultLineHeight(node.fontSize ?? 16);
      }
    }
  }

  // 先基于 role 应用默认值，再跑传统启发式。
  //
  // 这里的主题检测有一个流式场景专属细节：
  // 页面根框架总是会先于子节点写入 store，
  // 所以当子节点来到这里时，通常已经能从活动页根框架读到正确主题。
  //
  // 但对“根节点自己”来说，store 里看到的可能还是旧的默认空框架。
  // 因此这里会把当前 `node` 也一起参与主题检测：
  // 如果当前流入的根节点本身就带有深色背景填充，
  // 就优先用它；否则再退回 live store。
  const roleCtx: RoleContext = {
    parentRole: parentNode?.role,
    parentLayout: parentNode && 'layout' in parentNode ? parentNode.layout : undefined,
    canvasWidth: generationCanvasWidth,
    theme: detectActiveDocumentTheme([node]),
  };
  resolveNodeRole(node, roleCtx);

  applyGenerationHeuristics(node);

  // 递归删除布局容器子节点上的 `x/y`，
  // 让布局引擎在同步时重新接管定位。
  const parentHasLayout = parentNode ? hasActiveLayout(parentNode) : false;
  sanitizeLayoutChildPositions(node, parentHasLayout);

  // 如果父节点不存在，或者当前节点处在 Phone Placeholder 内部，就跳过。
  // Placeholder 内部结构会在流结束后统一规范化。
  if (resolvedParent !== null && !parentNode) {
    return;
  }
  if (parentNode && isInsidePhonePlaceholder(resolvedParent!, getNodeById)) {
    return;
  }

  if (resolvedParent === null && node.type === 'frame') {
    if (isCanvasOnlyEmptyFrame()) {
      // 根框架替换当前页空白框架，不需要动画。
      // `replaceEmptyFrame()` 会返回真实目标 ID：
      // 第 1 页通常是 `DEFAULT_FRAME_ID`，后续页面则是各自的 nanoid。
      const targetId = replaceEmptyFrame(node);
      if (targetId) generationRootFrameId = targetId;
    } else {
      // 画布已有内容时，把它作为新的顶级框架加到现有内容旁边
      const { document: doc } = useDocumentStore.getState();
      const activePageId = useCanvasStore.getState().activePageId;
      const pageChildren = getActivePageChildren(doc, activePageId);
      let maxRight = 0;
      for (const child of pageChildren) {
        const cx = child.x ?? 0;
        const cw = 'width' in child && typeof child.width === 'number' ? child.width : 0;
        maxRight = Math.max(maxRight, cx + cw);
      }
      node.x = maxRight + 100;
      node.y = 0;
      generationRootFrameId = node.id;
      addNode(null, node);
    }
  } else {
    const effectiveParent = resolvedParent ?? generationRootFrameId;
    // 先确认父节点存在，不存在就回退到本轮生成根框架
    const parent = getNodeById(effectiveParent);
    const insertParent = parent ? effectiveParent : generationRootFrameId;

    // 带填充的 frame 立即出现，方便子节点拿到正确背景上下文；
    // 其他节点则走交错淡入动画。
    const nodeFill = 'fill' in node ? node.fill : undefined;
    const hasFill = Array.isArray(nodeFill)
      ? nodeFill.length > 0
      : nodeFill != null && typeof nodeFill === 'object';
    const isBackgroundFrame = node.type === 'frame' && hasFill;
    if (!isBackgroundFrame) {
      pendingAnimationNodes.add(node.id);
      startNewAnimationBatch();
    }

    // badge / overlay 节点插到最前面，保证视觉层级更高；
    // 其余节点按追加顺序插入，保留自动布局生成顺序。
    addNode(insertParent, node, isOverlayNode(node) ? 0 : Infinity);

    // 向水平布局里插 frame 时，必要时把兄弟卡片宽度拉平，避免同一行溢出。
    if (node.type === 'frame') {
      equalizeHorizontalSiblings(insertParent);
    }

    // 直接往生成根框架下挂顶级 section 时，顺手增量扩展根高度。
    if (insertParent === generationRootFrameId) {
      expandRootFrameHeight();
    }
  }

  // 图片节点一落地就立刻排进后台补图队列
  if (node.type === 'image') {
    enqueueImageForSearch(node);
  }
}

// ---------------------------------------------------------------------------
// Canvas apply/upsert 操作
// ---------------------------------------------------------------------------

/**
 * 非流式 apply 路径的页面根清理。
 *
 * 它和 `applyPostStreamingTreeHeuristics()` 在流式路径里做的事情类似，
 * 主要负责清理页面根上的冗余 section fill，
 * 尤其是模型常见的那种“安全深色背景”硬编码。
 *
 * 因为 OpenPencil 是多页文档，不能再写死 `DEFAULT_FRAME_ID` 去找页面根，
 * 所以这里会取活动页的所有顶级子节点，并对其中的顶级 frame 逐个处理。
 * 如果任何 frame 被改动，就统一触发一次 resync。
 */
function finalizePageRootAfterApply(): void {
  const doc = useDocumentStore.getState().document;
  const activePageId = useCanvasStore.getState().activePageId;
  const topLevel = getActivePageChildren(doc, activePageId);
  if (!topLevel || topLevel.length === 0) return;

  let anyChanged = false;
  for (const node of topLevel) {
    if (node.type !== 'frame') continue;
    if (stripRedundantSectionFills(node)) {
      anyChanged = true;
    }
  }
  if (anyChanged) forcePageResync();
}

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { getFlatNodes } = useDocumentStore.getState();
  const existingIds = new Set(getFlatNodes().map((n) => n.id));
  const preparedNodes = sanitizeNodesForInsert(nodes, existingIds);

  // 如果画布当前只有一个空框，就直接用生成结果替换它
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    resolveAllPendingIcons().catch(console.warn);
    // Use 活动页面的主框架 ID，NOT generationRootFrameId。 The
    // 后者是流路径所拥有的模块级状态，并且在这里是过时的（模块初始值或上一流生成的剩余值 - 在 Page 2+
    // 上，它不会指向当前页面上的任何内容）。
    const rootId = getActivePagePrimaryFrameId();
    if (rootId) scanAndFillImages(rootId).catch(() => {});
    return;
  }

  const { addNode } = useDocumentStore.getState();
  // 优先插入到活动页根框架里；
  // `getActivePagePrimaryFrameId()` 取代了过去只适用于第一页的 `DEFAULT_FRAME_ID` 查找。
  const parentId = getActivePagePrimaryFrameId();
  for (const node of preparedNodes) {
    addNode(parentId, node, Infinity);
  }
  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  resolveAllPendingIcons().catch(console.warn);
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
}

export function upsertNodesToCanvas(nodes: PenNode[]): number {
  const preparedNodes = sanitizeNodesForUpsert(nodes);

  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    return 1;
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  const parentId = getActivePagePrimaryFrameId();
  let count = 0;

  for (const node of preparedNodes) {
    // 解析被重映射过的 ID，例如 Phase 1 里被替换成真实根框架的节点
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id;
    const existing = getNodeById(resolvedId);
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node;
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode);
      updateNode(resolvedId, merged);
    } else {
      addNode(parentId, node, Infinity);
    }
    count++;
  }

  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  // 这里要用活动页真实根框架 ID，而不是流式路径里的 `generationRootFrameId`。
  // 对非流式 apply 来说，后者通常是陈旧状态。
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
  return count;
}

/** 与 `upsertNodesToCanvas` 相同，但跳过清理步骤（调用方已自行处理）。 */
function upsertPreparedNodes(preparedNodes: PenNode[]): number {
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    return 1;
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  const parentId = getActivePagePrimaryFrameId();
  let count = 0;

  for (const node of preparedNodes) {
    // 解析被重映射过的 ID，例如被替换成真实根框架的节点
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id;
    const existing = getNodeById(resolvedId);
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node;
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode);
      updateNode(resolvedId, merged);
    } else {
      addNode(parentId, node, Infinity);
    }
    count++;
  }

  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  return count;
}

/**
 * 把节点以交错淡入的方式加到画布上。
 * 节点本身是同步插入的，动画由 canvas-sync 异步驱动。
 */
export function animateNodesToCanvas(nodes: PenNode[]): void {
  resetGenerationRemapping();
  resetAnimationState();
  const prepared = sanitizeNodesForUpsert(nodes);
  startNewAnimationBatch();
  markNodesForAnimation(prepared);

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  upsertPreparedNodes(prepared);
  useHistoryStore.getState().endBatch(useDocumentStore.getState().document);

  // 把需要异步解析的图标（例如品牌 Logo）继续排队处理
  resolveAllPendingIcons().catch(console.warn);
  // 重新扫描活动页根节点下的图片占位符。
  // 虽然 `resetGenerationRemapping()` 会刷新 `generationRootFrameId`，
  // 但这里直接读活动页真实根节点更稳，也和其他非流式路径保持一致。
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
}

// ---------------------------------------------------------------------------
// Extract + 应用便利包装
// ---------------------------------------------------------------------------

/**
 * Extract
 * PenNode JSON 来自 AI 响应文本并应用于画布。 Returns 添加的顶级元素的数量（如果没有
 found/applied，则为 0）。
 */
export function extractAndApplyDesign(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText);
  if (!nodes || nodes.length === 0) return 0;

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  try {
    applyNodesToCanvas(nodes);
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
  }
  return nodes.length;
}

/**
 * Extract
 * PenNode JSON 来自 AI 响应文本并将 updates/insertions 应用于画布。 Handles 新节点和修改（由 ID 匹配）。
 */
export function extractAndApplyDesignModification(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText);
  if (!nodes || nodes.length === 0) return 0;

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  let count = 0;

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  try {
    for (const node of nodes) {
      const existing = getNodeById(node.id);
      if (existing) {
        // Update 现有节点
        updateNode(node.id, node);
        count++;
      } else {
        // It 是修改隐含的新节点（例如“添加按钮”）。 Parent 它到活动页面的根框架，无论我们在哪个页面，而不仅仅是 Page 1
        // 常量。
        const parentId = getActivePagePrimaryFrameId();
        addNode(parentId, node);
        count++;
      }
    }
    finalizePageRootAfterApply();
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
  }
  return count;
}

// ---------------------------------------------------------------------------
// Generation 启发式
// ---------------------------------------------------------------------------

/**
 * Lightweight
 * 解析后清理应用于每个节点。 Handles 图标路径解析、表情符号删除和图像占位符生成。
 * Layout/sizing 启发式现在由角色解析器处理。
 */
export function applyGenerationHeuristics(node: PenNode): void {
  // Skip 预注入的 chrome（例如 iPhone 状态栏）——其路径数据是从 Pencil
  // 演示中硬编码的，并且不得被图标解析器覆盖。
  if ('role' in node && (node as { role?: string }).role === 'status-bar') return;

  // Default icon_font 未指定时用于阐明族的节点
  if (node.type === 'icon_font' && !node.iconFontFamily) {
    node.iconFontFamily = 'lucide';
  }

  applyIconPathResolution(node);
  applyNoEmojiIconHeuristic(node);
  // Re-在从表情符号文本→路径转换的节点上运行图标解析
// heuristic above. applyNoEmojiIconHeuristic sets a circle fallback path;
  // 图标解析器通常可以与名称匹配（例如“Pizza Emoji Path”→ 披萨）。
  if (node.type === 'path') {
    applyIconPathResolution(node);
  }
  applyImagePlaceholderHeuristic(node);

  if (!('children' in node) || !Array.isArray(node.children)) return;
  for (const child of node.children) {
    applyGenerationHeuristics(child);
  }
}

/**
 * Post-流式树启发式
 * - 在子任务的所有节点都插入存储后应用树感知修复。 During 流式传输，节点是单独插入的（没有子节点），因此树感知启发式方法（例如按钮宽度扩展、
 *
 * 框架高度扩展和 clipContent 检测）会默默失败。 This
 * 函数在完成的子树上重新运
 * 行它们。
 */
export function applyPostStreamingTreeHeuristics(rootNodeId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState();
  const rootNode = getNodeById(rootNodeId);
  if (!rootNode || rootNode.type !== 'frame') return;
  if (!Array.isArray(rootNode.children) || rootNode.children.length === 0) return;

  // Schema 级规范化首先运行：展开数组包裹的笔划，将填充形状的笔划对象迁移到正确的
  // PenStroke，删除非法的“无”/“透明”CSS 关键字填充。 Sub-agents 不断打破这些约束，下游通道采用有效的形状。
  normalizeStrokeFillSchema(rootNode);

  // Earliest pass：剥离较弱的子代理在误读提示的电话模型指导时生成的虚假电话模型包装。 Must 运行 BEFORE
  // resolveTreeRoles，否则角色解析器可能会将默认值（布局、填充）写入包装器及其内部的子级，然后我们将其丢弃。 Return
  // 值被故意忽略 - 请参阅最后的发布步骤：我们总是发布，因此布尔值仅提供信息。
  unwrapFakePhoneMockups(rootNode);

  // 基于 Role 的树解析+跨节点 post-pass。 Runs FIRST
  // 因此角色默认值（例如导航栏 → 水平、按钮 → 水平）可以使用语义上正确的值填充缺失的 `layout` 字段。
  resolveTreeRoles(rootNode, generationCanvasWidth);
  resolveTreePostPass(rootNode, generationCanvasWidth, getNodeById, updateNode);

  // Re - 在运行任何后续传递之前从存储中获取根。 resolveTreePostPass 在多个地方调用
  // `updateNode`（高度扩展、clipContent）。 Each 调用通过 `updateNodeInTree`
  // 进行路由，它会浅克隆沿更新路径的每个祖先。 Our 原始 `rootNode` 引用现在指向一棵独立的树：对于位于这些更新路径上的节点来说
  // ，其上的进一步突变将悄然消失。 Always 阅读了 updateNode 之后的突变传递的新参考。
  const freshRoot = useDocumentStore.getState().getNodeById(rootNodeId);
  if (!freshRoot || freshRoot.type !== 'frame') return;

  // Normalize 布局作为最终的安全网：为角色解析器未触及的框架（未知角色、普通容器）填充
  // `layout`，并从任何自动布局框架的子级中删除陈旧的 x/y。 MUST run AFTER 角色解析 —
  // 否则，此处的“垂直”回退会在角色默认值覆盖错误布局之前冻结它们。
  normalizeTreeLayout(freshRoot);

  // Strip 冗余节级填充。 Weaker 子代理对冲
  // 在每个节根上硬编码一个“安全暗”十六进制（例如#0a0a0a）
  // 发出，然后完全覆盖页面根目录的预期背景
  // 并打破主题切换。 This pass 会删除那些多余的填充，而
  // 保留 cards/buttons/badges。 Must 运行 AFTER 角色解析所以我们
  // 可以通过以下方式区分节容器和 card/button/chip 组件
  // 他们坚定的角色。
//
  // IMPORTANT: stripRedundantSectionFills 必须在 true 上调用 ONLY
  // 页面根框架。 Calling 它在任意子代理根（或任何
  // 非根嵌套框架）是错误的 - 嵌套框架的直接子级
  // 是组件，而不是“部分”，并且剥离它们的填充会
  // 破坏预期的视觉样式（例如卡片自己的深色标题）。
//
  // The 页面根目录是：
  //   - `parentOfRoot` 当子代理的根作为子代理插入时
  // 现有的页面框架（多部分计划的常见情况）
  //   - `freshRoot` 本身当子代理的根 IS 的页框
  // （replaceEmptyFrame 重新映射，或单子代理页面）
  // Pick 恰好是一个——绝不会两者兼而有之。
  const parentOfRoot = useDocumentStore.getState().getParentOf(rootNodeId);
  const pageRoot = parentOfRoot && parentOfRoot.type === 'frame' ? parentOfRoot : freshRoot;
  stripRedundantSectionFills(pageRoot);

  // Publish 点。展开、resolveTreeRoles 和 normalizeTreeLayout 全部
  // 就地改变商店拥有的节点； resolveTreePostPass 大部分时间都去
  // 通过 updateNode 但也有直接突变分支。 Without 一个
  // 显式发布，Zustand 订阅者（画布同步，MCP 推送）仅触发
// if some later code path happens to call updateNode — and that path is
  // 在子代理重试/无操作情况下跳过。
//
  // We 使用 forcePageResync （不是手卷的浅文档传播），因为
  // canvas-document-sync 订阅活动页面的子数组
  // 身份，而不是文档对象本身。天真的 `{ ...document }`
  // 传播会改变 NOT 而画布永远不会改变
  // 重新同步 — 请参阅 canvas-sync-utils.ts 陷阱头注释。
  // forcePageResync 也会绕过 mutateWithHistory 所以我们不会推送
  // 确定性流后清理的撤消条目。
  forcePageResync();

  // Resolve 通过 Iconify API 异步挂起图标（即发即忘）
  resolveAsyncIcons(rootNodeId).catch(console.warn);
}

// ---------------------------------------------------------------------------
// Root 框架高度管理
// ---------------------------------------------------------------------------

export function adjustRootFrameHeightToContent(frameId?: string): void {
  const { getNodeById, updateNode, getParentOf } = useDocumentStore.getState();
  // Prefer 显式传递的帧，然后是活动页面的主框架（非流应用路径的正确默认值，这是调用此函数的位置），最后是流路径的
  // generationRootFrameId 作为最后的手段。
  const rootId = frameId ?? getActivePagePrimaryFrameId() ?? generationRootFrameId;
  if (!rootId) return;
  const root = getNodeById(rootId);
  if (!root || root.type !== 'frame') return;
  if (!Array.isArray(root.children) || root.children.length === 0) return;

  const measurableRoot = { ...root, height: 0 } as typeof root;
  const requiredHeight = estimateNodeIntrinsicHeight(measurableRoot);
  const minimumHeight = getParentOf(rootId) ? 0 : 320;
  const targetHeight = Math.max(minimumHeight, Math.round(requiredHeight));
  const currentHeight = toSizeNumber(root.height, 0);
  if (Math.abs(currentHeight - targetHeight) < 8) return;

  updateNode(rootId, { height: targetHeight });
}

/**
 * adjustRootFr
 * ameHeightToContent 的仅 Expand 版本。 Used
 * 在流式传输期间：仅增大根框架，从不缩小它。 This 在逐步添加部分时可防止视觉抖动。 When
 *
 * 将一个框架插入到水平布局父级中，检查兄弟框架子级是否应等于 fill_container 以防止溢出。 This 运行 DURING
 * 流，因此卡片到达时均匀分
 * 布。
 */
export function expandRootFrameHeight(frameId?: string): void {
  const { getNodeById, updateNode, getParentOf } = useDocumentStore.getState();
  const rootId = frameId ?? generationRootFrameId;
  const root = getNodeById(rootId);
  if (!root || root.type !== 'frame') return;
  if (!Array.isArray(root.children) || root.children.length === 0) return;

  const measurableRoot = { ...root, height: 0 } as typeof root;
  const requiredHeight = estimateNodeIntrinsicHeight(measurableRoot);
  const minimumHeight = getParentOf(rootId) ? 0 : 320;
  const targetHeight = Math.max(minimumHeight, Math.round(requiredHeight));
  const currentHeight = toSizeNumber(root.height, 0);
  // Only 增长——在渐进生成过程中从不收缩
  if (currentHeight > 0 && targetHeight <= currentHeight) return;

  updateNode(rootId, { height: targetHeight });
}

// ---------------------------------------------------------------------------
// Internal 帮助者
// ---------------------------------------------------------------------------

/**
 * Check 如果活动页面
 * 恰好有一个顶级框架并且该框架还没有子级。 Used 来决定传入的 batch/streaming 插入是否应该 REPLACE 由
 * addPage() 创建的空样板框架，而不是在其旁边附加新内容。 Previously 这个硬编码的
 * DEFAULT_FRAME_ID，在第一个页面之后的每个页面上都会损坏：addPage() 为新页面提供基于 nanoid 的根框架
 *
 * id，因此在 Page 2+ 上检查为
 * `false`，并且替换
 * 分支从未触发。 The 检查现在查看活动页面的实际顶级框架，无论其 id 是什么。
 *
 *
 */
function isCanvasOnlyEmptyFrame(): boolean {
  const { document } = useDocumentStore.getState();
  const activePageId = useCanvasStore.getState().activePageId;
  const pageChildren = getActivePageChildren(document, activePageId);
  if (pageChildren.length !== 1) return false;
  const only = pageChildren[0];
  if (only.type !== 'frame') return false;
  return !('children' in only) || !only.children || only.children.length === 0;
}

/**
 * Replace
 * 活动页面的空根框架与生成的框架节点，保留现有的框架 ID，以便画布同步继续工作。 Returns 已更新的框架的
 * id，如果活动页面没有要替换的框架，则为 null（调用者应该在 isCanvasOnlyEmptyFrame 上门控此调用）。
 * Previously 这个硬编码的 DEFAULT_FRAME_ID 作为更新目标，这意味着在 Page 2+ 上调用
 * replaceEmptyFrame 会默默地修改 Page 1 的根框架，而不是用户实际编辑的页面。
 *
 *
 *
 *
 */
function replaceEmptyFrame(generatedFrame: PenNode): string | null {
  const targetId = getActivePagePrimaryFrameId();
  if (!targetId) return null;
  const { updateNode } = useDocumentStore.getState();
  // Record 重新映射，以便后续阶段可以通过其原始 ID 找到该节点
  generationRemappedIds.set(generatedFrame.id, targetId);
  // Keep 根帧 ID 和位置 (x=0, y=0)，从生成的帧中获取其他所有内容
  const { id: _id, x: _x, y: _y, ...rest } = generatedFrame;
  updateNode(targetId, rest);
  return targetId;
}

function equalizeHorizontalSiblings(parentId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState();
  const parent = getNodeById(parentId);
  if (!parent || parent.type !== 'frame') return;
  if (parent.layout !== 'horizontal') return;
  if (!Array.isArray(parent.children) || parent.children.length < 2) return;

  // Skip 如果任何卡已经使用 fill_container —— AI 故意选择它
  const cardCandidates = parent.children.filter(
    (c) =>
      c.type === 'frame' &&
      c.role !== 'phone-mockup' &&
      c.role !== 'divider' &&
      c.role !== 'badge' &&
      c.role !== 'pill' &&
      c.role !== 'tag' &&
      toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  );
  if (cardCandidates.some((c) => 'width' in c && c.width === 'fill_container')) return;

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  );
  if (fixedFrames.length < 2) return;

  // Only 当宽度变化很大时均衡（比率 < 0.6）
  const widths = fixedFrames.map((c) => toSizeNumber('width' in c ? c.width : undefined, 0));
  const maxW = Math.max(...widths);
  const minW = Math.min(...widths);
  if (maxW <= 0 || minW / maxW >= 0.6) return;

  // Check 如果它们看起来像一排卡片（相似高度）
  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0));
  const maxH = Math.max(...heights);
  const minH = Math.min(...heights);
  if (maxH <= 0 || minH / maxH <= 0.5) return;

  // Convert 至 fill_container 实现均匀分布和相等高度
  for (const child of fixedFrames) {
    updateNode(child.id, { width: 'fill_container', height: 'fill_container' } as Partial<PenNode>);
  }
}

function applyImagePlaceholderHeuristic(node: PenNode): void {
  if (node.type !== 'image') return;

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase();
  const contextMarker = generationContextHint.toLowerCase();
  const contextualScreenshotHint = /(截图|screenshot|mockup|手机|app[-_\s]*screen)/.test(
    contextMarker,
  );
  const screenshotLike =
    isScreenshotLikeMarker(marker) ||
    (contextualScreenshotHint && /(preview|hero|showcase|phone|screen)/.test(marker));
  if (!screenshotLike) return;

  const width = toSizeNumber(node.width, 360);
  const height = toSizeNumber(node.height, 720);
  // Detect dark/light from context hint (dark if mentions dark/terminal/cyber/night)
  const dark = !/(light|bright)/.test(generationContextHint.toLowerCase());
  node.src = createPhonePlaceholderDataUri(width, height, dark);
  if (node.cornerRadius === undefined) {
    node.cornerRadius = 24;
  }
}

function isScreenshotLikeMarker(text: string): boolean {
  return /app[-_\s]*screen|screenshot|mockup|phone|mobile|device|截图|手机/.test(text);
}

// ---------------------------------------------------------------------------
// Node sanitization for insert/upsert
// ---------------------------------------------------------------------------

/**
 * Resolve the theme that should drive role defaults for an incoming
 * batch of nodes.
 *
 * Detection order — INPUT NODES FIRST, then live store:
 *
 *   1. Walk the incoming `nodes` array top-down. The first frame at
 *      depth 0 (outermost) with a solid-color fill wins. If none of
 *      the outermost nodes has a fill, walk one level deeper, and so
 *      on. The first hit is the theme source.
 *
 *      Why input first: in a fresh generation the LLM emits the new
 *      page root (e.g. fill #0A0A0A) inside `nodes`, but the LIVE
 *      store still holds the previous empty default root. Reading
 *      the store would return 'light' from that empty default, and
 *      the LLM-supplied dark page would get white card defaults
 *      injected into every child before the new root reaches the
 *      store. Reading the input first guarantees the cards see the
 *      same theme as the page they belong to.
 *
 *   2. Fall back to the LIVE active-page primary frame in the store
 *      via `getActivePagePrimaryFrameId()`. This handles partial
 *      inserts (e.g. dropping a single navbar into an existing dark
 *      page where `nodes` doesn't carry the page root).
 *
 *      Always reads via `getActivePagePrimaryFrameId()` rather than
 *      the cached `generationRootFrameId` module variable. The cache
 *      is set by `resetGenerationRemapping()` at the start of an
 *      orchestrator generation flow but is stale or default for
 *      direct MCP call paths (`insert_node`, `batch_design`,
 *      `upsertNodesToCanvas` from non-streaming code) that bypass
 *      that initialization. The same precedent exists at line ~464
 *      of this file: `upsertNodesToCanvas` already reads
 *      `getActivePagePrimaryFrameId()` for the same reason.
 *
 *   3. Returns `undefined` when neither source has a usable fill
 *      (brand-new document, partial insert into empty page) —
 *      callers should treat that the same as the default light theme.
 */
function detectActiveDocumentTheme(nodes?: PenNode[]): 'dark' | 'light' | undefined {
  if (nodes && nodes.length > 0) {
    const fromInput = detectThemeFromNodeForest(nodes);
    if (fromInput) return fromInput;
  }

  const primaryFrameId = getActivePagePrimaryFrameId();
  if (!primaryFrameId) return undefined;
  const pageRoot = useDocumentStore.getState().getNodeById(primaryFrameId);
  if (!pageRoot) return undefined;
  return detectThemeFromNode(pageRoot);
}

/**
 * BFS over a forest of nodes, returning the theme detected from the
 * first frame with a usable solid fill. Returns `undefined` if no
 * frame in the entire forest carries a fill we can read.
 *
 * BFS (not DFS) so the OUTERMOST frames are visited first — the page
 * root and top-level sections are the most authoritative theme
 * source. A small white card nested deep inside a dark page must not
 * out-vote the page root.
 */
function detectThemeFromNodeForest(nodes: PenNode[]): 'dark' | 'light' | undefined {
  const queue: PenNode[] = [...nodes];
  while (queue.length > 0) {
    const node = queue.shift()!;
    if (node.type === 'frame') {
      const theme = readThemeFromNodeFill(node);
      if (theme) return theme;
    }
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) queue.push(child);
    }
  }
  return undefined;
}

/**
 * Read theme from a single node's fill if it has a parseable solid
 * color. Returns `undefined` for missing fill, empty fill, gradient,
 * variable ref, or any other unreadable shape — caller must keep
 * walking the tree.
 *
 * Mirrors `detectThemeFromNode` from role-resolver but returns
 * undefined (not 'light') when the fill is unreadable, so the caller
 * can distinguish "no fill found, keep looking" from "explicit light".
 */
function readThemeFromNodeFill(node: PenNode): 'dark' | 'light' | undefined {
  const fill = (node as { fill?: unknown }).fill;
  if (!Array.isArray(fill) || fill.length === 0) return undefined;
  const first = fill[0] as { type?: string; color?: string };
  if (first?.type !== 'solid' || typeof first.color !== 'string') return undefined;
  return detectThemeFromNode(node);
}

function sanitizeNodesForInsert(nodes: PenNode[], existingIds: Set<string>): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n));
  const activeTheme = detectActiveDocumentTheme(cloned);

  for (const node of cloned) {
    // Schema normalization first so later passes see valid stroke/fill
    // shapes (unwrap stroke arrays, migrate fill-shaped strokes, drop
    // CSS-keyword fill colors).
    normalizeStrokeFillSchema(node);
    // Strip fake phone mockup wrappers BEFORE role resolution so role
    // defaults aren't wasted on a wrapper we're about to discard.
    unwrapFakePhoneMockups(node);
    // Rewrite known LLM composition anti-patterns BEFORE role resolution
    // so the rewritten subtree still benefits from theme-aware defaults,
    // layout normalization, and post-pass fixes. Covers stacked-ellipse
    // progress rings rendering as overlapping top-left blobs, and
    // alternating bar/label siblings that break chart column layouts.
    rewriteLlmAntiPatterns(node);
    // Role resolution runs first so role defaults can populate `layout`
    // before normalizeTreeLayout's generic fallback would otherwise freeze
    // the wrong value (e.g. navbar → horizontal, not vertical fallback).
    //
    // `activeTheme` is detected from the LIVE page root (not from `node`)
    // because `node` here is an arbitrary subtree without its own fill —
    // a card or navbar that omitted fill expecting the dark page bg to
    // show through. Without this, theme detection would fall back to
    // 'light' and paint a white default on top of a dark page.
    resolveTreeRoles(
      node,
      generationCanvasWidth,
      undefined,
      undefined,
      undefined,
      false,
      activeTheme,
    );
    applyGenerationHeuristics(node);
    normalizeTreeLayout(node);
    // Intentionally NOT calling stripRedundantSectionFills here: `cloned`
    // is an arbitrary PenNode from MCP/batch APIs (could be a card, a
    // component, or a page). strip must only run on the true page root
    // frame, which this path cannot guarantee.
    sanitizeLayoutChildPositions(node, false);
    sanitizeScreenFrameBounds(node);
  }

  const counters = new Map<string, number>();
  const used = new Set(existingIds);
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters);
  }

  return cloned;
}

function sanitizeNodesForUpsert(nodes: PenNode[]): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n));
  const activeTheme = detectActiveDocumentTheme(cloned);

  for (const node of cloned) {
    // Schema normalization first so later passes see valid stroke/fill
    // shapes (unwrap stroke arrays, migrate fill-shaped strokes, drop
    // CSS-keyword fill colors).
    normalizeStrokeFillSchema(node);
    // Strip fake phone mockup wrappers BEFORE role resolution so role
    // defaults aren't wasted on a wrapper we're about to discard.
    unwrapFakePhoneMockups(node);
    // Rewrite known LLM composition anti-patterns BEFORE role resolution.
    // See sanitizeNodesForInsert for the full rationale.
    rewriteLlmAntiPatterns(node);
    // Role resolution runs first so role defaults can populate `layout`
    // before normalizeTreeLayout's generic fallback would otherwise freeze
    // the wrong value (e.g. navbar → horizontal, not vertical fallback).
    // See sanitizeNodesForInsert for the activeTheme rationale.
    resolveTreeRoles(
      node,
      generationCanvasWidth,
      undefined,
      undefined,
      undefined,
      false,
      activeTheme,
    );
    applyGenerationHeuristics(node);
    normalizeTreeLayout(node);
    // Intentionally NOT calling stripRedundantSectionFills here: `cloned`
    // is an arbitrary PenNode from MCP/batch APIs (could be a card, a
    // component, or a page). strip must only run on the true page root
    // frame, which this path cannot guarantee.
    sanitizeLayoutChildPositions(node, false);
    sanitizeScreenFrameBounds(node);
  }

  // Start with pre-existing node IDs to avoid collisions with content
  // that was on canvas before this generation started. IDs generated
  // within the current batch are also tracked so siblings stay unique.
  // Record remappings so progressive upsert can resolve renamed IDs.
  const counters = new Map<string, number>();
  const used = new Set(preExistingNodeIds);
  const newRemaps = new Map<string, string>();
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters, newRemaps);
  }

  // Merge new remappings into the generation-wide remap table
  for (const [from, to] of newRemaps) {
    generationRemappedIds.set(from, to);
  }

  return cloned;
}

/** Check if a node (by ID) is inside a Phone Placeholder frame (any ancestor). */
function isInsidePhonePlaceholder(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
): boolean {
  let current = getNodeById(nodeId);
  while (current) {
    if (current.name === 'Phone Placeholder') return true;
    const parent = useDocumentStore.getState().getParentOf(current.id);
    if (!parent) break;
    current = parent;
  }
  return false;
}
