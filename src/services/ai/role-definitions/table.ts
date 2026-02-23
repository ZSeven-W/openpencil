import { registerRole } from '../role-resolver'

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
