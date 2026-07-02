import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface AvatarV1Params {
  initial?: string;
  size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_avatar_v0.
   * - `'dark'`: byte-parity with light (avatar has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildAvatar` emits no fill literals — the frame and text nodes carry no
   * fill, relying on the canvas default / parent context. Theme param is
   * accepted for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Circular avatar with optional centered initial — theme-aware version of
 * buildAvatar. Light mode is byte-equal to add_avatar_v0.
 *
 * Since buildAvatar emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildAvatarV1(params: AvatarV1Params): ElementTree {
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
