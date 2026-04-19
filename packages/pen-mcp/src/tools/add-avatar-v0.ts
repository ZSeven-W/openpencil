import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddAvatarV0Params {
  initial?: string;
  size?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Circular avatar with optional centered initial. Emits the pattern
 * taught in packages/pen-ai-skills/skills/phases/generation/layout.md
 * §RING / CIRCLE WITH CENTER CONTENT: `frame(cornerRadius=size/2)` with
 * flex centering — **NEVER** an ellipse + sibling text (ellipse can't
 * have children; sibling text stacks above/below instead of centering).
 *
 * Default size 40 (compact, e.g. inline in a list row). For larger
 * profile avatars pass `size: 56` or `size: 96`.
 *
 * Without `initial`: emits an empty circle (placeholder) — caller can
 * add an image child via batch_design U-op.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddAvatarV0(
  params: AddAvatarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const size = params.size ?? 40;
  const children: Record<string, unknown>[] = [];
  if (params.initial) {
    children.push({
      type: 'text',
      name: 'Initial',
      role: 'heading',
      content: params.initial,
      fontSize: Math.max(12, Math.round(size * 0.4)),
      fontWeight: 600,
    });
  }
  const avatar = {
    type: 'frame',
    name: 'Avatar',
    role: 'avatar',
    width: size,
    height: size,
    cornerRadius: size / 2,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children,
  };
  assignIdsRecursively(avatar);
  return insertElementTree({ binding: 'avatar', tree: avatar, ...params });
}
