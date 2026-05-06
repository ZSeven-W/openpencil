import type { PenNode, PenFill, PenStroke, SolidFill } from '@zseven-w/pen-types';

/**
 * Normalize
 * stroke/fill 架构违规通常由不严格遵循 PenNode 类型的 AI 子代理（MiniMax M2.7、GLM、Kimi）发出。
 *
 * Three 类模式违规会在整个树中递归地就地修复： 1. `stroke` 作为一个条目的数组 — AI
 * 将笔画对象包装在一个数组
 *
 * 中，就好像它是 `fill`（其中 IS 是一个数组）。 We 解开第一个元素并继续对其进行规范化。 2. Stroke 值的形状类似于
 * `SolidFill` ({
 * type, color
 *
 * })，而不是 `PenStroke` ({ Thickness, fill })。 We 将内部 `color` 迁移到正确的
 * `stroke.fill[0]`，将 `strokeWidth` 顶级字段（许多模型发出的 CSS/SVG-style 拼写）拉入
 * `stroke.thickness`，并删除杂散的
 * `strokeWidth
 * `。 If 既不存在厚度也不存在 strokeWidth，默认为 2，因此笔画实际上绘制了一些东西。 3. 删除具有非法 CSS
 * 关键字颜色（`"none"`、`"transparent"`）的 Fill 条目。 The 8 位透明十六进制
 * (`"#00000000"`) 有效并保留。 The 相同的规则适用于任何 `stroke.fill[]` 条目。
 *
 * Returns 什么都没有——树就地变异，与其他笔核标准化通道相匹配。依赖于 Zustand 发布语义的 Callers
 * 应通过 `forcePageResync()` 路由结果，就像它们对其他变异后流传输传递所做的那样。
 *
 *
 *
 *
 *
 *
 */
export function normalizeStrokeFillSchema(node: PenNode): void {
  normalizeNodeStroke(node);
  normalizeNodeFill(node);

  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      normalizeStrokeFillSchema(child);
    }
  }
}

// ---------------------------------------------------------------------------
// Stroke 标准化
// ---------------------------------------------------------------------------

interface MaybeStrokeHolder {
  stroke?: unknown;
  strokeWidth?: unknown;
  'stroke-width'?: unknown;
  'stroke-dasharray'?: unknown;
  'stroke-dashoffset'?: unknown;
  'stroke-linecap'?: unknown;
  'stroke-linejoin'?: unknown;
}

function normalizeNodeStroke(node: PenNode): void {
  const rec = node as unknown as MaybeStrokeHolder;
  const rawStroke = rec.stroke;
  const hasSvgStrokeAttrs =
    rec['stroke-width'] !== undefined ||
    rec['stroke-dasharray'] !== undefined ||
    rec['stroke-dashoffset'] !== undefined ||
    rec['stroke-linecap'] !== undefined ||
    rec['stroke-linejoin'] !== undefined;
  if ((rawStroke === undefined || rawStroke === null) && !hasSvgStrokeAttrs) return;

  // (1) Unwrap `stroke: [ ... ]` 取第一个元素。
  let stroke: unknown = rawStroke;
  if (typeof stroke === 'string') {
    stroke = { type: 'solid', color: stroke };
  }
  if (Array.isArray(stroke)) {
    stroke = stroke.length > 0 ? stroke[0] : undefined;
  }
  if ((!stroke || typeof stroke !== 'object') && !hasSvgStrokeAttrs) {
    delete rec.stroke;
    delete rec.strokeWidth;
    delete rec['stroke-width'];
    delete rec['stroke-dasharray'];
    delete rec['stroke-dashoffset'];
    delete rec['stroke-linecap'];
    delete rec['stroke-linejoin'];
    return;
  }

  // (2) Detect 填充形状作为描边图案并迁移它。
  const maybeFillShape = (stroke ?? {}) as {
    type?: unknown;
    color?: unknown;
    thickness?: unknown;
    fill?: unknown;
  };
  const looksLikeFillShape =
    typeof maybeFillShape.type === 'string' &&
    typeof maybeFillShape.color === 'string' &&
    maybeFillShape.thickness === undefined &&
    maybeFillShape.fill === undefined;

  if (looksLikeFillShape) {
    const thickness = readThickness(rec);
    rec.stroke = {
      thickness,
      fill: [
        {
          type: 'solid',
          color: maybeFillShape.color as string,
        } as SolidFill,
      ],
    } as PenStroke;
    delete rec.strokeWidth;
    // Now 清除迁移笔画内的非法颜色。填充
    stripIllegalColorsFromStrokeFill(node);
    return;
  }

  // Otherwise 我们有一些看起来像真正的 PenStroke 的东西 - 修复缺失的厚度，清理非法颜色，并保留作为顶级属性幸存的
  // 任何 strokeWidth 字段。
  const strokeObj = (stroke ?? {}) as Partial<PenStroke> & { [k: string]: unknown };
  if (strokeObj.thickness === undefined || strokeObj.thickness === null) {
    const width = readThickness(rec);
    (strokeObj as { thickness?: number }).thickness = width;
  }
  const dashPattern = readDashPattern(rec['stroke-dasharray']);
  if (dashPattern && dashPattern.length > 0 && strokeObj.dashPattern === undefined) {
    strokeObj.dashPattern = dashPattern;
  }
  const dashOffset = readDashOffset(rec['stroke-dashoffset']);
  if (dashOffset !== null && strokeObj.dashOffset === undefined) {
    strokeObj.dashOffset = dashOffset;
  }
  const cap = readCap(rec['stroke-linecap']);
  if (cap && strokeObj.cap === undefined) {
    strokeObj.cap = cap;
  }
  const join = readJoin(rec['stroke-linejoin']);
  if (join && strokeObj.join === undefined) {
    strokeObj.join = join;
  }
  if (
    (!Array.isArray(strokeObj.fill) || strokeObj.fill.length === 0) &&
    typeof maybeFillShape.color !== 'string'
  ) {
    const inferredColor = inferStrokeColor(node);
    if (inferredColor) {
      strokeObj.fill = [{ type: 'solid', color: inferredColor }] as PenFill[];
    }
  }
  rec.stroke = strokeObj as PenStroke;
  delete rec.strokeWidth;
  delete rec['stroke-width'];
  delete rec['stroke-dasharray'];
  delete rec['stroke-dashoffset'];
  delete rec['stroke-linecap'];
  delete rec['stroke-linejoin'];
  stripIllegalColorsFromStrokeFill(node);

  // If 清理后，笔划根本没有填充，删除整个笔划。
  const cleaned = rec.stroke as PenStroke | undefined;
  if (cleaned && (!cleaned.fill || cleaned.fill.length === 0)) {
    delete rec.stroke;
  }
}

function readThickness(rec: MaybeStrokeHolder): number {
  const raw = rec.strokeWidth ?? rec['stroke-width'];
  if (typeof raw === 'number' && raw > 0) return raw;
  if (typeof raw === 'string') {
    const n = parseFloat(raw);
    if (Number.isFinite(n) && n > 0) return n;
  }
  return 2;
}

function readDashPattern(raw: unknown): number[] | null {
  if (Array.isArray(raw)) {
    const nums = raw.filter((value): value is number => typeof value === 'number' && value > 0);
    if (nums.length === 1) return [nums[0], nums[0]];
    return nums.length > 0 ? nums : null;
  }
  if (typeof raw === 'string') {
    const nums = raw
      .split(/[,\s]+/)
      .map((part) => parseFloat(part))
      .filter((value) => Number.isFinite(value) && value > 0);
    if (nums.length === 1) return [nums[0], nums[0]];
    return nums.length > 0 ? nums : null;
  }
  return null;
}

function readDashOffset(raw: unknown): number | null {
  if (typeof raw === 'number' && Number.isFinite(raw)) return raw;
  if (typeof raw === 'string') {
    const parsed = parseFloat(raw);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

function readCap(raw: unknown): PenStroke['cap'] | null {
  return raw === 'round' || raw === 'square' || raw === 'none' ? raw : null;
}

function readJoin(raw: unknown): PenStroke['join'] | null {
  return raw === 'round' || raw === 'bevel' || raw === 'miter' ? raw : null;
}

function inferStrokeColor(node: PenNode): string | null {
  const name = typeof node.name === 'string' ? node.name.toLowerCase() : '';
  if (/track/.test(name)) return '#2A2A2A';
  if (/(progress|chart line|line|curve|wave)/.test(name)) return '#22C55E';
  return null;
}

function stripIllegalColorsFromStrokeFill(node: PenNode): void {
  const rec = node as unknown as { stroke?: { fill?: unknown } };
  const stroke = rec.stroke;
  if (!stroke || typeof stroke !== 'object') return;
  const fillArr = stroke.fill;
  if (!Array.isArray(fillArr)) return;
  (stroke as { fill?: PenFill[] }).fill = fillArr.filter((f) => isLegalFillEntry(f)) as PenFill[];
}

// ---------------------------------------------------------------------------
// Fill 标准化
// ---------------------------------------------------------------------------

/**
 * Explicit
 * 透明十六进制。 Used for SHAPE 填充（框架、矩形、椭圆、路径、组...），其中“无填充”实际上意味着一个空心形状，应
 * 该让背景显示出来。 We 写入 8 位透明十六进制，以便 canvas-object-factory
 * 不会回退到不透​​明的默认灰色填充。
 *
 */
const EXPLICIT_TRANSPARENT_FILL: SolidFill = {
  type: 'solid',
  color: '#00000000',
};

/**
 * Node 类型，其
 * `fill` 表示 FOREGROUND 颜色（文本颜色、图标颜色）而不是形状的背景。 On
 * 这些类型非法的“无”/“透明”填充几乎肯定是一个错误：用户的意思是“默认文本颜色”，而不是“不可见文本”。 Freezing
 * 将它们设置为 #00000000 将完全隐藏内容，因此我们删除该字段并让下游层（角色默认值、按钮对比度、样式继承）提供可见颜色。
 *
 *
 *
 */
const FOREGROUND_NODE_TYPES = new Set<string>(['text', 'icon_font']);

function normalizeNodeFill(node: PenNode): void {
  const rec = node as unknown as { fill?: unknown };
  const raw = rec.fill;
  if (!raw) return;
  if (!Array.isArray(raw)) return;
  // Separate 合法条目来自 CSS-关键字非法条目。
  const cleaned = raw.filter((f) => isLegalFillEntry(f));
  if (cleaned.length > 0) {
    rec.fill = cleaned as PenFill[];
    return;
  }
  // Empty 输入，清空 - 保持不变。
  if (raw.length === 0) {
    rec.fill = [] as PenFill[];
    return;
  }
  // Every 原始条目是 CSS 关键字（“无”/“透明”）。
  // The 正确修复取决于 `fill` 是后台还是后台
  // 该节点类型的前景色：
//
  // SHAPE 类型（框架、矩形、椭圆、路径、组……）
  // 填充=形状背景。 “无填充”是空心的意思。 Keep 的
  // 显式透明十六进制，因此画布不会回落到
  // 默认灰色填充。
//
  // FOREGROUND 类型（文本、icon_font）
  // 填充 = text/icon 颜色。 “无填充”会隐藏内容 -
  // 几乎可以肯定不是 AI 的意思。 Delete 场如此
  // 下游层可以填充可见颜色。
  if (FOREGROUND_NODE_TYPES.has(node.type)) {
    delete rec.fill;
  } else {
    rec.fill = [EXPLICIT_TRANSPARENT_FILL] as PenFill[];
  }
}

/** Reject 填充颜色为不受支持的 CSS 关键字的条目。 */
function isLegalFillEntry(entry: unknown): boolean {
  if (!entry || typeof entry !== 'object') return false;
  const e = entry as { type?: unknown; color?: unknown };
  if (e.type === 'solid' && typeof e.color === 'string') {
    const c = e.color.trim().toLowerCase();
    if (c === 'none' || c === 'transparent') return false;
  }
  return true;
}
