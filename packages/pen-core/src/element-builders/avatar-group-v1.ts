import { coerceNonEmptyArray } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface AvatarGroupV1Item {
  /** Optional centered initial inside the circle. */
  initial?: string;
  /** Optional fill hex; falls back to a rotating palette so distinct
   *  avatars are visually distinguishable by default. */
  color?: string;
}

export interface AvatarGroupV1Params {
  items: AvatarGroupV1Item[];
  /** Avatar diameter in px. Default 32. Clamped 24..64. */
  size?: number;
  /** Cap on rendered avatars; the rest collapse into a "+N" tile.
   *  Default 4. Clamped 1..10. */
  max_visible?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_avatar_group_v0.
   * - `'dark'`: dark-mode fills for ring (surface dark), overflow bg
   *   (surface-2 dark), overflow text (textMuted dark), initial text
   *   (surface dark). Avatar palette colors are brand colors and stay
   *   hardcoded across all themes.
   * - `'system'`: emits `$color-*` ref strings for the non-brand slots.
   */
  theme?: V1Theme;
}

// Brand avatar palette — intentionally theme-independent across all modes
const DEFAULT_PALETTE = [
  '#3B82F6',
  '#10B981',
  '#F59E0B',
  '#EF4444',
  '#8B5CF6',
  '#EC4899',
  '#14B8A6',
  '#F97316',
];

/**
 * Stacked avatar group — theme-aware version of buildAvatarGroup.
 * Light mode is byte-equal to add_avatar_group_v0.
 *
 * The avatar palette colors (#3B82F6, #10B981, …) are brand tokens kept
 * hardcoded across all themes. Only the ring color (#FFFFFF ring → surface),
 * overflow bg (#F1F5F9 → surface-2), overflow text (#475569 → textMuted),
 * and initial text (#FFFFFF → surface) respond to theme mode.
 */
export function buildAvatarGroupV1(params: AvatarGroupV1Params): ElementTree {
  const size = Math.min(64, Math.max(24, Math.floor(params.size ?? 32)));
  const maxVisible = Math.min(10, Math.max(1, Math.floor(params.max_visible ?? 4)));
  const items = coerceNonEmptyArray<AvatarGroupV1Item>(
    params.items,
    [{ initial: 'A' }, { initial: 'B' }, { initial: 'C' }, { initial: 'D' }, { initial: 'E' }],
    'buildAvatarGroupV1',
    'items',
  );
  const visible = items.slice(0, maxVisible);
  const overflow = Math.max(0, items.length - maxVisible);
  const fontSize = Math.max(11, Math.round(size * 0.4));
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Light-mode constants (v0 byte-parity)
  const ringColor = isLight ? '#FFFFFF' : t.colors.surface;
  const overflowBg = isLight ? '#F1F5F9' : t.colors.surface2;
  const overflowText = isLight ? '#475569' : t.colors.textMuted;
  const initialText = isLight ? '#FFFFFF' : t.colors.surface;

  const ring = { thickness: 2, fill: [{ type: 'solid', color: ringColor }] };

  const children: ElementTree[] = visible.map((item, i) =>
    buildAvatarTile(item, i, size, fontSize, ring, initialText),
  );

  if (overflow > 0) {
    children.push({
      type: 'frame',
      name: `Overflow +${overflow}`,
      role: 'avatar-group-overflow',
      width: size,
      height: size,
      cornerRadius: size / 2,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [{ type: 'solid', color: overflowBg }],
      stroke: ring,
      children: [
        {
          type: 'text',
          name: 'Count',
          role: 'avatar-group-overflow-count',
          content: `+${overflow}`,
          fontSize,
          fontWeight: 600,
          fill: [{ type: 'solid', color: overflowText }],
        },
      ],
    });
  }

  return {
    type: 'frame',
    name: 'Avatar Group',
    role: 'avatar-group',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}

function buildAvatarTile(
  item: AvatarGroupV1Item,
  index: number,
  size: number,
  fontSize: number,
  ring: { thickness: number; fill: Array<{ type: string; color: string }> },
  initialText: string,
): ElementTree {
  const bg = item.color ?? DEFAULT_PALETTE[index % DEFAULT_PALETTE.length];
  const tileChildren: ElementTree[] = [];
  if (item.initial) {
    tileChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'avatar-group-initial',
      content: item.initial,
      fontSize,
      fontWeight: 600,
      fill: [{ type: 'solid', color: initialText }],
    });
  }
  return {
    type: 'frame',
    name: `Avatar ${index + 1}`,
    role: 'avatar-group-item',
    width: size,
    height: size,
    cornerRadius: size / 2,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    fill: [{ type: 'solid', color: bg }],
    stroke: ring,
    children: tileChildren,
  };
}
