import type { PenNode } from '@zseven-w/pen-types'
import { isVariableRef } from '@zseven-w/pen-core'
import { variableNameToCSS } from './css-variables-generator.js'

// Re-export for convenience
export { variableNameToCSS } from './css-variables-generator.js'
export { isVariableRef } from '@zseven-w/pen-core'

export function varOrLiteral(value: string): string {
  if (isVariableRef(value)) {
    return `var(${variableNameToCSS(value.slice(1))})`
  }
  return value
}

export function sanitizeName(name: string): string {
  return name
    .replace(/[^a-zA-Z0-9\s-_]/g, '')
    .split(/[\s\-_]+/)
    .filter(Boolean)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join('')
}

export function nodeTreeToSummary(nodes: PenNode[], depth: number = 0): string {
  if (nodes.length === 0) return ''

  const indent = '  '.repeat(depth)
  return nodes.map(node => {
    const n = node as unknown as Record<string, unknown>
    const dims = `${n.width ?? '?'}x${n.height ?? '?'}`
    const childCount = (n.children as PenNode[] | undefined)?.length ?? 0
    const role = n.role ? ` [${n.role}]` : ''
    const line = `${indent}- [${node.id}] ${node.type} "${node.name ?? ''}" (${dims})${role}${childCount > 0 ? ` [${childCount} children]` : ''}`
    const children = n.children as PenNode[] | undefined
    if (children && children.length > 0) {
      return line + '\n' + nodeTreeToSummary(children, depth + 1)
    }
    return line
  }).join('\n')
}
