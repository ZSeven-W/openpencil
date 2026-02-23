import { registerRole } from '../role-resolver'

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
