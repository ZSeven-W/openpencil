/**
 * Typography design principles — selective loading based on request context.
 */

export const TYPOGRAPHY_PRINCIPLES = `TYPOGRAPHY CRAFT:
- Build a clear type scale with real contrast: display 48-64px, heading 28-36px, body 16px. Jumps must be noticeable — 16 to 18 is not a "heading".
- Pair fonts with purpose: a geometric sans (Space Grotesk, Outfit) for headlines creates personality; a neutral sans (Inter, DM Sans) for body ensures readability. Never use the same font weight everywhere.
- Weight creates hierarchy: 700 for titles, 500 for subtitles, 400 for body. Using 600 for everything makes nothing stand out.
- Line height is tighter at large sizes (1.05-1.15 for 40px+) and looser at small sizes (1.5-1.6 for 16px). This is how print typography works.
- Letter spacing: pull headlines tighter (-0.5 to -1px) for density, push uppercase labels wider (+1-2px) for legibility. Body stays at 0.
- Use font weight shifts for emphasis within paragraphs (500 for inline emphasis), not font-size changes. Size hierarchy is structural, weight hierarchy is inline.
- Headlines are short and punchy (2-6 words). If your headline needs two lines, the font size is too large or the text is too verbose.
- Color reinforces hierarchy: primary text (#0F172A-level) for headings, muted (#475569-level) for body, lighter still for captions. Never same color at same opacity for all text.`
