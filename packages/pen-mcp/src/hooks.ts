import type { PenNode } from '@zseven-w/pen-types';

/**
 * Hooks 用于 AI
 * 支持的功能（角色解析、图标解析、节点清理）。 The Web 应用程序在启动时注入实现。 The CLI 无需它们即可运行。
 */
export interface McpHooks {
  /** Resolve 节点树上的语义角色（布局默认值、大小调整等） */
  resolveTreeRoles?: (nodes: PenNode[], canvasWidth: number) => void;
  /** Post-pass 角色解析（徽章叠加等） */
  resolveTreePostPass?: (nodes: PenNode[]) => void;
  /** Resolve 图标名称 → SVG 节点上的路径数据 */
  applyIconPathResolution?: (nodes: PenNode[]) => void;
  /** Replace 表情符号图标占位符与适当的图标节点 */
  applyNoEmojiIconHeuristic?: (nodes: PenNode[]) => void;
  /** Ensure 所有节点 IDs 都是唯一的 */
  ensureUniqueNodeIds?: (nodes: PenNode[]) => void;
  /** Sanitize 布局容器内的子位置 */
  sanitizeLayoutChildPositions?: (nodes: PenNode[]) => void;
  /** Sanitize 屏幕框架限制为合理的默认值 */
  sanitizeScreenFrameBounds?: (nodes: PenNode[]) => void;
  /** Register 角色定义（副作用） */
  registerRoleDefinitions?: () => void;
}

let _hooks: McpHooks = {};

/** Configure MCP 挂钩。 Call 从主机应用程序启动时一次。 */
export function configureMcpHooks(hooks: McpHooks): void {
  _hooks = hooks;
}

/** Get 当前的钩子实例。 */
export function getMcpHooks(): McpHooks {
  return _hooks;
}
