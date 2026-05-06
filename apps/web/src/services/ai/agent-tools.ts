import type { AuthLevel } from '@/types/agent';

export interface ToolDef {
  name: string;
  description: string;
  level: AuthLevel;
  parameters: Record<string, unknown>;
}

const TOOL_AUTH_MAP: Record<string, AuthLevel> = {
  // 读
  batch_get: 'read',
  snapshot_layout: 'read',
  get_selection: 'read',
  get_variables: 'read',
  find_empty_space: 'read',
  get_design_prompt: 'read',
  list_theme_presets: 'read',
  get_design_md: 'read',

  // 创造
  plan_layout: 'create',
  batch_insert: 'create',
  insert_node: 'create',
  add_page: 'create',
  duplicate_page: 'create',
  import_svg: 'create',
  copy_node: 'create',
  save_theme_preset: 'create',
  generate_design: 'create',

  // 修改
  update_node: 'modify',
  replace_node: 'modify',
  move_node: 'modify',
  set_variables: 'modify',
  set_themes: 'modify',
  load_theme_preset: 'modify',
  rename_page: 'modify',
  reorder_page: 'modify',
  batch_design: 'modify',
  set_design_md: 'modify',
  export_design_md: 'modify',

  // 删除
  delete_node: 'delete',
  remove_page: 'delete',
};

// ---------------------------------------------------------------------------
// Intent 检测 — 确定加载哪些工具和提示
// ---------------------------------------------------------------------------

export type AgentIntent = 'design' | 'crud';

// CJK 字符不是 `\w`，因此 `\b` 边界对它们来说默默地失败。 Keep 和 English 列表边界锚定（避免 `app` 匹配
// `approach`、`add` 匹配 `address` 等），并为 CJK 关键字运行单独的无边界传递。
const DESIGN_KEYWORDS_EN =
  /\b(design|create|make|build|generate|add|insert|landing|page|screen|app|dashboard|card|hero|navbar|form|layout)\b/i;
const DESIGN_KEYWORDS_CJK =
  /(设计|创建|生成|画|做一个|新建|增加|添加|加一个|插入|页面|界面|登录|首页|仪表盘|卡片|表单)/;

/** Detect 用户的消息是设计意图还是 CRUD 操作。 */
export function detectAgentIntent(message: string): AgentIntent {
  return DESIGN_KEYWORDS_EN.test(message) || DESIGN_KEYWORDS_CJK.test(message) ? 'design' : 'crud';
}

// ---------------------------------------------------------------------------
// Tool 按意图定义
// ---------------------------------------------------------------------------

/** CRUD-only 工具 — 用于 read/update/delete/move/insert 操作的轻量级工具集。 */
export function getCrudToolDefs(): ToolDef[] {
  return [
    {
      name: 'batch_get',
      description:
        'Search and read nodes from the document. ALWAYS call this first before update_node or delete_node to find the correct node IDs. ' +
        'With no arguments, returns top-level children (current page structure). Search by type/name patterns or read specific IDs.',
      level: TOOL_AUTH_MAP.batch_get,
      parameters: {
        type: 'object',
        properties: {
          ids: { type: 'array', items: { type: 'string' }, description: 'Node IDs to retrieve' },
          patterns: {
            type: 'array',
            items: { type: 'string' },
            description: 'Search patterns to match',
          },
        },
      },
    },
    {
      name: 'snapshot_layout',
      description:
        'Get a compact layout snapshot of the current page showing node positions and sizes. ' +
        'When sibling nodes visually overlap, the result includes an `overlaps` array (parentId, a, b, reason) — ' +
        'use it as a text-only screenshot-replacement to diagnose visual bugs like stacked badges or overlapping text. ' +
        'If an overlap reason mentions `layout:"none"`, fix the PARENT frame (set layout to "vertical" or "horizontal" with a gap); ' +
        'do not just resize the overlapping children.',
      level: TOOL_AUTH_MAP.snapshot_layout,
      parameters: {
        type: 'object',
        properties: {
          pageId: { type: 'string' },
        },
      },
    },
    {
      name: 'insert_node',
      description:
        'Insert a new node into the document tree. Always call snapshot_layout or batch_get first. ' +
        'Use "after" to insert next to a sibling (auto-finds parent and position), or "parent" for explicit placement.',
      level: TOOL_AUTH_MAP.insert_node,
      parameters: {
        type: 'object',
        properties: {
          after: {
            type: 'string',
            description:
              'Insert after this sibling node ID (preferred). Automatically uses the same parent and places the new node right after it.',
          },
          parent: {
            type: ['string', 'null'],
            description:
              'Explicit parent node ID. Use "after" instead when adding next to existing elements.',
          },
          data: {
            type: 'object',
            description: 'PenNode data (type, name, width, height, fills, children, etc.)',
          },
        },
        required: ['data'],
      },
    },
    {
      name: 'update_node',
      description: 'Update properties of an existing node by ID',
      level: TOOL_AUTH_MAP.update_node,
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'Node ID to update' },
          data: { type: 'object', description: 'Properties to update' },
        },
        required: ['id', 'data'],
      },
    },
    {
      name: 'move_node',
      description:
        'Move a node to a different parent container. Use when you need to reparent a node (e.g. move an element into a frame). ' +
        "The node will be placed at the end of the parent's children list by default.",
      level: TOOL_AUTH_MAP.move_node,
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'Node ID to move' },
          parent: { type: 'string', description: 'New parent node ID' },
          index: { type: 'number', description: 'Position index within parent (optional)' },
        },
        required: ['id', 'parent'],
      },
    },
    {
      name: 'delete_node',
      description:
        'Delete a node (and all its children) from the document. ' +
        'Use when the user asks to remove, delete, or clear elements. ' +
        'Always call batch_get first to find the correct node ID before deleting.',
      level: TOOL_AUTH_MAP.delete_node,
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'Node ID to delete' },
        },
        required: ['id'],
      },
    },
    {
      name: 'get_selection',
      description: 'Get the currently selected nodes on the canvas with their full data',
      level: TOOL_AUTH_MAP.get_selection,
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  ];
}

/**
 * Design 工具 —
 *
 * 强制模型通过 `generate_design` 的最小集。 Weak 模型（例如 MiniMax-M2.7）在选择时更喜欢每个节点
 * `insert_node
 * ` 调用，从而产生分散的输出而不是连贯的设计。 Read 工具（`batch_get`、`snapshot_layout`）保留下来，以便模型可
 * 以在生成之前检查现有上下文。 CRUD 工具位于 `getCrudToolDefs()` 中，可通过
 * `detectAgentIntent('crud')` 进行手术编辑。
 */
export function getDesignToolDefs(): ToolDef[] {
  return [
    {
      name: 'batch_get',
      description:
        'Search and read nodes from the document. ALWAYS call this first before update_node or delete_node to find the correct node IDs. ' +
        'With no arguments, returns top-level children (current page structure). Search by type/name patterns or read specific IDs.',
      level: TOOL_AUTH_MAP.batch_get,
      parameters: {
        type: 'object',
        properties: {
          ids: { type: 'array', items: { type: 'string' }, description: 'Node IDs to retrieve' },
          patterns: {
            type: 'array',
            items: { type: 'string' },
            description: 'Search patterns to match',
          },
        },
      },
    },
    {
      name: 'snapshot_layout',
      description:
        'Get a compact layout snapshot of the current page showing node positions and sizes. ' +
        'When sibling nodes visually overlap, the result includes an `overlaps` array (parentId, a, b, reason) — ' +
        'use it as a text-only screenshot-replacement to diagnose visual bugs like stacked badges or overlapping text. ' +
        'If an overlap reason mentions `layout:"none"`, fix the PARENT frame (set layout to "vertical" or "horizontal" with a gap); ' +
        'do not just resize the overlapping children.',
      level: TOOL_AUTH_MAP.snapshot_layout,
      parameters: {
        type: 'object',
        properties: {
          pageId: { type: 'string' },
        },
      },
    },
    {
      name: 'generate_design',
      description:
        'Generate a complete design on the canvas. Pass a natural language description. The pipeline handles layout, styling, icons, and rendering. Always use this for creating designs.',
      level: TOOL_AUTH_MAP.generate_design,
      parameters: {
        type: 'object',
        properties: {
          prompt: {
            type: 'string',
            description:
              'Natural language description of the design, e.g. "a modern mobile login screen with email, password, login button, and social login"',
          },
        },
        required: ['prompt'],
      },
    },
  ];
}

/**
 * Builtin
 * 单代理流通过 `plan_layout` 创建框架，然后使用 `batch_insert` 插入内容。 Do 在这里不公开
 * `generate_design`，因为在内置模式下它只创建框架，而不是完整的设计操作。
 *
 */
export function getBuiltinLeadToolDefs(): ToolDef[] {
  return getAllToolDefs().filter((def) => def.name !== 'generate_design');
}

/** All 工具定义 — 主要注册机构和成员注册机构的规范模式源。 */
export function getAllToolDefs(): ToolDef[] {
  return [
    ...getDesignToolDefs(),
    {
      name: 'plan_layout',
      description:
        'Create a root design frame and return a section plan. Use this FIRST before generating content. Returns section names and the root frame ID. Call it again only when you intentionally want a new root frame/artboard.',
      level: TOOL_AUTH_MAP.plan_layout,
      parameters: {
        type: 'object',
        properties: {
          prompt: { type: 'string', description: 'Design description to plan layout for' },
          newRoot: {
            type: 'boolean',
            description:
              'Set true only when you intentionally want to create another root frame/artboard in the same session',
          },
        },
        required: ['prompt'],
      },
    },
    {
      name: 'batch_insert',
      description:
        'Insert PenNode objects into the canvas. MAX 9 nodes per call — call multiple times for more nodes. Each node needs id, type, name. Use _parent field to specify parent node ID.',
      level: TOOL_AUTH_MAP.batch_insert,
      parameters: {
        type: 'object',
        properties: {
          parentId: {
            type: ['string', 'null'],
            description: 'Parent frame ID to insert into (from plan_layout result)',
          },
          nodes: {
            type: 'array',
            items: { type: 'object' },
            description: 'Array of PenNode objects to insert',
          },
        },
        required: ['parentId', 'nodes'],
      },
    },
    {
      name: 'insert_node',
      description:
        'Insert a new node into the document tree. Always call snapshot_layout or batch_get first. ' +
        'Use "after" to insert next to a sibling (auto-finds parent and position), or "parent" for explicit placement.',
      level: TOOL_AUTH_MAP.insert_node,
      parameters: {
        type: 'object',
        properties: {
          after: {
            type: 'string',
            description:
              'Insert after this sibling node ID (preferred). Automatically uses the same parent and places the new node right after it.',
          },
          parent: {
            type: ['string', 'null'],
            description:
              'Explicit parent node ID. Use "after" instead when adding next to existing elements.',
          },
          data: {
            type: 'object',
            description: 'PenNode data (type, name, width, height, fills, children, etc.)',
          },
          pageId: {
            type: 'string',
            description: 'Target page ID (optional, defaults to active page)',
          },
        },
        required: ['data'],
      },
    },
    {
      name: 'find_empty_space',
      description: 'Find empty space on the canvas for placing new content',
      level: TOOL_AUTH_MAP.find_empty_space,
      parameters: {
        type: 'object',
        properties: {
          width: { type: 'number', description: 'Required width' },
          height: { type: 'number', description: 'Required height' },
          pageId: { type: 'string', description: 'Target page ID (optional)' },
        },
        required: ['width', 'height'],
      },
    },
    {
      name: 'get_selection',
      description: 'Get the currently selected nodes on the canvas with their full data',
      level: TOOL_AUTH_MAP.get_selection,
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  ];
}

export { TOOL_AUTH_MAP };
