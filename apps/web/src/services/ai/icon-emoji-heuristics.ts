import type { PenNode } from '@/types/pen';
import { clamp, toSizeNumber, extractPrimaryColor } from './generation-utils';
import { ICON_PATH_MAP } from './icon-dictionary';

// ---------------------------------------------------------------------------
// Emoji detection + fallback icon heuristics
// ---------------------------------------------------------------------------

const EMOJI_REGEX = /[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F]/gu;

/**
 * When a text node contains emoji characters, strip them.
 * If the entire content is emoji (no text remains), convert the node
 * to a fallback circle path icon in-place.
 */
export function applyNoEmojiIconHeuristic(node: PenNode): void {
  if (node.type !== 'text') return;
  if (typeof node.content !== 'string' || !node.content) return;

  EMOJI_REGEX.lastIndex = 0;
  if (!EMOJI_REGEX.test(node.content)) return;
  EMOJI_REGEX.lastIndex = 0;
  const cleaned = node.content
    .replace(EMOJI_REGEX, '')
    .replace(/\s{2,}/g, ' ')
    .trim();
  if (cleaned.length > 0) {
    node.content = cleaned;
    return;
  }

  const iconSize = clamp(
    toSizeNumber(node.height, toSizeNumber(node.width, node.fontSize ?? 20)),
    14,
    24,
  );
  const iconFill = extractPrimaryColor('fill' in node ? node.fill : undefined) ?? '#64748B';
  const fallbackCircle = ICON_PATH_MAP['circle'] ?? ICON_PATH_MAP['feather:circle'];
  const replacement: PenNode = {
    id: node.id,
    type: 'path',
    name: `${node.name ?? 'Icon'} Path`,
    d: fallbackCircle?.d ?? 'M 2 12 a 10 10 0 1 0 20 0 a 10 10 0 1 0 -20 0 Z',
    width: iconSize,
    height: iconSize,
    stroke:
      fallbackCircle?.style === 'stroke'
        ? { thickness: 2, fill: [{ type: 'solid', color: iconFill }] }
        : undefined,
    fill: fallbackCircle?.style === 'stroke' ? [] : [{ type: 'solid', color: iconFill }],
  } as PenNode;

  if (typeof node.x === 'number') replacement.x = node.x;
  if (typeof node.y === 'number') replacement.y = node.y;
  if (typeof node.opacity === 'number') replacement.opacity = node.opacity;
  if (typeof node.rotation === 'number') replacement.rotation = node.rotation;
  replaceNode(node, replacement);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Replace all properties of `target` with those of `replacement` in-place. */
function replaceNode(target: PenNode, replacement: PenNode): void {
  const targetRecord = target as unknown as Record<string, unknown>;
  for (const key of Object.keys(target)) {
    delete targetRecord[key];
  }
  Object.assign(targetRecord, replacement as unknown as Record<string, unknown>);
}
