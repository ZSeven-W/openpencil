import type { PenNode } from '@zseven-w/pen-types';

/**
 * Check 如果节点是使
 * 用绝对定位的覆盖层并且不应参与布局流。 Requires 显式 `role: 'overlay'`。 Earlier 版本与 `role:
 *
 * 'badge' | 'pill' | 'tag'`
 * 加上名称正则表达式匹配，
 * 但这些是此存储库中的内联组件标记（请参阅 `role-resolver.ts` 和
 * `strip-redundant-section-fills.ts` PROTECTED_ROLES） -
 * 将它们从布局流中拉出，将它们折叠到其父级的 (0,0) 并将它们堆叠在同级之上。 `role: 'overlay'`
 * 是通知点和真正浮动装饰的专用选择。
 *
 */
export function isOverlayNode(node: PenNode): boolean {
  if ('role' in node) {
    const role = (node as { role?: string }).role;
    if (role === 'overlay') return true;
  }
  return false;
}

/**
 * @deprecated Renamed 至 `isOverlayNode`。 Semantics 也收紧了：
 * 此别名不再为 `role: 'badge' | 'pill' | 'tag'` 返回 true
 * （这些是此存储库中的内联组件角色，应该流入
 * 自动布局，而不是浮动）。 Use `isOverlayNode` 并标记为 true 浮动
 * 装饰与 `role: 'overlay'`。
 */
export const isBadgeOverlayNode = isOverlayNode;

/**
 * Convert 到
 * PascalCase 的名称字符串。 Strips
 非字母数字字符并连接单词。
 */
export function sanitizeName(name: string): string {
  return name
    .replace(/[^a-zA-Z0-9\s-_]/g, '')
    .split(/[\s\-_]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join('');
}
