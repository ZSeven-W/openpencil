import type { ToolResult } from '../../src/types/agent';

export type LayoutPhase = 'idle' | 'layout_done' | 'content_started';

export interface LayoutSessionState {
  layoutPhase: LayoutPhase;
  layoutRootId: string | null;
}

function parseToolInput(input: unknown): Record<string, unknown> | null {
  if (!input) return null;
  if (typeof input === 'string') {
    try {
      const parsed = JSON.parse(input);
      return parsed && typeof parsed === 'object' ? (parsed as Record<string, unknown>) : null;
    } catch {
      return null;
    }
  }
  return typeof input === 'object' ? (input as Record<string, unknown>) : null;
}

export function shouldShortCircuitPlanLayout(
  session: LayoutSessionState,
  toolName: string,
  input: unknown,
): ToolResult | null {
  if (toolName !== 'plan_layout') return null;
  if (session.layoutPhase === 'idle') return null;

  const parsed = parseToolInput(input);
  const newRoot = parsed?.newRoot === true;
  if (newRoot) return null;

  return {
    success: false,
    ...(session.layoutRootId ? { data: { rootFrameId: session.layoutRootId } } : {}),
    error:
      `Layout already exists for this session${session.layoutRootId ? ` (rootFrameId: ${session.layoutRootId})` : ''}. ` +
      'Use batch_insert or insert_node to add content to the existing frame. ' +
      'Only call plan_layout again with {"prompt": "...", "newRoot": true} if you intentionally want another root frame or artboard.',
  };
}

export function updateLayoutSessionState(
  session: LayoutSessionState,
  toolName: string | undefined,
  result: ToolResult,
): void {
  if (!toolName || !result?.success) return;

  if (toolName === 'plan_layout') {
    session.layoutPhase = 'layout_done';
    const rootFrameId = (result.data as { rootFrameId?: string } | undefined)?.rootFrameId;
    if (rootFrameId) session.layoutRootId = rootFrameId;
    return;
  }

  if (toolName === 'batch_insert' || toolName === 'insert_node') {
    session.layoutPhase = 'content_started';
  }
}
