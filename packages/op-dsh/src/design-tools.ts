/** Model-facing tools that directly drive the active OpenPencil canvas. */

import { defineTool, type ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { JsonValue } from './json-value.js'
import type { EditorHostController } from './editor-host.js'
import { resolveInputFile } from './renderer.js'
import type { OpenPencilSelectionSnapshot } from './mcp-client.js'
import {
  OPENPENCIL_CREATE_TOOL_NAME,
  OPENPENCIL_EDIT_TOOL_NAME,
  OPENPENCIL_SELECTION_TOOL_NAME,
} from './tool-names.js'

const MAX_OPERATIONS_LENGTH = 256 * 1024

function sessionWorkspace(exec: ToolRunContext): string {
  return exec.agent?.session.header.cwd ?? process.cwd()
}

async function expectedSource(path: string | undefined, exec: ToolRunContext): Promise<string | undefined> {
  return path === undefined ? undefined : resolveInputFile(path, sessionWorkspace(exec))
}

function ownerSessionId(exec: ToolRunContext): string | undefined {
  return exec.agent === undefined ? undefined : String(exec.agent.id)
}

const renderJson = (_args: unknown, value: unknown): [{ type: 'text'; text: string }] => [{
  type: 'text',
  text: JSON.stringify(value, null, 2),
}]

const nodeSchema = {
  type: 'object' as const,
  additionalProperties: false,
  properties: {
    id: { type: 'string' as const, required: true as const },
    type: { type: 'string' as const },
    name: { type: 'string' as const },
    x: { type: 'number' as const },
    y: { type: 'number' as const },
    width: { type: 'number' as const },
    height: { type: 'number' as const },
  },
} as const

const selectionProperties = {
  sourcePath: { type: 'string' as const, required: true as const },
  activePageId: { type: 'string' as const, required: true as const },
  selectedIds: { type: 'array' as const, items: { type: 'string' as const }, required: true as const },
  nodes: { type: 'array' as const, items: nodeSchema, required: true as const },
  count: { type: 'integer' as const, required: true as const },
  updatedAt: { type: 'integer' as const, required: true as const },
} as const

function selectionResult(selection: OpenPencilSelectionSnapshot) {
  return { ...selection, count: selection.selectedIds.length }
}

/** Read the exact selection the user currently has on the editable canvas. */
export function createDesignSelectionTool(editorHost: EditorHostController) {
  return defineTool({
    name: OPENPENCIL_SELECTION_TOOL_NAME,
    description: 'Read the current selection from the live OpenPencil sidebar canvas. '
      + 'Use this before modifying a node. Returns selected node ids, names, types, bounds, and the active page. '
      + 'The editable sidebar must already be open. If path is supplied, the tool refuses to target a different canvas.',
    parameters: {
      path: { type: 'string', description: 'Optional .op path used to assert which live editor is being read.' },
    },
    output: {
      schema: {
        type: 'object', additionalProperties: false, properties: selectionProperties,
      },
      render: renderJson,
    },
    async execute(args: { path?: string }, exec) {
      const sourcePath = await expectedSource(args.path, exec)
      const selection = await editorHost.getActiveSelection({ sourcePath, ownerSessionId: ownerSessionId(exec), signal: exec.signal })
      return selectionResult(selection)
    },
    presentCall: () => ({ card: 'generic', title: 'Read OpenPencil selection', kind: 'read' }),
  })
}

export interface DesignCreateArgs {
  path: string
  operations: string
  pageId?: string
  canvasWidth?: number
  postProcess?: boolean
}

/** Apply one transactional OpenPencil batch program to generate design nodes. */
export function createDesignCreateTool(editorHost: EditorHostController) {
  return defineTool({
    name: OPENPENCIL_CREATE_TOOL_NAME,
    description: 'Generate or restructure design nodes on an existing active OpenPencil canvas using one transactional batch_design program. '
      + 'Use this only after the editable sidebar is already open. For an ordinary new design with no .op file or live editor, use openpencil_pipeline_begin; openpencil_new is only for an explicitly requested simple one-shot compatibility path. '
      + 'Use concise newline-separated operations: '
      + 'I(parentId, nodeJson) inserts, U(nodeId, patchJson) updates, D(nodeId) deletes, '
      + 'M(nodeId,parentId,index) moves, C(nodeId,parentId,overrides) clones, and R(nodeId,nodeJson) replaces. '
      + 'All operations apply together or none apply. The canvas updates live but remains unsaved until the sidebar Save action.',
    parameters: {
      path: { type: 'string', required: true, description: 'The .op path used to bind this write to the intended live editor.' },
      operations: {
        type: 'string', required: true,
        description: 'Newline-separated batch_design operations, for example I("page-id", {"type":"frame","id":"hero","name":"Hero","x":0,"y":0,"width":1200,"height":800}).',
      },
      pageId: { type: 'string', description: 'Optional page id. Defaults to the live editor active page.' },
      canvasWidth: { type: 'number', description: 'Optional canvas width hint for post-processing.' },
      postProcess: { type: 'boolean', description: 'Run OpenPencil post-processing after the batch. Default false.' },
    },
    output: {
      schema: {
        type: 'object', additionalProperties: false,
        properties: {
          ...selectionProperties,
          applied: { type: 'boolean', const: true, required: true },
          dirty: { type: 'boolean', const: true, required: true },
          saved: { type: 'boolean', const: false, required: true },
          result: { type: 'object', additionalProperties: true },
          note: { type: 'string', required: true },
        },
      },
      render: renderJson,
    },
    async execute(args: DesignCreateArgs, exec) {
      if (args.operations.trim().length === 0) throw new Error(`${OPENPENCIL_CREATE_TOOL_NAME}: operations must not be empty`)
      if (args.operations.length > MAX_OPERATIONS_LENGTH) throw new Error(`${OPENPENCIL_CREATE_TOOL_NAME}: operations are too large`)
      const sourcePath = await expectedSource(args.path, exec)
      const owner = ownerSessionId(exec)
      const before = await editorHost.getActiveSelection({ sourcePath, ownerSessionId: owner, signal: exec.signal })
      const result = await editorHost.callActiveMcp('batch_design', {
        operations: args.operations,
        ...(args.pageId !== undefined && args.pageId.length > 0
          ? { pageId: args.pageId }
          : before.activePageId === '' ? {} : { pageId: before.activePageId }),
        ...(args.canvasWidth === undefined ? {} : { canvasWidth: args.canvasWidth }),
        ...(args.postProcess === undefined ? {} : { postProcess: args.postProcess }),
      }, { sourcePath, ownerSessionId: owner, signal: exec.signal })
      const selection = await editorHost.getActiveSelection({ sourcePath, ownerSessionId: owner, signal: exec.signal })
      return {
        ...selectionResult(selection),
        applied: true as const,
        dirty: true as const,
        saved: false as const,
        ...(typeof result.value === 'object' && result.value !== null && !Array.isArray(result.value)
          ? { result: result.value as Record<string, JsonValue> }
          : {}),
        note: 'Applied to the live OpenPencil canvas. Use Save in the sidebar to persist the .op file.',
      }
    },
    presentCall: () => ({ card: 'generic', title: 'Generate on OpenPencil canvas', kind: 'execute' }),
  })
}

export interface DesignEditArgs {
  path: string
  nodeId?: string
  changes: Record<string, JsonValue>
}

/** Patch one explicit node, or the single node the user selected. */
export function createDesignEditTool(editorHost: EditorHostController) {
  return defineTool({
    name: OPENPENCIL_EDIT_TOOL_NAME,
    description: 'Modify one node directly on the active OpenPencil canvas. '
      + 'When nodeId is omitted, this tool targets the single node currently selected by the user; '
      + 'it refuses an empty or multi-selection instead of guessing. Pass rich OpenPencil node fields in changes, '
      + 'for example content, name, x, y, width, height, fill, fontSize, padding, layout, or opacity. '
      + 'The canvas updates live but remains unsaved until the sidebar Save action.',
    parameters: {
      path: { type: 'string', required: true, description: 'The .op path used to bind this write to the intended live editor.' },
      nodeId: { type: 'string', description: 'Explicit target node id. Omit to use the one selected canvas node.' },
      changes: {
        type: 'object', additionalProperties: true, required: true,
        description: 'Shallow OpenPencil node patch. Rich canonical fields are preserved.',
      },
    },
    output: {
      schema: {
        type: 'object', additionalProperties: false,
        properties: {
          ...selectionProperties,
          applied: { type: 'boolean', const: true, required: true },
          dirty: { type: 'boolean', const: true, required: true },
          saved: { type: 'boolean', const: false, required: true },
          targetNodeId: { type: 'string', required: true },
          targetSource: { type: 'string', enum: ['explicit', 'selection'], required: true },
          result: { type: 'object', additionalProperties: true },
          note: { type: 'string', required: true },
        },
      },
      render: renderJson,
    },
    async execute(args: DesignEditArgs, exec) {
      if (Object.keys(args.changes).length === 0) throw new Error(`${OPENPENCIL_EDIT_TOOL_NAME}: changes must not be empty`)
      const sourcePath = await expectedSource(args.path, exec)
      const owner = ownerSessionId(exec)
      const before = await editorHost.getActiveSelection({ sourcePath, ownerSessionId: owner, signal: exec.signal })
      const explicit = args.nodeId?.trim()
      let targetNodeId: string
      let targetSource: 'explicit' | 'selection'
      if (explicit !== undefined && explicit.length > 0) {
        if (!before.selectedIds.includes(explicit)) {
          throw new Error(`${OPENPENCIL_EDIT_TOOL_NAME}: nodeId ${explicit} is not in the current canvas selection`)
        }
        targetNodeId = explicit
        targetSource = 'explicit'
      } else {
        if (before.selectedIds.length === 0) {
          throw new Error(`${OPENPENCIL_EDIT_TOOL_NAME}: no canvas node is selected; select one in OpenPencil or pass nodeId`)
        }
        if (before.selectedIds.length !== 1) {
          throw new Error(`${OPENPENCIL_EDIT_TOOL_NAME}: ${before.selectedIds.length} nodes are selected; pass nodeId to choose one`)
        }
        targetNodeId = before.selectedIds[0]!
        targetSource = 'selection'
      }
      const result = await editorHost.callActiveMcp('update_node', {
        nodeId: targetNodeId,
        data: args.changes,
        ...(before.activePageId === '' ? {} : { pageId: before.activePageId }),
      }, { sourcePath, ownerSessionId: owner, signal: exec.signal })
      const selection = await editorHost.getActiveSelection({ sourcePath, ownerSessionId: owner, signal: exec.signal })
      return {
        ...selectionResult(selection),
        applied: true as const,
        dirty: true as const,
        saved: false as const,
        targetNodeId,
        targetSource,
        ...(typeof result.value === 'object' && result.value !== null && !Array.isArray(result.value)
          ? { result: result.value as Record<string, JsonValue> }
          : {}),
        note: 'Applied to the live OpenPencil canvas. Use Save in the sidebar to persist the .op file.',
      }
    },
    presentCall: (args: DesignEditArgs) => ({
      card: 'generic', title: args.nodeId === undefined ? 'Edit selected OpenPencil node' : `Edit OpenPencil node ${args.nodeId}`, kind: 'execute',
    }),
  })
}
