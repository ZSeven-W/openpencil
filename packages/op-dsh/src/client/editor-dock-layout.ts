/** Self-contained DSH layout push used by the fallback OpenPencil workbench. */

export const OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE = 'openpencilWorkbenchDockOwner'

export interface EditorWorkbenchDockLease {
  update: (width: number) => void
  release: () => void
}

function dockWidth(width: number): string {
  return `${Math.max(0, Math.round(width))}px`
}

/**
 * Reserve real layout space for the fixed right-hand workbench.
 *
 * DSH's root is an auto-width block, so a right margin shrinks its AppFrame
 * grid instead of covering the conversation. Ownership and exact inline-style
 * restoration keep this compatible with HMR and fail closed around another
 * plugin that already owns the root margin.
 */
export function claimEditorWorkbenchDock(
  root: HTMLElement,
  owner: string,
  initialWidth: number,
  computedMarginRight = 0,
): EditorWorkbenchDockLease | undefined {
  const existingOwner = root.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE]
  if (existingOwner !== undefined && existingOwner !== owner) return undefined
  if (existingOwner === undefined && (
    root.style.marginRight.trim() !== ''
    || (Number.isFinite(computedMarginRight) && computedMarginRight > 0.5)
  )) return undefined

  const previousMarginRight = root.style.marginRight
  const previousMinWidth = root.style.minWidth
  root.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE] = owner
  root.style.minWidth = '0'

  let released = false
  const update = (width: number): void => {
    if (released || root.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE] !== owner) return
    root.style.marginRight = dockWidth(width)
  }
  const release = (): void => {
    if (released) return
    released = true
    if (root.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE] !== owner) return
    root.style.marginRight = previousMarginRight
    root.style.minWidth = previousMinWidth
    delete root.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE]
  }

  update(initialWidth)
  return { update, release }
}
