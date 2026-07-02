import type { ElementTree } from './helpers.js';

export interface AvatarParams {
  initial?: string;
  size?: number;
}

/**
 * Circular avatar with optional centered initial. Emits
 * `frame(cornerRadius=size/2)` with flex centering — NEVER an
 * ellipse + sibling text (ellipse can't have children; sibling
 * text stacks above/below instead of centering). Default size 40.
 */
export function buildAvatar(params: AvatarParams): ElementTree {
  const size = params.size ?? 40;
  const children: ElementTree[] = [];
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
  return {
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
}
