/**
 * Spacing and layout design principles — selective loading based on request context.
 */

export const SPACING_PRINCIPLES = `SPACING & LAYOUT RHYTHM:
- Use an 8px grid: all spacing values should be multiples of 8 (8, 16, 24, 32, 48, 64, 80, 96). This creates visual consistency that humans perceive subconsciously.
- Section padding: hero/major sections need generous breathing room (80-120px vertical, 80px horizontal). Cramped sections feel cheap.
- Gap hierarchy mirrors content hierarchy: related items 8-16px apart, groups 24-32px, sections 48-80px. The gap between a title and its paragraph should be smaller than the gap between two cards.
- Padding inside containers should scale with the container size: small cards 16-20px, medium cards 24-32px, large sections 48-80px. Uniform 16px on everything looks template-ish.
- Consistent content width: keep the main content column at 1040-1160px across sections. Sections can have full-bleed backgrounds but content must align. Inconsistent content widths look amateurish.
- White space is a design element, not wasted space. A hero section with generous padding looks premium; a hero crammed with elements looks like a flyer.
- Cards in a row: equal width via fill_container, equal height via fill_container, consistent padding, consistent gap. Uneven cards break visual rhythm.
- Responsive readability: body text line length should be 50-75 characters. In a 1200px layout, this means text columns of ~600-700px, not full-width.`
