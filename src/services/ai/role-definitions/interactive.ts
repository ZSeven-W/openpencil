import { registerRole } from '../role-resolver'

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
