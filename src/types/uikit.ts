import type { PenDocument } from './pen'

export type ComponentCategory =
  | 'buttons'
  | 'inputs'
  | 'cards'
  | 'navigation'
  | 'layout'
  | 'feedback'
  | 'data-display'
  | 'other'

export interface KitComponent {
  /** Node ID of the reusable FrameNode in the kit document */
  id: string
  /** Display name */
  name: string
  /** Category for organization in the browser */
  category: ComponentCategory
  /** Tags for search */
  tags: string[]
  /** Component dimensions for preview sizing */
  width: number
  height: number
}

export interface UIKit {
  /** Unique identifier */
  id: string
  /** Display name */
  name: string
  /** Optional description */
  description?: string
  /** Version string */
  version: string
  /** Whether this is a built-in kit that ships with the app */
  builtIn: boolean
  /** Backing PenDocument containing the reusable nodes */
  document: PenDocument
  /** Extracted component metadata for browsing */
  components: KitComponent[]
}
