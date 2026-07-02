import type { PenNode } from '@zseven-w/pen-types';

/**
 * Hooks for AI-powered features (role resolution, icon resolution, node sanitization).
 * The web app injects implementations at startup. The CLI can operate without them.
 */
export interface McpHooks {
  /** Resolve semantic roles on a node tree (layout defaults, sizing, etc.) */
  resolveTreeRoles?: (nodes: PenNode[], canvasWidth: number) => void;
  /** Post-pass role resolution (badge overlays, etc.) */
  resolveTreePostPass?: (nodes: PenNode[]) => void;
  /** Resolve icon name → SVG path data on nodes */
  applyIconPathResolution?: (nodes: PenNode[]) => void;
  /** Replace emoji icon placeholders with proper icon nodes */
  applyNoEmojiIconHeuristic?: (nodes: PenNode[]) => void;
  /** Ensure all node IDs are unique */
  ensureUniqueNodeIds?: (nodes: PenNode[]) => void;
  /** Sanitize child positions inside layout containers */
  sanitizeLayoutChildPositions?: (nodes: PenNode[]) => void;
  /** Sanitize screen frame bounds to sensible defaults */
  sanitizeScreenFrameBounds?: (nodes: PenNode[]) => void;
  /** Register role definitions (side-effect) */
  registerRoleDefinitions?: () => void;
}

let _hooks: McpHooks = {};

/** Configure MCP hooks. Call once at startup from the host app. */
export function configureMcpHooks(hooks: McpHooks): void {
  _hooks = hooks;
}

/** Get the current hooks instance. */
export function getMcpHooks(): McpHooks {
  return _hooks;
}
