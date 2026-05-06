import type { PenNode, PenFill, SolidFill } from '@zseven-w/pen-types';

/**
 * Strip 多余的“节级
 * ”从页面根框架的直接子级填充。 Weaker 子代理（MiniMax M2、GLM、Kimi）通常通过在它们生成的每个节根上编写硬编码
 *
 * 的暗十六进制（`#0A0A0A`
 * 、`#111`
 * 等）来进行对冲。然后 That 十六进制完全覆盖页面根部的预期背景颜色，破坏主题切换并在各部分之间创建可见的接缝。
 * Cards、按钮、芯片和其他合法填充的组件都会受到 NOT 的影响 - 仅“部分容器”框架（根的直接子级，没有角色或具有结构角色）。 ⚠️
 * SCOPE CONTRACT — 调用者 MUST 只传递真实的页面根框架。 Calling
 * 在任意嵌套框架（卡片、组件、子代理自己的根，即页面根）上使用 Calling 会错误地将该框架的内部子项视为“部分”，并且可能会删除预期的
 * 嵌套填充（例如，卡片自己的深色标题）。 The 函数本身是严格非递归的 - 它只触及传递节点的直接子节点 -
 * 因此错误目标的调用是有限的，但仍然是错误的。 Returns `true` 当任何填充被剥离时，因此调用者可以发布商店更新。
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
export function stripRedundantSectionFills(rootFrame: PenNode): boolean {
  if (!('children' in rootFrame) || !Array.isArray(rootFrame.children)) return false;

  const rootFill = getFirstSolidColor(rootFrame);
  let changed = false;

  for (const child of rootFrame.children) {
    if (child.type !== 'frame') continue;
    if (!isSectionLevelFrame(child)) continue;

    const childFill = getFirstSolidColor(child);
    if (!childFill) continue;

    if (shouldStripFill(childFill, rootFill)) {
      delete (child as PenNode & { fill?: unknown }).fill;
      changed = true;
    }
  }

  return changed;
}

/**
 * Roles 标识视觉上不
 * 同的组件，并且绝不能剥离其填充。
 */
const PROTECTED_ROLES = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
  'button',
  'icon-button',
  'badge',
  'chip',
  'tag',
  'pill',
  'input',
  'form-input',
  'search-bar',
  'phone-mockup',
  'banner',
  'metric-card',
  'gallery-item',
  'status-bar',
]);

/**
 * Roles 被认为是结构
 * 性的——只是一个将其他节点分组的容器。当 These 与根部背景或树篱相呼应时，它们是填充剥离的候选对象。
 *
 */
const STRUCTURAL_ROLES = new Set([
  'section',
  'row',
  'column',
  'stack',
  'container',
  'content-area',
  'section-header',
  'wrapper',
  'group',
  'hero',
  'footer',
  'cta-section',
  'stats-section',
]);

function isSectionLevelFrame(node: PenNode): boolean {
  const role = (node as PenNode & { role?: string }).role;
  if (!role) return true; // 展开的节根
  if (PROTECTED_ROLES.has(role)) return false;
  if (STRUCTURAL_ROLES.has(role)) return true;
  // Unknown 角色：保守一点，视为受保护，这样我们就不会破坏未来的角色添加。
  return false;
}

/**
 * Hex 是子代理在不知道
 * 真实设计背景颜色的情况下想要“安全深色”背景时所采用的色调。 Any 在一个部分容器上的这些几乎可以肯定是一种对冲，而不是有意的
 * 视觉选择。
 *
 */
const SAFE_DARK_HEXES = new Set([
  '#000000',
  '#000',
  '#0a0a0a',
  '#0f0f0f',
  '#111',
  '#111111',
  '#121212',
  '#141414',
  '#1a1a1a',
  '#181818',
  '#1c1c1c',
  '#1e1e1e',
  '#202020',
]);

/**
 * Light 模式与
 * SAFE_DARK_HEXES 对应。 Two 来源在部分根源上产生这些： 1. Weaker
 * 子代理，用纯白色/近白色代替正确透
 * 明填充的作用。 2. The 传统 `fixSectionAlternation` 后通道在每次运行 ≥3 个未填充部分时绘制
 * #FFFFFF / #F8FAFC 梯子。 On
 * 深色页面，梯子留下可见的白色条纹；如果在交替跳过着陆后重新打开或重新渲染具有这些过时填充的文档，则填充仍然存在，并且仅跳过不会
 * 删除它们。 Treat 这些与安全黑暗树篱相同：剥去任何部分的根部。
 *
 *
 *
 *
 */
const SAFE_LIGHT_HEXES = new Set([
  '#ffffff',
  '#fff',
  '#fefefe',
  '#fdfdfd',
  '#fcfcfc',
  '#fafafa',
  '#f9fafb',
  '#f8f8f8',
  '#f8fafc',
  '#f5f5f5',
  '#f4f4f5',
  '#f3f4f6',
]);

function shouldStripFill(childFill: string, rootFill: string | null): boolean {
  const childKey = normalizeHex(childFill);
  if (rootFill) {
    const rootKey = normalizeHex(rootFill);
    if (childKey === rootKey) return true;
  }
  return SAFE_DARK_HEXES.has(childKey) || SAFE_LIGHT_HEXES.has(childKey);
}

function normalizeHex(color: string): string {
  let c = color.trim().toLowerCase();
  // Strip alpha 如果存在 (#rrggbbaa → #rrggbb)
  if (c.length === 9 && c.startsWith('#')) c = c.slice(0, 7);
  return c;
}

function getFirstSolidColor(node: PenNode): string | null {
  const fill = (node as PenNode & { fill?: PenFill[] | string }).fill;
  if (!fill) return null;
  if (typeof fill === 'string') return fill;
  if (!Array.isArray(fill) || fill.length === 0) return null;
  const first = fill[0];
  if (first && first.type === 'solid') {
    return (first as SolidFill).color;
  }
  return null;
}
