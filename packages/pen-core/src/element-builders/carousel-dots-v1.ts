import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CarouselDotsV1Params {
  total: number;
  current?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_carousel_dots_v0.
   * - `'dark'`: dark-mode fills — active dot uses $color-text-primary dark
   *   (#F1F5F9), inactive uses $color-border dark (#334155).
   * - `'system'`: emits `$color-*` ref strings for active and inactive dot fills.
   */
  theme?: V1Theme;
}

/**
 * Carousel pagination dots — theme-aware version of buildCarouselDots.
 * Light mode is byte-equal to add_carousel_dots_v0.
 * Active dot = stretched 16×6 pill; inactive = 6×6 circle (frame+cornerRadius).
 */
export function buildCarouselDotsV1(params: CarouselDotsV1Params): ElementTree {
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Active dot color: light=#111827 (v0 byte-parity)
  // dark/system: use textPrimary (close enough — active dot stands out against surface)
  const activeFill = isLight ? '#111827' : t.colors.textPrimary;
  // Inactive dot color: light=#D1D5DB (slate-300), dark/system: border
  const inactiveFill = isLight ? '#D1D5DB' : t.colors.border;

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
      fill: [{ type: 'solid', color: isActive ? activeFill : inactiveFill }],
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
