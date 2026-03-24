/**
 * Role definitions for the AI design generation system.
 * All registerRole() calls are consolidated here for runtime registration.
 */

import { registerRole } from '../role-resolver'
import { hasCjkText, getTextContentForNode } from '../generation-utils'

// ---------------------------------------------------------------------------
// Layout roles
// ---------------------------------------------------------------------------

registerRole('section', (_node, ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  gap: 24,
  padding:
    ctx.canvasWidth <= 480
      ? ([40, 16] as [number, number])
      : ([60, 80] as [number, number]),
  alignItems: 'center',
}))

registerRole('row', (_node, _ctx) => ({
  layout: 'horizontal',
  width: 'fill_container' as const,
  gap: 16,
  alignItems: 'center',
}))

registerRole('column', (_node, _ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  gap: 16,
}))

registerRole('centered-content', (_node, ctx) => ({
  layout: 'vertical',
  width: ctx.canvasWidth <= 480 ? ('fill_container' as const) : 1080,
  gap: 24,
  alignItems: 'center',
}))

registerRole('form-group', (_node, _ctx) => ({
  layout: 'vertical',
  width: 'fill_container' as const,
  gap: 16,
}))

registerRole('spacer', (_node, _ctx) => ({
  width: 'fill_container' as const,
  height: 40,
}))

registerRole('divider', (node, _ctx) => {
  const isVertical = node.name?.toLowerCase().includes('vertical')
  if (isVertical) {
    return { width: 1, height: 'fill_container' as const, layout: 'none' as const }
  }
  return {
    width: 'fill_container' as const,
    height: 1,
    layout: 'none' as const,
  }
})

// ---------------------------------------------------------------------------
// Navigation roles
// ---------------------------------------------------------------------------

registerRole('navbar', (_node, ctx) => ({
  layout: 'horizontal',
  width: 'fill_container' as const,
  height: ctx.canvasWidth <= 480 ? 56 : 72,
  padding:
    ctx.canvasWidth <= 480
      ? ([0, 16] as [number, number])
      : ([0, 80] as [number, number]),
  alignItems: 'center',
  justifyContent: 'space_between' as const,
}))

registerRole('nav-links', (_node, _ctx) => ({
  layout: 'horizontal',
  gap: 24,
  alignItems: 'center',
}))

registerRole('nav-link', (_node, _ctx) => ({
  textGrowth: 'auto' as const,
  lineHeight: 1.2,
}))

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
    }
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
    }
  }
  return {
    padding: [12, 24] as [number, number],
    height: 44,
    layout: 'horizontal' as const,
    gap: 8,
    alignItems: 'center' as const,
    justifyContent: 'center' as const,
    cornerRadius: 8,
  }
})

registerRole('icon-button', (_node, _ctx) => ({
  width: 44,
  height: 44,
  layout: 'horizontal' as const,
  justifyContent: 'center' as const,
  alignItems: 'center' as const,
  cornerRadius: 8,
}))

registerRole('badge', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [6, 12] as [number, number],
  gap: 4,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 999,
}))

registerRole('tag', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [4, 10] as [number, number],
  gap: 4,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 6,
}))

registerRole('pill', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  padding: [6, 14] as [number, number],
  gap: 6,
  alignItems: 'center' as const,
  justifyContent: 'center' as const,
  cornerRadius: 999,
}))

registerRole('input', (_node, ctx) => {
  if (ctx.parentLayout === 'vertical') {
    return {
      width: 'fill_container' as const,
      height: 48,
      layout: 'horizontal' as const,
      padding: [12, 16] as [number, number],
      alignItems: 'center' as const,
      cornerRadius: 8,
    }
  }
  return {
    height: 48,
    layout: 'horizontal' as const,
    padding: [12, 16] as [number, number],
    alignItems: 'center' as const,
    cornerRadius: 8,
  }
})

registerRole('form-input', (_node, _ctx) => ({
  width: 'fill_container' as const,
  height: 48,
  layout: 'horizontal' as const,
  padding: [12, 16] as [number, number],
  alignItems: 'center' as const,
  cornerRadius: 8,
}))

registerRole('search-bar', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  height: 44,
  padding: [10, 16] as [number, number],
  gap: 8,
  alignItems: 'center' as const,
  cornerRadius: 22,
}))

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
    }
  }
  return {
    layout: 'vertical' as const,
    gap: 12,
    cornerRadius: 12,
    clipContent: true,
  }
})

registerRole('stat-card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 8,
      padding: [24, 24] as [number, number],
      cornerRadius: 12,
    }
  }
  return {
    layout: 'vertical' as const,
    gap: 8,
    padding: [24, 24] as [number, number],
    cornerRadius: 12,
  }
})

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
    }
  }
  return {
    layout: 'vertical' as const,
    gap: 16,
    padding: [32, 24] as [number, number],
    cornerRadius: 16,
    clipContent: true,
  }
})

registerRole('image-card', (_node, _ctx) => ({
  layout: 'vertical' as const,
  gap: 0,
  cornerRadius: 12,
  clipContent: true,
}))

// ---------------------------------------------------------------------------
// Content roles
// ---------------------------------------------------------------------------

registerRole('hero', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding:
    ctx.canvasWidth <= 480
      ? ([40, 16] as [number, number])
      : ([80, 80] as [number, number]),
  gap: 24,
  alignItems: 'center',
}))

registerRole('feature-grid', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  gap: 24,
  alignItems: 'start' as const,
}))

registerRole('feature-card', (_node, ctx) => {
  if (ctx.parentLayout === 'horizontal') {
    return {
      width: 'fill_container' as const,
      height: 'fill_container' as const,
      layout: 'vertical' as const,
      gap: 12,
      padding: [24, 24] as [number, number],
      cornerRadius: 12,
    }
  }
  return {
    layout: 'vertical' as const,
    gap: 12,
    padding: [24, 24] as [number, number],
    cornerRadius: 12,
  }
})

registerRole('testimonial', (_node, _ctx) => ({
  layout: 'vertical' as const,
  gap: 16,
  padding: [24, 24] as [number, number],
  cornerRadius: 12,
}))

registerRole('cta-section', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding:
    ctx.canvasWidth <= 480
      ? ([40, 16] as [number, number])
      : ([60, 80] as [number, number]),
  gap: 20,
  alignItems: 'center',
}))

registerRole('footer', (_node, ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding:
    ctx.canvasWidth <= 480
      ? ([32, 16] as [number, number])
      : ([48, 80] as [number, number]),
  gap: 24,
}))

registerRole('stats-section', (_node, ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  height: 'fit_content' as const,
  padding:
    ctx.canvasWidth <= 480
      ? ([32, 16] as [number, number])
      : ([48, 80] as [number, number]),
  gap: 32,
  justifyContent: 'center' as const,
  alignItems: 'center' as const,
}))

// ---------------------------------------------------------------------------
// Media roles
// ---------------------------------------------------------------------------

registerRole('phone-mockup', (_node, _ctx) => ({
  width: 280,
  height: 560,
  cornerRadius: 32,
  layout: 'none' as const,
}))

registerRole('screenshot-frame', (_node, _ctx) => ({
  cornerRadius: 12,
  clipContent: true,
}))

registerRole('avatar', (node, _ctx) => {
  const rawWidth = 'width' in node ? node.width : undefined
  const size =
    typeof rawWidth === 'number' && rawWidth > 0 ? rawWidth : 48
  return {
    width: size,
    height: size,
    cornerRadius: Math.round(size / 2),
    clipContent: true,
  }
})

registerRole('icon', (_node, _ctx) => ({
  width: 24,
  height: 24,
}))

// ---------------------------------------------------------------------------
// Typography roles
// ---------------------------------------------------------------------------

registerRole('heading', (node, ctx) => {
  const text = getTextContentForNode(node)
  const isCjk = hasCjkText(text)
  return {
    lineHeight: isCjk ? 1.35 : 1.2,
    letterSpacing: isCjk ? 0 : -0.5,
    textGrowth:
      ctx.parentLayout === 'vertical'
        ? ('fixed-width' as const)
        : ('auto' as const),
    width:
      ctx.parentLayout === 'vertical'
        ? ('fill_container' as const)
        : undefined,
  }
})

registerRole('subheading', (node, _ctx) => {
  const text = getTextContentForNode(node)
  const isCjk = hasCjkText(text)
  return {
    lineHeight: isCjk ? 1.4 : 1.3,
    textGrowth: 'fixed-width' as const,
    width: 'fill_container' as const,
  }
})

registerRole('body-text', (node, _ctx) => {
  const text = getTextContentForNode(node)
  const isCjk = hasCjkText(text)
  return {
    lineHeight: isCjk ? 1.6 : 1.5,
    textGrowth: 'fixed-width' as const,
    width: 'fill_container' as const,
  }
})

registerRole('caption', (node, _ctx) => {
  const text = getTextContentForNode(node)
  const isCjk = hasCjkText(text)
  return {
    lineHeight: isCjk ? 1.4 : 1.3,
    textGrowth: 'auto' as const,
  }
})

registerRole('label', (_node, _ctx) => ({
  lineHeight: 1.2,
  textGrowth: 'auto' as const,
  textAlignVertical: 'middle' as const,
}))

// ---------------------------------------------------------------------------
// Table roles
// ---------------------------------------------------------------------------

registerRole('table', (_node, _ctx) => ({
  layout: 'vertical' as const,
  width: 'fill_container' as const,
  gap: 0,
  clipContent: true,
}))

registerRole('table-row', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  alignItems: 'center' as const,
  padding: [12, 16] as [number, number],
}))

registerRole('table-header', (_node, _ctx) => ({
  layout: 'horizontal' as const,
  width: 'fill_container' as const,
  alignItems: 'center' as const,
  padding: [12, 16] as [number, number],
}))

registerRole('table-cell', (_node, _ctx) => ({
  width: 'fill_container' as const,
}))
