import type { AuthLevel } from '@/types/agent';

export interface ToolDef {
  name: string;
  description: string;
  level: AuthLevel;
  parameters: Record<string, unknown>;
}

const TOOL_AUTH_MAP: Record<string, AuthLevel> = {
  // read
  batch_get: 'read',
  snapshot_layout: 'read',
  get_selection: 'read',
  get_variables: 'read',
  find_empty_space: 'read',
  get_design_prompt: 'read',
  list_theme_presets: 'read',
  get_design_md: 'read',

  // create
  plan_layout: 'create',
  batch_insert: 'create',
  insert_node: 'create',
  add_page: 'create',
  duplicate_page: 'create',
  import_svg: 'create',
  copy_node: 'create',
  save_theme_preset: 'create',
  generate_design: 'create',

  // modify
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

  // delete
  delete_node: 'delete',
  remove_page: 'delete',
};

export function getDesignToolDefs(): ToolDef[] {
  return [
    {
      name: 'batch_get',
      description: 'Get nodes by IDs or search patterns from the document tree',
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
        'Get a compact layout snapshot of the current page showing node positions and sizes',
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
      name: 'delete_node',
      description: 'Delete a node from the document by ID',
      level: TOOL_AUTH_MAP.delete_node,
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'Node ID to delete' },
        },
        required: ['id'],
      },
    },
  ];
}

/**
 * Builtin single-agent flows create the frame via `plan_layout` and then
 * insert content with `batch_insert`. Do not expose `generate_design` here,
 * because in builtin mode it only creates the frame and is not a complete
 * design operation.
 */
export function getBuiltinLeadToolDefs(): ToolDef[] {
  return getAllToolDefs().filter((def) => def.name !== 'generate_design');
}

/** All tool definitions — canonical schema source for both lead and member registries. */
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
      description: 'Insert a new node into the document tree with full support for nested children',
      level: TOOL_AUTH_MAP.insert_node,
      parameters: {
        type: 'object',
        properties: {
          parent: {
            type: ['string', 'null'],
            description: 'Parent node ID, or null for root-level insertion',
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
        required: ['parent', 'data'],
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
