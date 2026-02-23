import { registerRole } from '../role-resolver'

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
