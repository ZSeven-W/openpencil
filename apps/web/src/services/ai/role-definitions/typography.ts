import { registerRole } from '../role-resolver'
import { hasCjkText, getTextContentForNode } from '../generation-utils'

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
