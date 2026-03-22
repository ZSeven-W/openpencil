import { registerRole } from '../role-resolver'

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
