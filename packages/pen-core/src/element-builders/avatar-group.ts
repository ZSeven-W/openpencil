import type { ElementTree } from './helpers.js';

export interface AvatarGroupItem {
  /** Optional centered initial inside the circle. */
  initial?: string;
  /** Optional fill hex; falls back to a rotating palette so distinct
   *  avatars are visually distinguishable by default. */
  color?: string;
}

export interface AvatarGroupParams {
  items: AvatarGroupItem[];
  /** Avatar diameter in px. Default 32. Clamped 24..64. */
  size?: number;
  /** Cap on rendered avatars; the rest collapse into a "+N" tile.
   *  Default 4. Clamped 1..10. */
  max_visible?: number;
}

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
 * Stacked avatar group — team / collab / "+N more" presence indicator.
 * Renders up to `max_visible` filled circles with optional initials,
 * each ringed with a white stroke so they read as distinct tiles even
 * at a small gap. Items past the cap collapse into a slate "+N" tile
 * appended at the end.
 *
 * Implementation note: pen-core's flex layout doesn't support negative
 * gap (`Math.max(0, gap)` clamp in `layout/engine.ts`), so we can't
 * literally overlap the circles. The 2px white ring + 4px gap gives
 * the same "stacked" affordance that Slack / Linear / GitHub use,
 * staying inside the layout contract.
 */
export function buildAvatarGroup(params: AvatarGroupParams): ElementTree {
  const size = Math.min(64, Math.max(24, Math.floor(params.size ?? 32)));
  const maxVisible = Math.min(10, Math.max(1, Math.floor(params.max_visible ?? 4)));
  const items = params.items ?? [];
  const visible = items.slice(0, maxVisible);
  const overflow = Math.max(0, items.length - maxVisible);
  const fontSize = Math.max(11, Math.round(size * 0.4));
  const ring = { thickness: 2, fill: [{ type: 'solid', color: '#FFFFFF' }] };

  const children: ElementTree[] = visible.map((item, i) =>
    buildAvatarTile(item, i, size, fontSize, ring),
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
      fill: [{ type: 'solid', color: '#F1F5F9' }],
      stroke: ring,
      children: [
        {
          type: 'text',
          name: 'Count',
          role: 'avatar-group-overflow-count',
          content: `+${overflow}`,
          fontSize,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#475569' }],
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
  item: AvatarGroupItem,
  index: number,
  size: number,
  fontSize: number,
  ring: { thickness: number; fill: Array<{ type: string; color: string }> },
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
      fill: [{ type: 'solid', color: '#FFFFFF' }],
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
