import type { PenNode, FrameNode, SizingBehavior } from '@/types/pen';
import type { PathNode } from '@/types/pen';
import type { PenFill, PenStroke, PenEffect, SolidFill } from '@/types/styles';
import {
  toSizeNumber,
  toGapNumber,
  parsePaddingValues,
  estimateNodeIntrinsicHeight,
  getTextContentForNode,
  hasCjkText,
} from './generation-utils';
import { resolveIconPathBySemanticName } from './icon-resolver';

// ---------------------------------------------------------------------------
// Context 传递给每个角色规则函数
// ---------------------------------------------------------------------------

export interface RoleContext {
  /** 父节点的 Role（如果有） */
  parentRole?: string;
  /** 父节点的 Layout */
  parentLayout?: 'none' | 'vertical' | 'horizontal';
  /** Width 父节点的内容区域（px） */
  parentContentWidth?: number;
  /** Root 画布宽度（桌面版 1200，移动版 375） */
  canvasWidth: number;
  /** Whether 在设计上下文中检测到 CJK 文本 */
  hasCjk?: boolean;
  /** Whether 该节点位于表状结构内 */
  isTableContext?: boolean;
  /**
   * 从 `resolveTr
   * eeRoles` 开头的页面根填充中检测到 Document 主题。具有视觉默认值（导航栏、卡片、输入、分隔线）的 Roles
   * 会读取此内容，因此 LLM 不会在明确的深色页面背景上绘制 #FFFFFF 默认值。
   *
   */
  theme?: 'dark' | 'light';
}

// ---------------------------------------------------------------------------
// Role defaults — 填充节点上未设置值的部分属性
// ---------------------------------------------------------------------------

export type RoleDefaults = Partial<{
  layout: 'none' | 'vertical' | 'horizontal';
  gap: number;
  padding: number | [number, number] | [number, number, number, number];
  justifyContent: 'start' | 'center' | 'end' | 'space_between' | 'space_around';
  alignItems: 'start' | 'center' | 'end';
  width: SizingBehavior;
  height: SizingBehavior;
  clipContent: boolean;
  cornerRadius: number;
  textGrowth: 'auto' | 'fixed-width' | 'fixed-width-height';
  textAlign: 'left' | 'center' | 'right';
  textAlignVertical: 'top' | 'middle' | 'bottom';
  lineHeight: number;
  letterSpacing: number;
  fill: PenFill[];
  stroke: PenStroke;
  effects: PenEffect[];
}>;

/** 角色规则函数根据上下文计算默认值。 */
export type RoleRuleFn = (node: PenNode, ctx: RoleContext) => RoleDefaults;

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

const roleRegistry = new Map<string, RoleRuleFn>();

/**
 * Register
 * 角色规则。 Any 字符串是有效的角色名称。 If 同一角色注册两次，后者获胜。
 */
export function registerRole(role: string, ruleFn: RoleRuleFn): void {
  roleRegistry.set(role, ruleFn);
}

// ---------------------------------------------------------------------------
// 针对不输出 `role` 的模型进行基于 Name 的角色推断
// ---------------------------------------------------------------------------

/** Exact 名称 → 角色映射（不区分大小写）。 */
const NAME_EXACT_MAP: Record<string, string> = {
  navbar: 'navbar',
  navigation: 'navbar',
  'navigation bar': 'navbar',
  'nav bar': 'navbar',
  nav: 'navbar',
  header: 'navbar',
  'top bar': 'navbar',
  topbar: 'navbar',
  hero: 'hero',
  'hero section': 'hero',
  footer: 'footer',
  'search bar': 'search-bar',
  searchbar: 'search-bar',
  'search input': 'search-bar',
  search: 'search-bar',
  avatar: 'avatar',
  divider: 'divider',
  separator: 'divider',
  spacer: 'spacer',
  badge: 'badge',
  tag: 'tag',
  pill: 'pill',
  table: 'table',
};

/** Names 表示容器而不是单个组件。 */
const CONTAINER_SUFFIXES = /\b(group|row|container|wrapper|section|list|area|stack|grid|bar)s?\b/i;

/**
 * Words，当与类似角色
 * 的单词组合时，将节点转换为该角色的 PART 而不是它的实例。 “Card Header”、“Card Body”、“Card
 * Footer”都是父卡内的结构部分 - 它们必须 NOT 继承卡的角色默认值（白色填充、阴影、圆角...）。 Stored
 * 作为 Set （不是正则表达式），因为检查是“角色关键字之后的第一个单词恰好是其中一个” -
 * 位置敏感和单词范围，而不是子字符串扫描。子字符串扫描会错误地拒绝诸如“Card with Icon”或“Button with
 *
 * Image”之类的介词变体，其中部分词与角色词通过修
 * 饰语（“with”）分开
 * ，并且整个名称描述角色的变体而不是其内部部分。
 *
 *
 *
 *
 */
const ROLE_PART_WORDS = new Set([
  'header',
  'body',
  'footer',
  'title',
  'subtitle',
  'content',
  'wrapper',
  'container',
  'area',
  'label',
  'value',
  'caption',
  'description',
  'image',
  'media',
  'icon',
  'action',
  'actions',
  'meta',
  'row',
  'column',
  'stack',
  'grid',
]);

/**
 * Extract
 * 字符串中第一个有意义的字母标记，跳过前导空格、标点符号、数字和单字母片段。当不存在合格标记时，Returns null。 Why 仅
 * alpha 和最小长度 2：子代理经常使用角色词和部分词之间的数字索引来命名节点“Card 1 Content”、“Card 2
 *
 * Header”、“Button 3 Label”。天真的 `\w+`
 * 将匹配索引（“1”）并丢
 * 失实际上确定节点是否是结构片段的尾部“内容”/“标题”/“标签”。 We 跳过任何非 alpha（或短于 2 个字符的
 * alpha）并向前扫描，直到我们找到真正的单词，因此“Card 1 Content”正确地显示“内容”并被视为卡片片段。
 *
 *
 *
 *
 */
function firstWordToken(s: string): string | null {
  const m = /[a-z]{2,}/i.exec(s);
  return m ? m[0].toLowerCase() : null;
}

/** Substring 模式 → 角色（按顺序检查，第一个匹配获胜）。 */
const NAME_PATTERN_MAP: [RegExp, string, boolean?][] = [
  [/\bbtn\b|\bbutton\b/i, 'button', true],
  [/\bcard\b/i, 'card', true],
  [/\binput\b|text\s*field|text\s*box/i, 'input'],
  [/\bform\b/i, 'form-group'],
  [/\bsearch/i, 'search-bar'],
  [/\bnav\s*link/i, 'nav-link'],
  [/\bstat/i, 'stat-card', true],
  [/\bpricing/i, 'pricing-card', true],
  [/\btestimonial\b|\breview\b|\bquote\b/i, 'testimonial'],
  [/\bcta\b|call\s*to\s*action/i, 'cta-section'],
  [/\bfeature/i, 'feature-card', true],
  [/\bicon\b/i, 'icon'],
];

/**
 * Infer 当未设置显式
 * 角色时，来自节点名称的语义角色。 Only 适用于框架节点 - 文本、路径、图像等不需要角色推断。
 */
function inferRoleFromName(node: PenNode): string | undefined {
  if (node.type !== 'frame') return undefined;
  const name = node.name;
  if (!name) return undefined;

  const lower = name.toLowerCase().trim();

  // Exact 首先匹配
  const exact = NAME_EXACT_MAP[lower];
  if (exact) return exact;

  // Pattern 比赛
  for (const [pattern, role, skipContainers] of NAME_PATTERN_MAP) {
    // Use exec（不是测试），因此我们知道角色单词所在的名称中的 WHERE。 Position 对于下面的
    // ROLE_PART_WORDS 后卫很重要。
    const match = pattern.exec(lower);
    if (!match) continue;

    if (skipContainers) {
      // Skip 类似容器的名称（例如“Button Group”、“Buttons Row”）
      if (CONTAINER_SUFFIXES.test(lower)) continue;
      // Skip "Card Header"、"Card Body"、"Button Label" 等 — 当角色关键字后面的
      // FIRST 词是部分词时，节点是角色的 PIECE。 Two 位置守卫在这里很重要： 1. We 查看文本 AFTER
      // 匹配，因此“Icon Button”（角色之前的部分单词）正确保留为按钮。 2. We 仅检查该后缀中的
      // FIRST 单词，因此“Card with Icon”/“Button with Image”（介词变体：“一张
      // HAS 图标的卡片”）保留其角色 - 第一个单词是“with”，而不是部分单词。
      const afterMatch = lower.slice(match.index + match[0].length);
      const nextWord = firstWordToken(afterMatch);
      if (nextWord && ROLE_PART_WORDS.has(nextWord)) continue;
    }
    return role;
  }

  return undefined;
}

// ---------------------------------------------------------------------------
// Per-节点分辨率
// ---------------------------------------------------------------------------

/**
 * Apply 基于角色默认
 * 为单个节点。 Only 填充 NOT 已由 AI 设置的属性。 The AI
 * 的显式属性总是获胜。 If 未设置显式角色，尝试从节点名称推断角色。
 *
 */
export function resolveNodeRole(node: PenNode, ctx: RoleContext): void {
  let role = node.role;

  // 如果未明确设置，则 Infer 角色来自名称
  if (!role) {
    role = inferRoleFromName(node);

    // Page-chrome 推断在卡系列父级中是错误的。 The LLM 经常将卡片的内部部分命名为“Header”和“Foote
    // r”（分别是卡片的标题行和操作行），但 `NAME_EXACT_MAP`
    // 盲目地将这些映射到页面级“导航栏”和“页脚”角色 — 然后将导航栏填充 +
    // 边框或页脚填充注入内部卡片部分，将其变成一个耀眼的白色条（心率“Mini Chart”回归）。 Strip
    // 当直接父级已经是卡片系列角色时的推论：这些是容器角色，其子级是卡片片段，而不是页面镶边。
    if (
      role &&
      PAGE_CHROME_ROLES.has(role) &&
      ctx.parentRole &&
      CARD_LIKE_ROLES.has(ctx.parentRole)
    ) {
      role = undefined;
    }

    if (role) {
      (node as unknown as Record<string, unknown>).role = role;
    }
  }

  if (!role) return;

  // Size 卡系列角色的健全性检查。 The `inferRoleFromName`
  // 模式匹配器是词法的 — 名为“Status Dot”的 6×6 节点
  // `/\bstat/` 正则表达式并获取 `role: 'stat-card'`，然后
  // 注入 24px 填充、卡片阴影和 cornerRadius。 Refuse 至
// apply card-like roles on nodes too small to plausibly be a card;
  // 完全删除该角色，以便下游通道也处理
  // 节点已展开。 This 捕获名称推断和 LLM-
  // 相同错误的直接发出版本。
  if (CARD_LIKE_ROLES.has(role) && isAbsurdlyTinyForCardRole(node)) {
    delete (node as { role?: string }).role;
    return;
  }

  const ruleFn = roleRegistry.get(role);
  if (!ruleFn) return; // 未知角色——不变地通过

  const defaults = ruleFn(node, ctx);
  if (!defaults) return;

  applyDefaults(node, defaults);
}

/**
 * Roles 注入大量视觉
 * 默认值（填充 ≥ 16、卡片阴影、cornerRadius ≥ 12、填充），这仅在足以容纳内容的容器上才有意义。 Applying
 * 它们到一个小元素（例如，一个 6×6 状态点，其名称恰好与 `/\bstat/` 匹配并触发 stat-card
 * 模式）默默地将其膨胀为具有 24px 填充和投影的超大卡片。 When `resolveNodeRole` 在此列表中看到一个角色
 * AND 节点声明的宽度或高度低于 `CARD_LIKE_MIN_DIMENSION`，在应用任何默认值之前，该角色将被剥离（设
 * 置回未定义）。 The 节点保留其已有的任何内容。
 *
 *
 *
 *
 *
 */
const CARD_LIKE_ROLES = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
]);
const CARD_LIKE_MIN_DIMENSION = 40;

/**
 * Roles 仅在页面树的
 * 顶部有意义 - 它们绘制页面级镶边（顶部的导航栏、底部的页脚带、全出血英雄块、全宽号召性用语带）。 When LLM 将卡片的
 * INTERNAL 部分命名为“Header”或“Footer”（卡片的标题行/操作行），`NAME_EXACT_MAP`
 *
 * 中的词汇名称匹配盲目返回“导航栏”/“页脚”——然后将导航栏填充 + 边框或页脚填充注入到卡片内部的部分中，将其变成耀眼的白色不属于那
 * 里的酒吧。 `resol
 * veNodeRole` 会删除任何直接父级位于 `CARD_LIKE_ROLES`
 * 中的推断角色，原则是页面镶边角色不能存在于卡内。搜索栏有意包含 NOT：搜索输入合法地出现在设置卡、个人资料卡等内部。搜索栏
 * 的 The 视觉默认值也是无害的（圆角输入填充），因此即使它被错误推断，视觉成本也很小。
 *
 *
 *
 *
 *
 *
 *
 *
 */
const PAGE_CHROME_ROLES = new Set(['navbar', 'footer', 'hero', 'cta-section']);

/**
 * Read 如果可能，将声
 * 明的尺寸作为像素数。 Returns `null` 代表 `'fill_container'`、`'fit_content'`、`u
 * ndefined` 或任何非数字值 - 这些大小调整模式不会告诉我们最终渲染是否会很小，因此我们拒绝从它们中做出决定并回退到允许的默认
 * 值（应用角色）。
 *
 */
function readDeclaredPixelSize(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  return null;
}

function isAbsurdlyTinyForCardRole(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  const w = readDeclaredPixelSize((node as { width?: unknown }).width);
  const h = readDeclaredPixelSize((node as { height?: unknown }).height);
  // 当 BOTH 维度被声明为数字 AND 至少有一个低于阈值时，Only 拒绝。 Unknown/sizing-keyword
  // 维度保持不变（我们无法判断它们将解析为什么）。
  if (w == null || h == null) return false;
  return w < CARD_LIKE_MIN_DIMENSION || h < CARD_LIKE_MIN_DIMENSION;
}

/**
 * Apply 默认为节点，仅设置 undefined/missing 的属性。
 */
function applyDefaults(node: PenNode, defaults: RoleDefaults): void {
  const record = node as unknown as Record<string, unknown>;

  for (const [key, value] of Object.entries(defaults)) {
    if (value === undefined) continue;

    // Only 如果该属性尚未存在于节点上则设置
    if (record[key] === undefined) {
      record[key] = value;
    }
  }
}

// ---------------------------------------------------------------------------
// Tree 级分辨率
// ---------------------------------------------------------------------------

/**
 * Walk 树深度优先，解
 * 析每个节点的角色。 This 取代了旧的 applyGenerationHeuristics 树行走。
 */
export function resolveTreeRoles(
  root: PenNode,
  canvasWidth: number,
  parentRole?: string,
  parentLayout?: 'none' | 'vertical' | 'horizontal',
  parentContentWidth?: number,
  isTableContext = false,
  theme?: 'dark' | 'light',
): void {
  // Detect 主题来自第一次（入口）调用时根节点的填充。 Subsequent
  // 递归调用继承父级的已解析主题，因此每个节点都会看到相同的值。
  const resolvedTheme = theme ?? detectThemeFromNode(root);

  const ctx: RoleContext = {
    parentRole,
    parentLayout,
    parentContentWidth,
    canvasWidth,
    isTableContext,
    theme: resolvedTheme,
  };

  // 文本节点中的 Detect CJK
  if (root.type === 'text') {
    const text = getTextContentForNode(root);
    ctx.hasCjk = hasCjkText(text);
  }

  resolveNodeRole(root, ctx);

  // Recurse 进入儿童
  if (!('children' in root) || !Array.isArray(root.children)) return;

  const nodeW = toSizeNumber(
    ('width' in root ? root.width : undefined) as number | string | undefined,
    0,
  );
  const pad = parsePaddingValues('padding' in root ? root.padding : undefined);
  const contentW = nodeW > 0 ? nodeW - pad.left - pad.right : 0;

  const childTableContext = isTableContext || root.role === 'table' || root.role === 'table-row';

  for (const child of root.children) {
    resolveTreeRoles(
      child,
      canvasWidth,
      root.role,
      'layout' in root ? root.layout : undefined,
      contentW || parentContentWidth,
      childTableContext,
      resolvedTheme,
    );
  }
}

/**
 * Detect 节点填充颜色的浅色主题与深色主题。
 *
 * Used by `resolveTreeRoles`（并导出用于清理时间调用站点
 * 在 `design-canvas-ops.ts`）所以角色默认功能可以选择填充
 * 与页面背景相匹配，而不是始终默认为
 * 浅色主题 `#FFFFFF`。
 *
 * IMPORTANT: 在这里传递实际的 PAGE ROOT ，而不是任何子树
 * 解析器当前正在行走。暗页内的卡片没有填充
 * 它自己的（LLM 省略了它，因为它期望黑暗页面 bg
 * 显示透）——在卡上调用它会返回“light”（默认
 * 后备），这是错误的答案。 Always 查找文档
 * 页面根目录并在解析 MCP-emissed 时显式传递它
 * 插入之前的子树。
 *
 * Heuristic：如果第一个纯色填充颜色具有 WCAG 相对亮度
 * 低于 0.3，设计为深色主题。 Otherwise （或者当填充时
 * 缺少/变量引用/不是纯色）我们默认为“浅色”
 * for backward compatibility with all existing light-theme designs.
 */
export function detectThemeFromNode(node: PenNode): 'dark' | 'light' {
  if (!('fill' in node) || !Array.isArray((node as { fill?: unknown[] }).fill)) return 'light';
  const fills = (node as { fill: Array<{ type?: string; color?: string }> }).fill;
  const first = fills[0];
  if (!first || first.type !== 'solid' || typeof first.color !== 'string') return 'light';
  const color = first.color.trim();
  // Skip 变量引用（$color-1 等）——我们无法在这里解析它们。
  if (color.startsWith('$')) return 'light';
  const m = color.match(/^#([0-9a-fA-F]{3,8})$/);
  if (!m) return 'light';
  let hex = m[1];
  if (hex.length === 3)
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  if (hex.length !== 6 && hex.length !== 8) return 'light';
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return 'light';
  // sRGB → 相对亮度
  const lin = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  const Y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  return Y < 0.3 ? 'dark' : 'light';
}

// ---------------------------------------------------------------------------
// Post-pass：需要完整树的跨节点修复
// ---------------------------------------------------------------------------

/**
 * Apply 在完整树进行
 * 角色解析后修复跨节点。 These 修复了每个节点规则看不到的需要 sibling/parent 上下文的问题。
 */
export function resolveTreePostPass(
  root: PenNode,
  canvasWidth: number,
  getNodeById?: (id: string) => PenNode | undefined,
  updateNode?: (id: string, updates: Partial<PenNode>) => void,
  parentNode?: PenNode,
): void {
  if (root.type !== 'frame') return;
  if (!('children' in root) || !Array.isArray(root.children)) return;

  // `updateNode` 遍历 Zustand 的不可变 `updateNodeInTree`，它沿更新路径浅克隆每个祖先。 Once
  // 我们称之为 Once，我们的 `root` 参数引用与存储分离：稍后对 `root` (fill/effects)
  // 的直接突变将被默默删除，并且过时的读取（例如，我们传递给孩子的填充为 `parentNode`）可能会掩盖实时树状态。 After 每次
  // updateNode 调用时，通过 `getNodeById` 重新获取新引用并重新绑定 `currentRoot` +
  // `children`，然后再继续。 Children 数组标识在 `updateNode(currentRoot.id, patch)`
  // 中保留，因为补丁从未触及 `children`，但为了清晰和安全，我们通过新根重新绑定它。
  let currentRoot: FrameNode = root as FrameNode;
  const refreshRoot = () => {
    if (!getNodeById) return;
    const fresh = getNodeById(currentRoot.id);
    if (fresh && fresh.type === 'frame') {
      currentRoot = fresh as FrameNode;
    }
  };
  const currentChildren = (): PenNode[] =>
    Array.isArray(currentRoot.children) ? currentRoot.children : [];

  // --- Card 行均衡 ---
  if (currentRoot.layout === 'horizontal' && currentChildren().length >= 2) {
    equalizeCardRow(currentRoot, currentChildren());
  }

  // --- Horizontal 溢出修复 ---
  if (
    currentRoot.layout === 'horizontal' &&
    typeof currentRoot.width === 'number' &&
    currentChildren().length >= 2
  ) {
    fixHorizontalOverflow(currentRoot, currentChildren(), canvasWidth);
  }

  // --- Form 输入一致性 ---
  if (
    currentRoot.layout === 'vertical' &&
    currentRoot.width !== 'fit_content' &&
    currentChildren().length >= 2
  ) {
    normalizeFormInputWidths(currentRoot, currentChildren());
  }

  // --- Input 尾随图标对齐 ---
  if (currentRoot.layout === 'horizontal' && currentChildren().length >= 2) {
    normalizeInputTrailingIconAlignment(currentRoot, currentChildren());
  }

  // --- Placeholder 图标修复 ---
  repairPlaceholderIcons(currentRoot, parentNode);

  // --- Text 高度估计 ---
  if (currentRoot.layout && currentRoot.layout !== 'none') {
    fixTextHeights(currentRoot, currentChildren(), canvasWidth);
  }

  // --- Frame 高度扩展 ---
  if (
    typeof currentRoot.height === 'number' &&
    currentRoot.layout &&
    currentRoot.layout !== 'none'
  ) {
    const intrinsic = estimateNodeIntrinsicHeight(currentRoot, undefined, canvasWidth);
    const maxExpansion = currentRoot.height * 1.3;
    if (intrinsic > currentRoot.height && intrinsic <= maxExpansion) {
      if (updateNode) {
        updateNode(currentRoot.id, { height: Math.round(intrinsic) });
        refreshRoot();
      } else {
        (currentRoot as unknown as Record<string, unknown>).height = Math.round(intrinsic);
      }
    }
  }

  // --- clipContent 用于带有 cornerRadius + 子图像 --- 的框架
  if (!currentRoot.clipContent) {
    const cr =
      typeof currentRoot.cornerRadius === 'number'
        ? currentRoot.cornerRadius
        : Array.isArray(currentRoot.cornerRadius) && currentRoot.cornerRadius.length > 0
          ? currentRoot.cornerRadius[0]
          : 0;
    if (cr > 0 && currentChildren().some((c) => c.type === 'image')) {
      if (updateNode) {
        updateNode(currentRoot.id, { clipContent: true } as Partial<PenNode>);
        refreshRoot();
      } else {
        currentRoot.clipContent = true;
      }
    }
  }

  // --- Button 前景对比 ---
  fixButtonForegroundContrast(currentRoot);

  // --- Section 背景交替 ---
  if (currentRoot.layout === 'vertical' && currentChildren().length >= 3) {
    fixSectionAlternation(currentRoot, currentChildren());
  }

  // --- Orphan 容器对比 --- This 其中之一是上面
  // refreshRoot 舞蹈的主要动机：它直接变异 `currentRoot.fill` 和
  // `currentRoot.effects`，并且如果我们仍然持有陈旧的 `root`，则会默默地丢失其写入。
  fixOrphanContainerContrast(currentRoot, parentNode);

  // --- Input 同级 fill/stroke 一致性 ---
  if (currentRoot.layout === 'vertical' && currentChildren().length >= 2) {
    fixInputSiblingConsistency(currentRoot, currentChildren());
  }

  // Recurse。 Pass `currentRoot` 与 parentNode
  // 一样，因此后代看到此传递刚刚写入的任何状态（例如，从 fixOrphanContainerContrast
  // 新分配的填充），而不是突变前快照。
  for (const child of currentChildren()) {
    resolveTreePostPass(child, canvasWidth, getNodeById, updateNode, currentRoot);
  }
}

// ---------------------------------------------------------------------------
// Visual 助手（导出用于测试）
// ---------------------------------------------------------------------------

/**
 * Compute
 * 从十六进制颜色字符串感知亮度。 Returns 0（黑色）到 1（白色）。 Handles
 #RRGGBB 和 #RRGGBBAA。
 */
export function hexLuminance(hex: string): number {
  const h = hex.replace('#', '');
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

/**
 * Check 如果节点具有
 * 非空填充数组。 Does NOT 区分 AI 显式填充和角色默认填充。
 */
/**
 * Returns
 *
 * 当节点具有 ANY 声明的填充条目（可见或不可见）时为 true。 This 是“覆盖保护”谓词：像
 * `fixOrphanCo
 * ntainerContrast` 和 `fixSectionAlternation`
 * 这样的启发式方法要求它来决定作者是否已经做出了他们应该尊重的故意填充选择。 An
 * 显式透明填充（`#00000000`、`"transparent"`、`"none"`） IS 是经过深思熟虑的选择 -
 * “我希望此容器透明” - 并且必须保留，而不是交换为默认的白色背景。 When
 * 您需要知道“这会在屏幕上绘制可见的颜色吗？” （例如，要确定按钮前景对比度通道是否需要以可读颜色绘制），请改用
 *
 * `hasVisibleFill`。
 *
 *
 *
 */
export function hasFill(node: PenNode): boolean {
  return 'fill' in node && Array.isArray(node.fill) && node.fill.length > 0;
}

/**
 * Returns
 * 当节点具有实际渲染可见颜色的填充时为 true。 Differs 从 `hasFill` 通过拒绝不绘制任何内容的填充： -
 * Solid 填充其颜色为 `#00000000`、任何带 `00` alpha 的 8 位十六进制，或 CSS 关键字
 * `"transparen
 * t"` / `"none"` - Any 填充（实心、渐变、图像），其 `opacity` 字段为 `0` （或任何非正数
 * - 负值被防御性地视为零） Use 在决定节点是否需要颜色时使用 PAINTED ONTO
 * 它（按钮前景对比度、聚焦环电源等）。 Do NOT 使用此来决定是否覆盖作者的填充选择 - 透明是合法的选择。 See
 * `hasFill` 对于这种情况。
 *
 *
 *
 *
 *
 */
export function hasVisibleFill(node: PenNode): boolean {
  if (!('fill' in node) || !Array.isArray(node.fill) || node.fill.length === 0) return false;
  const first = node.fill[0];
  if (!first) return false;
  return !isFillInvisible(first);
}

/** 当填充的不透明度 <= 0 或（对于固体）其颜色是显式透明的十六进制 / CSS 关键字时，填充是不可见的。
 *  */
function isFillInvisible(fill: PenFill): boolean {
  const opacity = (fill as { opacity?: unknown }).opacity;
  if (typeof opacity === 'number' && opacity <= 0) return true;
  if (fill.type === 'solid') {
    return isInvisibleColor((fill as SolidFill).color);
  }
  return false;
}

function isInvisibleColor(color: unknown): boolean {
  if (typeof color !== 'string') return false;
  const c = color.trim().toLowerCase();
  if (c === 'transparent' || c === 'none') return true;
  // 带 00 alpha 的 8 位十六进制 (#RRGGBB00)。 Valid 十六进制颜色文字，但它什么也不绘制。
  if (/^#[0-9a-f]{6}00$/i.test(c)) return true;
  return false;
}

/**
 * Extract
 * 节点中的第一个纯色填充颜色，或未定义。 Used 通过后期视觉修复（Tasks 5、7、8）。
 */
export function getFirstSolidColor(node: PenNode): string | undefined {
  if (!hasFill(node)) return undefined;
  const fills = (node as unknown as { fill: PenFill[] }).fill;
  const solid = fills.find((f): f is SolidFill => f.type === 'solid');
  return solid?.color;
}

// ---------------------------------------------------------------------------
// Post-pass 助手
// ---------------------------------------------------------------------------

function fixButtonForegroundContrast(parent: FrameNode): void {
  if (parent.role !== 'button' && parent.role !== 'icon-button') return;
  // 透明按钮没有背景颜色来计算对比度——无事可做，而且我们绝对不应该在不可见的按钮上将文本涂成白色。
  if (!hasVisibleFill(parent)) return;

  const bgColor = getFirstSolidColor(parent);
  if (!bgColor) return;

  const lum = hexLuminance(bgColor);
  const fgColor = lum < 0.5 ? '#FFFFFF' : '#0F172A';
  const fgFill: PenFill[] = [{ type: 'solid', color: fgColor }];

  if (!('children' in parent) || !Array.isArray(parent.children)) return;

  for (const child of parent.children) {
    const rec = child as unknown as Record<string, unknown>;

    if (child.type === 'text' || child.type === 'icon_font') {
      // `hasVisibleFill` 将透明十六进制占位符填充视为未填充，因此标准化器的 #00000000
      // 剩余部分不会阻止对比度提供可见颜色。
      if (!hasVisibleFill(child)) {
        rec.fill = fgFill;
      }
    } else if (child.type === 'path') {
      const hasStroke = 'stroke' in child && child.stroke != null;
      const hasStrokeFill =
        hasStroke &&
        Array.isArray((child.stroke as PenStroke)?.fill) &&
        (child.stroke as PenStroke).fill!.length > 0;

      if (hasVisibleFill(child)) {
        // 填充样式图标 — 已设置样式，跳过
      } else if (hasStroke && !hasStrokeFill) {
        (child.stroke as unknown as Record<string, unknown>).fill = fgFill;
      } else if (!hasStroke && !hasVisibleFill(child)) {
        rec.fill = fgFill;
      }
    }
  }
}

const SECTION_ROLES = new Set(['section', 'hero', 'cta-section', 'stats-section', 'footer']);
const ALTERNATING_BG = ['#FFFFFF', '#F8FAFC'];

function fixSectionAlternation(parent: FrameNode, children: PenNode[]): void {
  if (parent.layout !== 'vertical') return;

  // Only 在浅色主题页面上交替出现。 ALTERNATING_BG 被硬编码为 #FFFFFF/#F8FAFC，它在深色根背景上绘制可见
  // 的白色条带 - 与用户想要的相反。 Dark 主题依赖于 card/component 与组部分的内部对比，而不是外部部分背景清洗。
  // When 父级没有实体填充，我们陷入现有（灯光模式）行为。
  const parentBg = getFirstSolidColor(parent);
  if (parentBg && hexLuminance(parentBg) < 0.5) return;

  const runs: PenNode[][] = [];
  let current: PenNode[] = [];

  for (const child of children) {
    if (child.type === 'frame' && child.role && SECTION_ROLES.has(child.role)) {
      current.push(child);
    } else {
      if (current.length > 0) {
        runs.push(current);
        current = [];
      }
    }
  }
  if (current.length > 0) runs.push(current);

  for (const run of runs) {
    const unfilled = run.filter((c) => !hasFill(c));
    if (unfilled.length < 3) continue;

    let idx = 0;
    for (const section of run) {
      if (!hasFill(section)) {
        (section as unknown as Record<string, unknown>).fill = [
          { type: 'solid', color: ALTERNATING_BG[idx % 2] },
        ];
        idx++;
      }
    }
  }
}

const STRUCTURAL_DENYLIST = new Set([
  'section',
  'row',
  'column',
  'centered-content',
  'form-group',
  'feature-grid',
  'screenshot-frame',
  'phone-mockup',
  'navbar',
  'nav-links',
  'hero',
  'footer',
  'cta-section',
  'stats-section',
  'table',
  'table-row',
  'table-header',
  'spacer',
  'divider',
]);

const CARD_LIKE_ALLOWLIST = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
]);

function fixOrphanContainerContrast(node: FrameNode, parentNode?: PenNode): void {
  if (!parentNode) return;
  if (hasFill(node)) return;
  if (hasFill(parentNode)) return;
  if (isRingLikeDecorativeContainer(node)) return;

  const cr =
    typeof node.cornerRadius === 'number'
      ? node.cornerRadius
      : Array.isArray(node.cornerRadius) && node.cornerRadius.length > 0
        ? node.cornerRadius[0]
        : 0;
  if (cr <= 0) return;

  if (!('children' in node) || !Array.isArray(node.children) || node.children.length === 0) return;

  const role = node.role;
  if (role && STRUCTURAL_DENYLIST.has(role)) return;
  if (role && !CARD_LIKE_ALLOWLIST.has(role)) return;

  const rec = node as unknown as Record<string, unknown>;
  rec.fill = [{ type: 'solid', color: '#FFFFFF' }];
  rec.effects = [
    { type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: '#0000001A' },
    { type: 'shadow', offsetX: 0, offsetY: 1, blur: 2, spread: -1, color: '#0000000F' },
  ];
}

function isRingLikeDecorativeContainer(node: FrameNode): boolean {
  const label = `${node.id ?? ''} ${node.name ?? ''}`.toLowerCase();
  if (!/(ring|circle|progress|activity)/.test(label)) return false;
  if (!node.stroke) return false;

  const width = toSizeNumber(node.width, 0);
  const height = toSizeNumber(node.height, 0);
  if (width <= 0 || height <= 0) return false;

  const roughlySquare = Math.abs(width - height) <= Math.max(2, Math.max(width, height) * 0.08);
  if (!roughlySquare) return false;

  const cr =
    typeof node.cornerRadius === 'number'
      ? node.cornerRadius
      : Array.isArray(node.cornerRadius) && node.cornerRadius.length > 0
        ? node.cornerRadius[0]
        : 0;

  return cr >= Math.min(width, height) * 0.35;
}

function fixInputSiblingConsistency(_parent: FrameNode, children: PenNode[]): void {
  const inputs = children.filter(
    (c) => c.type === 'frame' && (c.role === 'input' || c.role === 'form-input') && hasFill(c),
  );
  if (inputs.length < 2) return;

  const firstColor = getFirstSolidColor(inputs[0]);
  if (!firstColor) return;
  const allMatch = inputs.every((inp) => getFirstSolidColor(inp) === firstColor);
  if (allMatch) return;

  const sourceFill = (inputs[0] as unknown as Record<string, unknown>).fill;
  const sourceStroke = (inputs[0] as unknown as Record<string, unknown>).stroke;

  for (let i = 1; i < inputs.length; i++) {
    const rec = inputs[i] as unknown as Record<string, unknown>;
    rec.fill = sourceFill;
    if (sourceStroke) {
      rec.stroke = sourceStroke;
    }
  }
}

function equalizeCardRow(parent: FrameNode, children: PenNode[]): void {
  if (parent.width === 'fit_content') return;

  const cardCandidates = children.filter(
    (c) =>
      c.type === 'frame' &&
      c.role !== 'divider' &&
      c.role !== 'phone-mockup' &&
      toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  );
  if (cardCandidates.some((c) => 'width' in c && c.width === 'fill_container')) return;

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  );
  if (fixedFrames.length < 2) return;

  const widths = fixedFrames.map((c) => toSizeNumber('width' in c ? c.width : undefined, 0));
  const maxW = Math.max(...widths);
  const minW = Math.min(...widths);
  if (maxW <= 0 || minW / maxW >= 0.6) return;

  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0));
  const maxH = Math.max(...heights);
  const minH = Math.min(...heights);
  if (maxH <= 0 || minH / maxH <= 0.5) return;

  for (const child of fixedFrames) {
    (child as unknown as Record<string, unknown>).width = 'fill_container';
    (child as unknown as Record<string, unknown>).height = 'fill_container';
  }
}

function fixHorizontalOverflow(parent: FrameNode, children: PenNode[], canvasWidth: number): void {
  const parentW = toSizeNumber(parent.width, 0);
  if (parentW <= 0) return;

  const pad = parsePaddingValues(parent.padding);
  const gap = toGapNumber(parent.gap);
  const availW = parentW - pad.left - pad.right;

  let childrenTotalW = 0;
  for (const child of children) {
    const cw = toSizeNumber(
      'width' in child ? (child as { width?: number | string }).width : undefined,
      0,
    );
    if (typeof (child as { width?: unknown }).width === 'number' && cw > 0) {
      childrenTotalW += cw;
    } else {
      childrenTotalW += 80;
    }
  }
  const gapTotal = gap * (children.length - 1);
  childrenTotalW += gapTotal;

  if (childrenTotalW <= availW) return;

  // Strategy 1: Reduce gap
  for (const tryGap of [8, 4]) {
    if (gap > tryGap) {
      const reduced = childrenTotalW - gapTotal + tryGap * (children.length - 1);
      if (reduced <= availW) {
        (parent as unknown as Record<string, unknown>).gap = tryGap;
        childrenTotalW = reduced;
        break;
      }
    }
  }

  // Strategy 2: Expand parent
  if (childrenTotalW > availW) {
    const neededW = Math.round(childrenTotalW + pad.left + pad.right);
    if (neededW > parentW && neededW <= canvasWidth) {
      (parent as unknown as Record<string, unknown>).width = neededW;
    } else if (neededW > canvasWidth * 0.8) {
      (parent as unknown as Record<string, unknown>).width = 'fill_container';
    }
  }
}

function normalizeFormInputWidths(_parent: FrameNode, children: PenNode[]): void {
  const hasFillSibling = children.some(
    (c) => c.type === 'frame' && c.width === 'fill_container' && c.role !== 'divider',
  );
  if (!hasFillSibling) return;

  for (const child of children) {
    if (child.type !== 'frame') continue;
    if (child.role === 'divider') continue;
    if (child.role !== 'input' && child.role !== 'form-input') continue;
    if (typeof child.width !== 'number') continue;
    (child as unknown as Record<string, unknown>).width = 'fill_container';
  }
}

function normalizeInputTrailingIconAlignment(parent: FrameNode, children: PenNode[]): void {
  if (parent.role !== 'input' && parent.role !== 'form-input') return;
  if (parent.justifyContent && parent.justifyContent !== 'start') return;

  const visibleChildren = children.filter((c) => c.visible !== false);
  if (visibleChildren.length < 2) return;

  const trailing = visibleChildren[visibleChildren.length - 1];
  if (!isIconLikeNode(trailing)) return;

  const textChildren = visibleChildren.slice(0, -1).filter((child) => child.type === 'text');
  if (textChildren.length === 0) return;

  // Make text children fill available space so trailing icon is pushed to the
  // right edge while text stays left-aligned. This avoids the centering effect
  // that space_between causes with [icon, text, icon] layouts.
  for (const textChild of textChildren) {
    if (textChild.width !== 'fill_container') {
      (textChild as unknown as Record<string, unknown>).width = 'fill_container';
    }
    if (!textChild.textGrowth) {
      (textChild as unknown as Record<string, unknown>).textGrowth = 'fixed-width';
    }
  }
}

function isIconLikeNode(node: PenNode): boolean {
  if (node.type === 'path' || node.type === 'image') return true;

  if (node.type === 'frame') {
    if (node.role === 'icon' || node.role === 'icon-button') return true;
    const w = toSizeNumber(node.width, 0);
    const h = toSizeNumber(node.height, 0);
    if (w > 0 && h > 0 && Math.max(w, h) <= 32) return true;
  }

  return false;
}

function repairPlaceholderIcons(node: FrameNode, parentNode?: PenNode): void {
  if (!Array.isArray(node.children) || node.children.length === 0) return;

  for (const child of node.children) {
    if (!isPlaceholderCircleIcon(child)) continue;
    const semanticName = inferSemanticIconName(child, node, parentNode);
    if (!semanticName) continue;
    resolveIconPathBySemanticName(child as PathNode, semanticName);
  }
}

function isPlaceholderCircleIcon(node: PenNode): boolean {
  return (
    node.type === 'path' && (node.iconId === 'lucide:circle' || node.iconId === 'feather:circle')
  );
}

function inferSemanticIconName(
  node: PenNode,
  localParent: FrameNode,
  parentNode?: PenNode,
): string | null {
  const candidates = [
    node.name,
    localParent.name,
    ...collectNearbyText(localParent, 2, node),
    ...(parentNode ? collectNearbyText(parentNode, 2, localParent) : []),
    parentNode?.name,
  ]
    .filter((value): value is string => typeof value === 'string' && value.trim().length > 0)
    .map((value) => value.toLowerCase());

  for (const text of candidates) {
    if (/(run|jog|walk|hike|cardio|activity|exercise|training)/.test(text)) return 'activity';
    if (/(workout|workouts|gym|strength|dumbbell|barbell)/.test(text)) return 'dumbbell';
    if (/(yoga|meditation|stretch|profile|account|person|user)/.test(text)) return 'user';
    if (/(nutrition|meal|food|diet|apple|fruit)/.test(text)) return 'apple';
    if (/(today|sun|morning)/.test(text)) return 'sun';
  }

  return null;
}

function collectNearbyText(node: PenNode, depth: number, exclude?: PenNode): string[] {
  if (depth < 0 || node === exclude) return [];

  const out: string[] = [];
  if (node.type === 'text') {
    const content = getTextContentForNode(node).trim();
    if (content) out.push(content);
  } else if (typeof node.name === 'string' && node.name.trim()) {
    out.push(node.name.trim());
  }

  if ('children' in node && Array.isArray(node.children) && depth > 0) {
    for (const child of node.children) {
      if (child === exclude) continue;
      out.push(...collectNearbyText(child, depth - 1, exclude));
    }
  }

  return out;
}

function fixTextHeights(_parent: FrameNode, children: PenNode[], _canvasWidth: number): void {
  for (const child of children) {
    if (child.type !== 'text') continue;
    // Strip explicit pixel heights from text nodes — the layout engine auto-calculates
    // height from content + fontSize + lineHeight. Explicit heights always cause
    // clipping (height too small) or wasted space (height too large).
    if (typeof child.height === 'number' && child.textGrowth !== 'fixed-width-height') {
      delete (child as { height?: unknown }).height;
    }
  }
}
