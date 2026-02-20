/**
 * Recursively replace `$variable` references in a PenNode tree.
 *
 * Used when renaming or deleting a variable to keep the tree consistent.
 */

import type { PenNode } from '@/types/pen'
import type { PenFill } from '@/types/styles'
import type { VariableDefinition } from '@/types/variables'
import { resolveVariableRef } from './resolve-variables'

/**
 * Replace all occurrences of `$oldRef` with `$newRef` in the node tree.
 * When `newRef` is null (variable deleted), resolves to the concrete value.
 */
export function replaceVariableRefsInTree(
  nodes: PenNode[],
  oldRef: string,
  newRef: string | null,
  variables: Record<string, VariableDefinition>,
  activeTheme: Record<string, string>,
): PenNode[] {
  const oldToken = `$${oldRef}`
  const replacement = newRef ? `$${newRef}` : undefined

  function resolveOrReplace(val: string): string {
    if (val !== oldToken) return val
    if (replacement) return replacement
    const resolved = resolveVariableRef(oldToken, variables, activeTheme)
    return typeof resolved === 'string' ? resolved : val
  }

  function resolveOrReplaceNumeric(val: string | number): string | number {
    if (typeof val !== 'string' || val !== oldToken) return val
    if (replacement) return replacement
    const resolved = resolveVariableRef(oldToken, variables, activeTheme)
    return typeof resolved === 'number' ? resolved : val
  }

  function replaceFills(fills: PenFill[] | undefined): PenFill[] | undefined {
    if (!fills) return fills
    return fills.map((f) => {
      if (f.type === 'solid' && f.color === oldToken) {
        return { ...f, color: resolveOrReplace(f.color) }
      }
      if (f.type === 'linear_gradient' || f.type === 'radial_gradient') {
        const newStops = f.stops.map((s) =>
          s.color === oldToken ? { ...s, color: resolveOrReplace(s.color) } : s,
        )
        return { ...f, stops: newStops }
      }
      return f
    })
  }

  function replaceInNode(node: PenNode): PenNode {
    const out: Record<string, unknown> = { ...node }
    let changed = false

    // Opacity
    if (typeof node.opacity === 'string' && node.opacity === oldToken) {
      out.opacity = resolveOrReplaceNumeric(node.opacity)
      changed = true
    }

    // Gap
    if ('gap' in node && (node as unknown as Record<string, unknown>).gap === oldToken) {
      out.gap = resolveOrReplaceNumeric(oldToken)
      changed = true
    }

    // Padding
    if ('padding' in node && (node as unknown as Record<string, unknown>).padding === oldToken) {
      out.padding = resolveOrReplaceNumeric(oldToken)
      changed = true
    }

    // Fill
    if ('fill' in node && (node as unknown as Record<string, unknown>).fill) {
      const fills = (node as unknown as Record<string, unknown>).fill as PenFill[]
      const newFills = replaceFills(fills)
      if (newFills !== fills) {
        out.fill = newFills
        changed = true
      }
    }

    // Stroke fill & thickness
    if ('stroke' in node && (node as unknown as Record<string, unknown>).stroke) {
      const stroke = (node as unknown as Record<string, unknown>).stroke as Record<string, unknown>
      const newStroke = { ...stroke }
      let strokeChanged = false
      if (typeof stroke.thickness === 'string' && stroke.thickness === oldToken) {
        newStroke.thickness = resolveOrReplaceNumeric(oldToken)
        strokeChanged = true
      }
      if (stroke.fill) {
        const newFill = replaceFills(stroke.fill as PenFill[])
        if (newFill !== stroke.fill) {
          newStroke.fill = newFill
          strokeChanged = true
        }
      }
      if (strokeChanged) {
        out.stroke = newStroke
        changed = true
      }
    }

    // Effects
    if ('effects' in node && Array.isArray((node as unknown as Record<string, unknown>).effects)) {
      const effects = (node as unknown as Record<string, unknown>).effects as Record<string, unknown>[]
      const newEffects = effects.map((e) => {
        const ne = { ...e }
        let ec = false
        if (typeof e.color === 'string' && e.color === oldToken) {
          ne.color = resolveOrReplace(e.color as string)
          ec = true
        }
        for (const key of ['blur', 'offsetX', 'offsetY', 'spread']) {
          if (typeof e[key] === 'string' && e[key] === oldToken) {
            ne[key] = resolveOrReplaceNumeric(oldToken)
            ec = true
          }
        }
        return ec ? ne : e
      })
      out.effects = newEffects
      changed = true
    }

    // Text content
    if (node.type === 'text' && typeof node.content === 'string' && node.content === oldToken) {
      out.content = resolveOrReplace(node.content)
      changed = true
    }

    // Recurse children
    if ('children' in node && (node as unknown as Record<string, unknown>).children) {
      const children = (node as unknown as Record<string, unknown>).children as PenNode[]
      out.children = replaceVariableRefsInTree(children, oldRef, newRef, variables, activeTheme)
      changed = true
    }

    return changed ? (out as unknown as PenNode) : node
  }

  return nodes.map(replaceInNode)
}
