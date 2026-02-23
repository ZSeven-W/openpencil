import { registerRole } from '../role-resolver'

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
