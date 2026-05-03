import type { ElementTree } from './helpers.js';

export interface CarouselDotsParams {
  total: number;
  current?: number;
}

/**
 * Carousel pagination dots. Active dot stretched into a 16×6 pill
 * (cornerRadius=3); inactive dots 6×6 circles. frame+cornerRadius
 * (not ellipse) per layout.md §RING.
 */
export function buildCarouselDots(params: CarouselDotsParams): ElementTree {
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const children: ElementTree[] = [];
  for (let i = 0; i < total; i += 1) {
    const isActive = i === current;
    children.push({
      type: 'frame',
      name: isActive ? 'Dot Active' : 'Dot',
      role: isActive ? 'dot-active' : 'dot',
      width: isActive ? 16 : 6,
      height: 6,
      cornerRadius: 3,
      fill: [{ type: 'solid', color: isActive ? '#111827' : '#D1D5DB' }],
    });
  }
  return {
    type: 'frame',
    name: 'Carousel Dots',
    role: 'carousel-dots',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    children,
  };
}
