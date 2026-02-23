import { registerRole } from '../role-resolver'

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
