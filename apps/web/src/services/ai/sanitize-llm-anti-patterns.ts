/**
 * Structural
 *
 * 重写已知的 LLM 生成反模式。 Weaker 模型（有时是强模型）会重现一些破碎的构图模式，无论提示如何仔细地描述正确
 * 的方法。 Prompt
 * 级别的教育是不可靠的 - LLMs 从训练数据中复制最常见的模式，而不是指令头中的规则。 This
 * 模块在清理时捕获重复出现的反模式，并将子树重写为结构正确的形式 BEFORE `resolveTreeRoles` /
 * `normalizeTreeLayout` 运行。 Design 原理： - Pure 树突变。 No 在传递的节点之外产生副作用。 -
 * Conservative 检测。 Each 重写器在触发之前需要多个信号，因此它不会触及未损坏的设计。 - Structural
 * 替换，而不是属性编辑。 The 损坏的模式一步即可替换为正确的等效模式 - 没有可能使树处于部分重写状态的零碎修复。 - Runs
 *
 * BEFORE 角色解析，因此重写的子树仍然可以从下游传递中受益（主题感知默认值、布局规范化、传递后徽章覆盖检测）。 Current
 * 重写器：rewriteStackedEllipsesToRingFrames 一个 `layout: 'none'`
 * 父级，包含 ≥2
 * 个同心椭圆（透明填充，仅描边）加上任
 * 意数量的文本/ icon_font 子级。 The LLM 意图是同心环组合 (Apple Activity
 * Rings)，但当布局=none 并且椭圆上没有 x/y 时，它们都在 (0,0) 处渲染并重叠在左上角。 Rewritten
 * 进入嵌套框架 + cornerRadius 树，其中每个框架通过 Flex 在其父框架内居中。
 * rewriteAlternatingBarLabelSiblings 一个水平父母，有交替的 `frame(no-text
 * children)` + `text` 兄弟姐妹（总共 ≥6 个孩子，≥3 对）。 The LLM
 * 意图是一个条形图，每个条形下方都有标签，但在 `space_between`
 *
 * 下将它们作为平面兄弟发出会产生分散在条形之间的标签，而不是分组在列中。 Rewritten 将每个条形与其后面的文本配对到
 *
 * `layout=vertical` 列框架中。 rewriteOpenStrokePathsWithDuplicateF
 * ill Weak 模型通常在 `fill` 和 `stroke`
 * 中发出具有相同纯色的折线
 * 图/迷你图路径，但忘记用 `Z`
 * 关闭 SVG 路径。
 * SVG fill 隐式关闭开放路径，将简单的趋势线变成
 * wedge/area 斑点。 When 路径显然是一条开放的描边线，填充复制了描边颜色，删除填充并仅保留描边。
 * normalizeRingTrackProgressGeometry Weak
 *
 * 模型有时会从椭圆轨道加上同级路径弧构建进度环，但该路径使用不同的 bbox（例如，在 x=5,y=5 处的
 * 80×80 轨道上的 90×90 路径）。 That 不匹配使环在路径 normalization/scaling
 * 之后看起来被拉伸或偏离中心。 When 我们检测到这种模式，将进度路径重写到轨道的本地 bbox 上并保留
 * dash/cap 样式。 rewritePseudoRingFrames Some
 * 较弱的模型会生成一个人造环作为宽药丸框架，其中包含方形“Progress
 * Ring”子项和较小的“
 * Inner Circle”子项。 The
 * 外框变成一个拉伸的胶囊而不是圆形。 When 检测到此模式，将外框重写到方形进度 bbox
 *
 * 上，删除虚假的白色胶囊填充，并将内部内容居中，以便结果至少是一个稳定的圆环。 stripRingFrameFills Ring
 * 框架是笔画几何体。 Downstream role/post 通过，某些型号可以通过添加白色或中性填充物将它们变成填充圆盘。
 * For 名为 ring/circle/progress 的方形、圆形、描边节点，删除内部填充，同时保留描边和子项。
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 */

import type { PenNode } from '@/types/pen';

// ---------------------------------------------------------------------------
// Shared 帮助者
// ---------------------------------------------------------------------------

function getChildren(node: PenNode): PenNode[] | undefined {
  if (!('children' in node) || !Array.isArray(node.children)) return undefined;
  return node.children as PenNode[];
}

function setChildren(node: PenNode, children: PenNode[]): void {
  (node as PenNode & { children: PenNode[] }).children = children;
}

function getNum(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

/** Counter 用于为新创建的包装框架创建确定性 ID。 */
let _rewriteCounter = 0;
function mintId(prefix: string): string {
  _rewriteCounter += 1;
  return `${prefix}-${_rewriteCounter}`;
}

// ---------------------------------------------------------------------------
// Rewriter 1: stacked concentric ellipses → nested ring frames
// ---------------------------------------------------------------------------

interface StackedEllipsesMatch {
  ringsFromLargest: PenNode[];
  textContents: PenNode[];
}

/**
 * Detect the "stacked concentric ellipses" pattern.
 *
 * Signals required (all must hold):
 *   - Node has explicit `layout: 'none'` (LLM opted out of flex)
 *   - Node has ≥2 children that are stroke-only ellipses (fill
 *     transparent or missing) with no x/y set
 *   - The ellipses are distinctly sized (no two share both width and
 *     height) — rules out tiled grids of identical circles
 *   - All other children (if any) are text or icon_font — the center
 *     label
 *   - No child carries x/y (if any did, the LLM was clearly trying
 *     absolute positioning and we must not second-guess it)
 */
function matchStackedEllipses(node: PenNode): StackedEllipsesMatch | null {
  if (node.type !== 'frame') return null;
  const layout = (node as { layout?: string }).layout;
  if (layout !== 'none') return null;
  const children = getChildren(node);
  if (!children || children.length < 2) return null;

  const ellipses: PenNode[] = [];
  const textContents: PenNode[] = [];
  for (const child of children) {
    const c = child as PenNode & { x?: unknown; y?: unknown };
    if (typeof c.x === 'number' || typeof c.y === 'number') return null;

    if (child.type === 'ellipse') {
      if (!isStrokeOnlyEllipse(child)) return null;
      ellipses.push(child);
      continue;
    }
    if (child.type === 'text' || child.type === 'icon_font') {
      textContents.push(child);
      continue;
    }
    // Any other child type disqualifies — we don't know what it is.
    return null;
  }

  if (ellipses.length < 2) return null;

  // Ensure distinct sizes. If two ellipses share dimensions the LLM
  // was probably drawing repeated circles (dots in a row), not
  // concentric rings — don't rewrite.
  const sizeKeys = new Set(
    ellipses.map(
      (e) =>
        `${getNum((e as { width?: unknown }).width) ?? 0}x${getNum((e as { height?: unknown }).height) ?? 0}`,
    ),
  );
  if (sizeKeys.size !== ellipses.length) return null;

  // Sort largest → smallest so callers can nest outer-first.
  const ringsFromLargest = [...ellipses].sort((a, b) => ringArea(b) - ringArea(a));
  return { ringsFromLargest, textContents };
}

function isStrokeOnlyEllipse(node: PenNode): boolean {
  const stroke = (node as { stroke?: { thickness?: unknown } }).stroke;
  const strokeThickness = typeof stroke?.thickness === 'number' ? stroke.thickness : 0;
  if (strokeThickness <= 0) return false;

  // Fill must be missing, empty array, or a fully-transparent solid.
  const fill = (node as { fill?: unknown }).fill;
  if (fill == null) return true;
  if (Array.isArray(fill) && fill.length === 0) return true;
  if (Array.isArray(fill) && fill.length === 1) {
    const f = fill[0] as { type?: string; color?: string };
    if (f?.type === 'solid' && typeof f.color === 'string') {
      const color = f.color.trim();
      // Explicit transparent: #00000000 or 8-digit hex ending in 00
      const m = color.match(/^#([0-9a-fA-F]{8})$/);
      if (m && m[1].slice(6, 8).toLowerCase() === '00') return true;
    }
  }
  return false;
}

function ringArea(node: PenNode): number {
  const w = getNum((node as { width?: unknown }).width) ?? 0;
  const h = getNum((node as { height?: unknown }).height) ?? 0;
  return w * h;
}

/**
 * Rewrite a matched stacked-ellipses parent into a nested ring-frame
 * composition. Returns the new node (same id as the original parent)
 * so callers can swap it in place. Mutates nothing on the input.
 *
 * Nesting strategy: each ring becomes a frame with cornerRadius=w/2,
 * stroke copied from the ellipse, and `layout: 'horizontal'` +
 * `alignItems: 'center'` + `justifyContent: 'center'` so every child
 * is auto-centered. The outermost ring wraps the next, and so on. The
 * innermost ring contains the text/icon center label, if any.
 */
function rewriteToNestedRingFrames(original: PenNode, match: StackedEllipsesMatch): PenNode {
  const outerW = getNum((original as { width?: unknown }).width);
  const outerH = getNum((original as { height?: unknown }).height);

  // Build from innermost outward so we can feed `innerChildren` up.
  // Start with the text/icon label children as the innermost content.
  let innerChildren: PenNode[] = match.textContents.map((t) => ({ ...(t as object) }) as PenNode);

  // Iterate rings smallest-to-largest, wrapping each layer.
  const ringsFromSmallest = [...match.ringsFromLargest].reverse();
  for (const ring of ringsFromSmallest) {
    const w = getNum((ring as { width?: unknown }).width) ?? 0;
    const h = getNum((ring as { height?: unknown }).height) ?? 0;
    const cornerRadius = Math.max(w, h) / 2;
    const stroke = (ring as { stroke?: unknown }).stroke;
    const wrapper: PenNode = {
      id: ring.id || mintId('ring'),
      type: 'frame',
      name: (ring as { name?: string }).name,
      width: w,
      height: h,
      cornerRadius,
      fill: [],
      stroke,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: innerChildren,
    } as unknown as PenNode;
    innerChildren = [wrapper];
  }

  // Now `innerChildren` is a single-element array containing the
  // outermost ring frame. Return a new parent of the same id that
  // centers the outermost ring.
  return {
    id: original.id,
    type: 'frame',
    name: (original as { name?: string }).name,
    width: outerW ?? undefined,
    height: outerH ?? undefined,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: innerChildren,
  } as unknown as PenNode;
}

/**
 * Walk the tree and rewrite every stacked-ellipses subtree in place.
 * Mutates the passed node.
 */
export function rewriteStackedEllipsesToRingFrames(node: PenNode): void {
  const match = matchStackedEllipses(node);
  if (match) {
    const rewritten = rewriteToNestedRingFrames(node, match);
    // Copy rewritten props onto the original node to preserve parent's
    // child array reference. Walk the original's keys and null them
    // (except id), then copy rewritten's keys.
    const original = node as unknown as Record<string, unknown>;
    const rewrittenRec = rewritten as unknown as Record<string, unknown>;
    for (const key of Object.keys(original)) {
      if (key === 'id') continue;
      delete original[key];
    }
    for (const [key, value] of Object.entries(rewrittenRec)) {
      if (key === 'id') continue;
      original[key] = value;
    }
    // Recurse into the rewritten children too (in case of nested
    // compositions — unlikely but free safety).
    const rewrittenChildren = getChildren(node);
    if (rewrittenChildren) {
      for (const child of rewrittenChildren) rewriteStackedEllipsesToRingFrames(child);
    }
    return;
  }

  const children = getChildren(node);
  if (!children) return;
  for (const child of children) rewriteStackedEllipsesToRingFrames(child);
}

// ---------------------------------------------------------------------------
// Rewriter 2: alternating bar/label siblings → grouped columns
// ---------------------------------------------------------------------------

/**
 * Detect the "alternating bar+label siblings" pattern.
 *
 * Signals required:
 *   - Parent has `layout: 'horizontal'`
 *   - ≥6 children (≥3 pairs)
 *   - Children alternate strictly: frame, text, frame, text, …
 *     ending on text (even count)
 *   - Each frame child has no text children of its own (it's a bar,
 *     not a card with a title)
 *
 * Non-signals (intentionally permissive):
 *   - Parent's justifyContent doesn't matter (could be space_between,
 *     space_around, even start — bar charts are broken either way)
 *   - Bar sizes don't have to match (a varied bar chart is the
 *     common case)
 */
function matchAlternatingBarLabel(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  const layout = (node as { layout?: string }).layout;
  if (layout !== 'horizontal') return false;
  const children = getChildren(node);
  if (!children || children.length < 6 || children.length % 2 !== 0) return false;

  for (let i = 0; i < children.length; i++) {
    const c = children[i];
    const expected = i % 2 === 0 ? 'frame' : 'text';
    if (c.type !== expected) return false;
    if (expected === 'frame' && frameHasTextDescendant(c)) return false;
  }
  return true;
}

function frameHasTextDescendant(node: PenNode): boolean {
  const children = getChildren(node);
  if (!children) return false;
  for (const child of children) {
    if (child.type === 'text') return true;
    if (frameHasTextDescendant(child)) return true;
  }
  return false;
}

/**
 * Rewrite the parent's children array: each (frame, text) pair becomes
 * a new `layout=vertical` column frame holding both. The parent keeps
 * its `layout=horizontal` and whatever justifyContent it already had;
 * the column wrappers inherit `alignItems=center` for label under bar.
 */
function rewriteBarLabelToColumns(parent: PenNode): void {
  const children = getChildren(parent);
  if (!children) return;
  const columns: PenNode[] = [];
  for (let i = 0; i < children.length; i += 2) {
    const bar = children[i];
    const label = children[i + 1];
    const column: PenNode = {
      id: mintId(`${parent.id}-col`),
      type: 'frame',
      name: (label as { content?: string }).content
        ? `${String((label as { content?: string }).content).trim()} Column`
        : 'Bar Column',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'vertical',
      alignItems: 'center',
      gap: 8,
      children: [bar, label],
    } as unknown as PenNode;
    columns.push(column);
  }
  setChildren(parent, columns);
}

/**
 * Walk the tree and rewrite every alternating-bar-label subtree in
 * place. Mutates the passed node.
 */
export function rewriteAlternatingBarLabelSiblings(node: PenNode): void {
  if (matchAlternatingBarLabel(node)) {
    rewriteBarLabelToColumns(node);
  }
  const children = getChildren(node);
  if (!children) return;
  for (const child of children) rewriteAlternatingBarLabelSiblings(child);
}

// ---------------------------------------------------------------------------
// Rewriter 3: open stroke paths with duplicate fill → stroke only
// ---------------------------------------------------------------------------

function normalizeSolidColor(color: unknown): string | null {
  return typeof color === 'string' ? color.trim().toLowerCase() : null;
}

function shouldDropDuplicateOpenPathFill(node: PenNode): boolean {
  if (node.type !== 'path') return false;

  const d = typeof node.d === 'string' ? node.d : '';
  if (!d || /[zZ]/.test(d)) return false;

  const stroke = node.stroke;
  if (!stroke || typeof stroke.thickness !== 'number' || stroke.thickness <= 0) return false;
  if (!Array.isArray(stroke.fill) || stroke.fill.length === 0) return false;

  const strokeFill = stroke.fill[0];
  const fill = node.fill;
  if (!Array.isArray(fill) || fill.length !== 1) return false;

  const solidFill = fill[0];
  if (strokeFill?.type !== 'solid' || solidFill?.type !== 'solid') return false;

  const strokeColor = normalizeSolidColor(strokeFill.color);
  const fillColor = normalizeSolidColor(solidFill.color);
  if (!strokeColor || !fillColor || strokeColor !== fillColor) return false;

  return true;
}

export function rewriteOpenStrokePathsWithDuplicateFill(node: PenNode): void {
  if (shouldDropDuplicateOpenPathFill(node)) {
    delete (node as PenNode & { fill?: unknown }).fill;
  }

  const children = getChildren(node);
  if (!children) return;
  for (const child of children) rewriteOpenStrokePathsWithDuplicateFill(child);
}

// ---------------------------------------------------------------------------
// Rewriter 4: normalize ring track/progress geometry
// ---------------------------------------------------------------------------

function isEllipseTrack(node: PenNode): boolean {
  return node.type === 'ellipse' && /track/i.test(node.name ?? '');
}

function isProgressArcPath(node: PenNode): boolean {
  return node.type === 'path' && /(progress|arc)/i.test(node.name ?? '');
}

function normalizeRingProgressPath(track: PenNode, progress: PenNode): void {
  if (track.type !== 'ellipse' || progress.type !== 'path') return;

  const w = getNum(track.width) ?? 0;
  const h = getNum(track.height) ?? 0;
  if (w <= 0 || h <= 0) return;

  const rx = w / 2;
  const ry = h / 2;
  const startX = rx;
  const startY = 0;
  const endX = Math.max(0, rx - 0.01);
  const endY = 0;

  progress.width = w;
  progress.height = h;
  progress.x = track.x ?? 0;
  progress.y = track.y ?? 0;
  progress.d = `M ${f(startX)} ${f(startY)} A ${f(rx)} ${f(ry)} 0 1 1 ${f(endX)} ${f(endY)}`;
}

export function normalizeRingTrackProgressGeometry(node: PenNode): void {
  const children = getChildren(node);
  if (!children || children.length < 2) return;

  if (
    node.type === 'frame' &&
    ((node as { layout?: string }).layout === 'none' || !(node as { layout?: string }).layout)
  ) {
    const track = children.find(isEllipseTrack);
    const progress = children.find(isProgressArcPath);
    if (track && progress) {
      normalizeRingProgressPath(track, progress);
    }
  }

  for (const child of children) normalizeRingTrackProgressGeometry(child);
}

function f(n: number): string {
  return Math.abs(n) < 0.005 ? '0' : parseFloat(n.toFixed(2)).toString();
}

// ---------------------------------------------------------------------------
// Rewriter 5: pill-shaped pseudo-rings -> stable circular rings
// ---------------------------------------------------------------------------

function matchPseudoRingFrame(node: PenNode): {
  progress: PenNode;
  inner: PenNode;
  size: number;
} | null {
  if (node.type !== 'frame') return null;
  const width = getNum(node.width);
  const height = getNum(node.height);
  const radius = getNum((node as { cornerRadius?: unknown }).cornerRadius);
  if (!width || !height || !radius) return null;
  if (width <= height * 1.25) return null;
  if (Math.abs(radius - height / 2) > 2) return null;

  const children = getChildren(node);
  if (!children || children.length < 2) return null;

  const progress = children.find(
    (child) =>
      child.type === 'frame' &&
      /progress ring/i.test(child.name ?? '') &&
      getNum(child.width) != null &&
      getNum(child.height) != null,
  );
  const inner = children.find(
    (child) =>
      child.type === 'frame' &&
      /inner circle/i.test(child.name ?? '') &&
      getNum(child.width) != null &&
      getNum(child.height) != null,
  );
  if (!progress || !inner) return null;

  const progressSize = progress as { width?: unknown; height?: unknown };
  const innerSizeSource = inner as { width?: unknown; height?: unknown };
  const pw = getNum(progressSize.width);
  const ph = getNum(progressSize.height);
  const iw = getNum(innerSizeSource.width);
  const ih = getNum(innerSizeSource.height);
  if (!pw || !ph || !iw || !ih) return null;
  if (Math.abs(pw - ph) > 2) return null;
  if (Math.abs(iw - ih) > 2) return null;
  if (iw >= pw) return null;

  return { progress, inner, size: pw };
}

function rewritePseudoRingFrame(node: PenNode): void {
  const match = matchPseudoRingFrame(node);
  if (!match) return;

  const { progress, inner, size } = match;
  const innerSize = getNum((inner as { width?: unknown }).width) ?? Math.round(size * 0.7);
  const innerOffset = Math.round((size - innerSize) / 2);

  (node as PenNode & { width?: number }).width = size;
  (node as PenNode & { height?: number }).height = size;
  (node as PenNode & { cornerRadius?: number }).cornerRadius = size / 2;
  (node as PenNode & { layout?: string }).layout = 'none';
  (node as PenNode & { fill?: unknown }).fill = [];

  const rootStroke = (node as { stroke?: unknown }).stroke;
  const progressStroke = (progress as { stroke?: unknown }).stroke as
    | { thickness?: number; fill?: unknown }
    | undefined;
  if (rootStroke && progressStroke && typeof progressStroke.thickness === 'number') {
    progressStroke.thickness = Math.max(
      2,
      Math.min(progressStroke.thickness, Math.round(progressStroke.thickness * 0.72)),
    );
  }

  (progress as PenNode & { width?: number }).width = size;
  (progress as PenNode & { height?: number }).height = size;
  (progress as PenNode & { cornerRadius?: number }).cornerRadius = size / 2;
  (progress as PenNode & { x?: number }).x = 0;
  (progress as PenNode & { y?: number }).y = 0;
  (progress as PenNode & { fill?: unknown }).fill = [];

  (inner as PenNode & { width?: number }).width = innerSize;
  (inner as PenNode & { height?: number }).height = innerSize;
  (inner as PenNode & { cornerRadius?: number }).cornerRadius = innerSize / 2;
  (inner as PenNode & { x?: number }).x = innerOffset;
  (inner as PenNode & { y?: number }).y = innerOffset;
}

export function rewritePseudoRingFrames(node: PenNode): void {
  rewritePseudoRingFrame(node);
  const children = getChildren(node);
  if (!children) return;
  for (const child of children) rewritePseudoRingFrames(child);
}

// ---------------------------------------------------------------------------
// Rewriter 6: remove accidental fills from stroked ring frames
// ---------------------------------------------------------------------------

function isStrokedRingFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false;

  const label = `${node.id ?? ''} ${node.name ?? ''}`.toLowerCase();
  if (!/(ring|circle|progress|activity)/.test(label)) return false;

  const width = getNum(node.width);
  const height = getNum(node.height);
  if (!width || !height) return false;
  if (Math.abs(width - height) > Math.max(2, Math.max(width, height) * 0.08)) return false;

  const radius = getNum((node as { cornerRadius?: unknown }).cornerRadius);
  if (!radius || radius < Math.min(width, height) * 0.35) return false;

  const stroke = (node as { stroke?: { fill?: unknown; thickness?: unknown } }).stroke;
  if (!stroke || !Array.isArray(stroke.fill) || stroke.fill.length === 0) return false;

  const fill = (node as { fill?: unknown }).fill;
  return Array.isArray(fill) && fill.length > 0;
}

export function stripRingFrameFills(node: PenNode): void {
  if (isStrokedRingFrame(node)) {
    (node as PenNode & { fill?: unknown }).fill = [];
    delete (node as PenNode & { effects?: unknown }).effects;
  }

  const children = getChildren(node);
  if (!children) return;
  for (const child of children) stripRingFrameFills(child);
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/**
 * Run all anti-pattern rewriters against a subtree. Intended to be
 * called by `sanitizeNodesForInsert` / `sanitizeNodesForUpsert`
 * BEFORE `resolveTreeRoles` so the rewritten tree still goes through
 * theme-aware role defaults and layout normalization.
 */
export function rewriteLlmAntiPatterns(node: PenNode): void {
  rewriteStackedEllipsesToRingFrames(node);
  rewriteAlternatingBarLabelSiblings(node);
  rewriteOpenStrokePathsWithDuplicateFill(node);
  normalizeRingTrackProgressGeometry(node);
  rewritePseudoRingFrames(node);
  stripRingFrameFills(node);
}
