/**
 * Design principles — condensed design knowledge for AI sub-agents.
 *
 * Previously loaded 5 individual principle files (~7400 chars total).
 * Now uses a single condensed version (~1000 chars) since the SUB_AGENT_PROMPT
 * already contains essential design rules — principles only add high-value guidance.
 */

const CONDENSED_PRINCIPLES = `DESIGN CRAFT:
- Type scale with real contrast: display 48-64, heading 28-36, body 16. Weight: 700 titles, 500 subtitles, 400 body.
- Line height: tighter at large sizes (1.05-1.15 for 40px+), looser at small (1.5-1.6 for 16px).
- Palette: 1 primary action color, 1 accent, neutral scale. Max 2 saturated colors. Page bg slightly tinted (#F8FAFC not #FFFFFF).
- Contrast law: WCAG AA (4.5:1 body, 3:1 large text). Text: primary #0F172A for headings, muted #475569 for body.
- 8px grid spacing: related items 8-16px, groups 24-32px, sections 48-80px. Section padding 80-120px vertical.
- Hero: ONE headline, ONE subtitle, ONE CTA, optional visual. Every extra element dilutes focus.
- Cards: consistent cornerRadius/padding/shadow. Content: image → title → description → action.
- Nav: logo + 3-5 links + CTA. space_between distribution. Keep minimal.
- Alternate section backgrounds (white/#F8FAFC) for natural separation.`

/**
 * Get all design principles combined.
 */
export function getAllPrinciples(): string {
  return CONDENSED_PRINCIPLES
}

// Re-export individual principles for direct access
export { TYPOGRAPHY_PRINCIPLES } from './typography'
export { COLOR_PRINCIPLES } from './color'
export { SPACING_PRINCIPLES } from './spacing'
export { COMPOSITION_PRINCIPLES } from './composition'
export { COMPONENT_PRINCIPLES } from './components'
