/**
 * Role definitions for the AI design generation system.
 * All registerRole() calls are consolidated here for runtime registration.
 *
 * Theme awareness: roles with visual defaults (cards, inputs, navbar,
 * dividers) read `ctx.theme` and pick a theme-appropriate fill/stroke.
 * Without this, an LLM generating a dark-theme design that omits the
 * fill on a card or navbar would have the role resolver paint a
 * `#FFFFFF` default on top of the dark page background — the visual
 * regression that motivated the change.
 *
 * The theme is detected once at the entry of `resolveTreeRoles` from
 * the page root fill (luminance < 0.3 = dark). All defaults below use
 * the helper accessors `cardFill / inputFill / navbarFill / etc.` to
 * stay consistent.
 */

import { registerRole } from '../role-resolver';
import { hasCjkText, getTextContentForNode } from '../generation-utils';
import type { PenFill, PenStroke, PenEffect } from '@/types/styles';

const CARD_FILL_LIGHT: PenFill[] = [{ type: 'solid', color: '#FFFFFF' }];
const CARD_FILL_DARK: PenFill[] = [{ type: 'solid', color: '#1A1A1A' }];
const cardFill = (theme?: 'dark' | 'light'): PenFill[] =>
  theme === 'dark' ? CARD_FILL_DARK : CARD_FILL_LIGHT;

const CARD_SHADOW: PenEffect[] = [
  { type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: '#0000001A' },
  { type: 'shadow', offsetX: 0, offsetY: 1, blur: 2, spread: -1, color: '#0000000F' },
];

const INPUT_FILL_LIGHT: PenFill[] = [{ type: 'solid', color: '#F8FAFC' }];
const INPUT_FILL_DARK: PenFill[] = [{ type: 'solid', color: '#1A1A1A' }];
const inputFill = (theme?: 'dark' | 'light'): PenFill[] =>
  theme === 'dark' ? INPUT_FILL_DARK : INPUT_FILL_LIGHT;

const INPUT_STROKE_LIGHT: PenStroke = {
  thickness: 1,
  fill: [{ type: 'solid', color: '#E2E8F0' }],
};
const INPUT_STROKE_DARK: PenStroke = {
  thickness: 1,
  fill: [{ type: 'solid', color: '#2A2A2A' }],
};
const inputStroke = (theme?: 'dark' | 'light'): PenStroke =>
  theme === 'dark' ? INPUT_STROKE_DARK : INPUT_STROKE_LIGHT;

const navbarFill = (theme?: 'dark' | 'light'): PenFill[] =>
  theme === 'dark' ? [{ type: 'solid', color: '#111111' }] : [{ type: 'solid', color: '#FFFFFF' }];

const navbarBottomBorder = (theme?: 'dark' | 'light'): PenStroke => ({
  thickness: [0, 0, 1, 0] as [number, number, number, number],
  fill: [{ type: 'solid', color: theme === 'dark' ? '#1F1F1F' : '#E2E8F0' }],
});

const dividerFill = (theme?: 'dark' | 'light'): PenFill[] => [
  { type: 'solid', color: theme === 'dark' ? '#2A2A2A' : '#E2E8F0' },
];

// ---------------------------------------------------------------------------
// Layout roles
// ---------------------------------------------------------------------------

registerRole('section', (_node, ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  gap: 24,
  padding: ctx.canvasWidth <= 480 ? ([40, 16] as [number, number]) : ([60, 80] as [number, number]),
  alignItems: 'center',
}));

registerRole('row', (_node, _ctx) => ({
  layout: 'horizontal',
  width: 'fill_container' as const,
  gap: 16,
  alignItems: 'center',
}));

registerRole('column', (_node, _ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  gap: 16,
}));

registerRole('centered-content', (_node, ctx) => ({
  layout: 'vertical',
  width: ctx.canvasWidth <= 480 ? ('fill_container' as const) : 1080,
  gap: 24,
  alignItems: 'center',
}));

registerRole('form-group', (_node, _ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  gap: 16,
}));

registerRole('spacer', (_node, _ctx) => ({
  width: 'fill_container' as const,
  height: 40,
}));

registerRole('divider', (node, ctx) => {
  const isVertical = node.name?.toLowerCase().includes('vertical');
  if (isVertical) {
    return {
      width: 1,
      height: 'fill_container' as const,
      layout: 'none' as const,
      fill: dividerFill(ctx.theme),
    };
  }
  return {
    width: 'fill_container' as const,
    height: 1,
    layout: 'none' as const,
    fill: dividerFill(ctx.theme),
  };
});

// ---------------------------------------------------------------------------
// Navigation roles
// ---------------------------------------------------------------------------

registerRole('navbar', (_node, ctx) => ({
  layout: 'horizontal',
  width: 'fill_container' as const,
  height: ctx.canvasWidth <= 480 ? 56 : 72,
  padding: ctx.canvasWidth <= 480 ? ([0, 16] as [number, number]) : ([0, 80] as [number, number]),
  alignItems: 'center',
  justifyContent: 'space_between' as const,
  fill: navbarFill(ctx.theme),
  stroke: navbarBottomBorder(ctx.theme),
}));

registerRole('nav-links', (_node, _ctx) => ({
  layout: 'horizontal',
  gap: 24,
  alignItems: 'center',
}));

registerRole('nav-link', (_node, _ctx) => ({
  textGrowth: 'auto' as const,
  lineHeight: 1.2,
}));

// ---------------------------------------------------------------------------
// Interactive roles
// ---------------------------------------------------------------------------

registerRole('button', (_node, ctx) => {
  if (ctx.parentRole === 'navbar') {
    return {
      padding: [8, 16] as [number, number],
      height: 36,
      layout: 'horizontal' as const,
      gap: 8,
      alignItems: 'center' as const,
      justifyContent: 'center' as const,
      cornerRadius: 8,
    };
  }
  if (ctx.parentRole === 'form-group') {
    return {
      width: 'fill_container' as const,
      height: 48,
      layout: 'horizontal' as const,
      gap: 8,
      padding: [12, 24] as [number, number],
      alignItems: 'center' as const,
      justifyContent: 'center' as const,
      cornerRadius: 10,
    };
  }
  return {
    padding: [12, 24] as [number, number],
    height: 44,
    layout: 'horizontal' as const,
    gap: 8,
    alignItems: 'center' as const,
    justifyContent: 'center' as const,
    cornerRadius: 8,
  };
});

registerRole('icon-button', (_node, _ctx) => ({
  width: 44,
  height: 44,
  layout: 'horizontal' as const,
  justifyContent: 'center' as const,
  alignItems: 'center' as const,
  cornerRadius: 8,
}));

registerRole('badge', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [6, 12] as [number, number],
  gap: 4,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 999,
  fill: [{ type: 'solid', color: '#DBEAFE' }] as PenFill[],
}));

registerRole('tag', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [4, 10] as [number, number],
  gap: 4,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 6,
}));

registerRole('pill', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [6, 14] as [number, number],
  gap: 6,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 999,
}));

registerRole('input', (_node, ctx) => {
  if (ctx.parentLayout === 'vertical') {
    return {
      width: 'fill_container' as const,
      height: 48,
      layout: 'horizontal' as const,
      padding: [12, 16] as [number, number],
      alignItems: 'center' as const,
      cornerRadius: 8,
      fill: inputFill(ctx.theme),
      stroke: inputStroke(ctx.theme),
    };
  }
  return {
    height: 48,
    layout: 'horizontal' as const,
    padding: [12, 16] as [number, number],
    alignItems: 'center' as const,
    cornerRadius: 8,
    fill: inputFill(ctx.theme),
    stroke: inputStroke(ctx.theme),
  };
});

registerRole('form-input', (_node, ctx) => ({
  width: 'fill_container' as const,
  height: 48,
  layout: 'horizontal' as const,
  padding: [12, 16] as [number, number],
  alignItems: 'center' as const,
  cornerRadius: 8,
  fill: inputFill(ctx.theme),
  stroke: inputStroke(ctx.theme),
}));

registerRole('search-bar', (_node, ctx) => ({
  layout: 'horizontal' as const,
  height: 44,
  padding: [10, 16] as [number, number],
  gap: 8,
  alignItems: 'center' as const,
  cornerRadius: 22,
  fill: inputFill(ctx.theme),
  stroke: inputStroke(ctx.theme),
}));

// ---------------------------------------------------------------------------
// Display roles
// ---------------------------------------------------------------------------

registerRole('card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 12,
      cornerRadius: 12,
      clipContent: true,
      fill: cardFill(ctx.theme),
      effects: CARD_SHADOW,
    };
  }
  return {
    layout: 'vertical' as const,
    gap: 12,
    cornerRadius: 12,
    clipContent: true,
    fill: cardFill(ctx.theme),
    effects: CARD_SHADOW,
  };
});

registerRole('stat-card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 8,
      padding: [24, 24] as [number, number],
      cornerRadius: 12,
      fill: cardFill(ctx.theme),
      effects: CARD_SHADOW,
    };
  }
  return {
    layout: 'vertical' as const,
    gap: 8,
    padding: [24, 24] as [number, number],
    cornerRadius: 12,
    fill: cardFill(ctx.theme),
    effects: CARD_SHADOW,
  };
});

registerRole('pricing-card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 16,
      padding: [32, 24] as [number, number],
      cornerRadius: 16,
      clipContent: true,
      fill: cardFill(ctx.theme),
      effects: CARD_SHADOW,
    };
  }
  return {
    layout: 'vertical' as const,
    gap: 16,
    padding: [32, 24] as [number, number],
    cornerRadius: 16,
    clipContent: true,
    fill: cardFill(ctx.theme),
    effects: CARD_SHADOW,
  };
});

registerRole('image-card', (_node, _ctx) => ({
  layout: 'vertical' as const,
  gap: 0,
  cornerRadius: 12,
  clipContent: true,
  effects: CARD_SHADOW,
}));

// ---------------------------------------------------------------------------
// Content roles
// ---------------------------------------------------------------------------

registerRole('hero', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding: ctx.canvasWidth <= 480 ? ([40, 16] as [number, number]) : ([80, 80] as [number, number]),
  gap: 24,
  alignItems: 'center',
}));

registerRole('feature-grid', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  gap: 24,
  alignItems: 'start' as const,
}));

registerRole('feature-card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 12,
      padding: [24, 24] as [number, number],
      cornerRadius: 12,
      fill: cardFill(ctx.theme),
      effects: CARD_SHADOW,
    };
  }
  return {
    layout: 'vertical' as const,
    gap: 12,
    padding: [24, 24] as [number, number],
    cornerRadius: 12,
    fill: cardFill(ctx.theme),
    effects: CARD_SHADOW,
  };
});

registerRole('testimonial', (_node, ctx) => ({
  layout: 'vertical' as const,
  gap: 16,
  padding: [24, 24] as [number, number],
  cornerRadius: 12,
  fill: cardFill(ctx.theme),
  effects: CARD_SHADOW,
}));

registerRole('cta-section', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding: ctx.canvasWidth <= 480 ? ([40, 16] as [number, number]) : ([60, 80] as [number, number]),
  gap: 20,
  alignItems: 'center',
}));

registerRole('footer', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding: ctx.canvasWidth <= 480 ? ([32, 16] as [number, number]) : ([48, 80] as [number, number]),
  gap: 24,
}));

registerRole('stats-section', (_node, ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding: ctx.canvasWidth <= 480 ? ([32, 16] as [number, number]) : ([48, 80] as [number, number]),
  gap: 32,
  justifyContent: 'center' as const,
  alignItems: 'center' as const,
}));

// ---------------------------------------------------------------------------
// Media roles
// ---------------------------------------------------------------------------

registerRole('phone-mockup', (_node, _ctx) => ({
  width: 280,
  height: 560,
  cornerRadius: 32,
  layout: 'none' as const,
}));

registerRole('screenshot-frame', (_node, _ctx) => ({
  cornerRadius: 12,
  clipContent: true,
}));

registerRole('avatar', (node, _ctx) => {
  const rawWidth = 'width' in node ? node.width : undefined;
  const size = typeof rawWidth === 'number' && rawWidth > 0 ? rawWidth : 48;
  return {
    width: size,
    height: size,
    cornerRadius: Math.round(size / 2),
    clipContent: true,
  };
});

registerRole('icon', (node, _ctx) =>
  node.type === 'frame'
    ? {
        width: 24,
        height: 24,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
      }
    : {
        width: 24,
        height: 24,
      },
);

// ---------------------------------------------------------------------------
// Typography roles
// ---------------------------------------------------------------------------

registerRole('heading', (node, ctx) => {
  const text = getTextContentForNode(node);
  const isCjk = hasCjkText(text);
  return {
    lineHeight: isCjk ? 1.35 : 1.2,
    letterSpacing: isCjk ? 0 : -0.5,
    textGrowth: ctx.parentLayout === 'vertical' ? ('fixed-width' as const) : ('auto' as const),
    width: ctx.parentLayout === 'vertical' ? ('fill_container' as const) : undefined,
  };
});

registerRole('subheading', (node, _ctx) => {
  const text = getTextContentForNode(node);
  const isCjk = hasCjkText(text);
  return {
    lineHeight: isCjk ? 1.4 : 1.3,
    textGrowth: 'fixed-width' as const,
    width: 'fill_container' as const,
  };
});

registerRole('body-text', (node, _ctx) => {
  const text = getTextContentForNode(node);
  const isCjk = hasCjkText(text);
  return {
    lineHeight: isCjk ? 1.6 : 1.5,
    textGrowth: 'fixed-width' as const,
    width: 'fill_container' as const,
  };
});

registerRole('caption', (node, _ctx) => {
  const text = getTextContentForNode(node);
  const isCjk = hasCjkText(text);
  return {
    lineHeight: isCjk ? 1.4 : 1.3,
    textGrowth: 'auto' as const,
  };
});

registerRole('label', (_node, _ctx) => ({
  lineHeight: 1.2,
  textGrowth: 'auto' as const,
  textAlignVertical: 'middle' as const,
}));

// ---------------------------------------------------------------------------
// Table roles
// ---------------------------------------------------------------------------

registerRole('table', (_node, _ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  gap: 0,
  clipContent: true,
}));

registerRole('table-row', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  alignItems: 'center' as const,
  padding: [12, 16] as [number, number],
}));

registerRole('table-header', (_node, ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  alignItems: 'center' as const,
  padding: [12, 16] as [number, number],
  fill: inputFill(ctx.theme),
}));

registerRole('table-cell', (_node, _ctx) => ({
  width: 'fill_container' as const,
}));
