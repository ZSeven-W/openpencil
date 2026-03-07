/**
 * Design principles — domain-specific design knowledge extracted from
 * professional design practices.
 *
 * All principles are always included since the total content is small (~60 lines).
 * Selective loading via keyword matching was removed to avoid hardcoded keyword lists.
 */

import { TYPOGRAPHY_PRINCIPLES } from './typography'
import { COLOR_PRINCIPLES } from './color'
import { SPACING_PRINCIPLES } from './spacing'
import { COMPOSITION_PRINCIPLES } from './composition'
import { COMPONENT_PRINCIPLES } from './components'

const ALL_PRINCIPLES = [
  COMPOSITION_PRINCIPLES,
  TYPOGRAPHY_PRINCIPLES,
  COLOR_PRINCIPLES,
  SPACING_PRINCIPLES,
  COMPONENT_PRINCIPLES,
].join('\n\n')

/**
 * Get all design principles combined.
 */
export function getAllPrinciples(): string {
  return ALL_PRINCIPLES
}

// Re-export individual principles for direct access
export { TYPOGRAPHY_PRINCIPLES } from './typography'
export { COLOR_PRINCIPLES } from './color'
export { SPACING_PRINCIPLES } from './spacing'
export { COMPOSITION_PRINCIPLES } from './composition'
export { COMPONENT_PRINCIPLES } from './components'
