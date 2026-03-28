import { z } from 'zod'
import { createToolRegistry } from '@zseven-w/agent'
import type { AuthLevel } from '@zseven-w/agent'

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
}

/**
 * Create a tool registry pre-loaded with MVP design tools.
 * Tool execute functions are NOT provided — they run on the client via ToolExecutor.
 */
export function createDesignToolRegistry() {
  const registry = createToolRegistry()

  // MVP tools (Phase 1: 6 tools)
  registry.register({
    name: 'batch_get',
    description: 'Get nodes by IDs or search patterns from the document tree',
    level: TOOL_AUTH_MAP.batch_get,
    schema: z.object({
      ids: z.array(z.string()).optional().describe('Node IDs to retrieve'),
      patterns: z.array(z.string()).optional().describe('Search patterns to match'),
    }),
  })

  registry.register({
    name: 'snapshot_layout',
    description: 'Get a compact layout snapshot of the current page showing node positions and sizes',
    level: TOOL_AUTH_MAP.snapshot_layout,
    schema: z.object({
      pageId: z.string().optional(),
    }),
  })

  // Design creation — delegates to the full internal pipeline (orchestrator + sub-agents)
  registry.register({
    name: 'generate_design',
    description: 'Generate a complete design on the canvas. Pass a natural language description. The pipeline handles layout, styling, icons, and rendering. Always use this for creating designs.',
    level: TOOL_AUTH_MAP.generate_design,
    schema: z.object({
      prompt: z.string().describe('Natural language description of the design, e.g. "a modern mobile login screen with email, password, login button, and social login"'),
    }),
  })

  // Modification tools — for editing existing designs
  registry.register({
    name: 'update_node',
    description: 'Update properties of an existing node by ID',
    level: TOOL_AUTH_MAP.update_node,
    schema: z.object({
      id: z.string().describe('Node ID to update'),
      data: z.record(z.unknown()).describe('Properties to update'),
    }),
  })

  registry.register({
    name: 'delete_node',
    description: 'Delete a node from the document by ID',
    level: TOOL_AUTH_MAP.delete_node,
    schema: z.object({
      id: z.string().describe('Node ID to delete'),
    }),
  })

  return registry
}

export { TOOL_AUTH_MAP }
