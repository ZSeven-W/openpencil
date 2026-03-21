// ---------------------------------------------------------------------------
// Shared insertion indicator + container highlight state
// Used by layout-reorder.ts and drag-into-layout.ts, rendered by
// use-layout-indicator.ts.
// ---------------------------------------------------------------------------

export interface InsertionIndicator {
  x: number
  y: number
  length: number
  orientation: 'vertical' | 'horizontal'
}

export interface ContainerHighlight {
  x: number
  y: number
  w: number
  h: number
}

export let activeInsertionIndicator: InsertionIndicator | null = null
export let activeContainerHighlight: ContainerHighlight | null = null

export function setInsertionIndicator(v: InsertionIndicator | null) {
  activeInsertionIndicator = v
}

export function setContainerHighlight(v: ContainerHighlight | null) {
  activeContainerHighlight = v
}
