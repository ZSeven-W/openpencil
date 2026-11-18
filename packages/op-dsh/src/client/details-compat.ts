/** Compatibility boundary for future DSH builds that add a keyed Tool-details seam. */

import type { ToolCallViewProps } from '@deepseek-ai/dsh-client-ui-tool/client'

/**
 * The proposed `tool.details.toolview` slot was declared here so a future host
 * could activate a resident details column without breaking current ones. DSH
 * 0.1.5 settled the question the other way: it declares `tool.call.toolview`
 * and `tool.call.images` only, and removed `DetailsToolOwnerProps` — the type
 * the forward-compat declaration named as the slot owner. The declaration and
 * its registration are therefore gone.
 *
 * Nothing observable changes. `slots.inject()` waits while a slot is absent,
 * so on every host that ever shipped, the registration never fired and
 * `requestOpenPencilEditor` already fell through to the plugin's own modal.
 * The panel component stays, driven by the page-owned host below.
 */
export interface CompatibleToolDetailsViewProps {
  /** The tool-call block whose design this panel edits. */
  block: ToolCallViewProps['block']
  /** Session the block belongs to. */
  sessionId: string
}

/** Call props with a possible future additive sidebar capability. */
export type CompatibleToolCallViewProps = ToolCallViewProps & {
  openDetails?: (() => void) | undefined
}

export type OpenPencilEditorSurface = 'details' | 'modal'

/** Prefer the native resident details panel and otherwise open our own modal. */
export function requestOpenPencilEditor(
  openDetails: (() => void) | undefined,
  openModal: () => void,
): OpenPencilEditorSurface {
  if (openDetails !== undefined) {
    openDetails()
    return 'details'
  }
  openModal()
  return 'modal'
}
