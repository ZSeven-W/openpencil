/**
 * Color theory design principles — selective loading based on request context.
 */

export const COLOR_PRINCIPLES = `COLOR THEORY & PALETTE:
- A strong palette has 1 primary action color, 1 accent for secondary actions, and a neutral scale for text/backgrounds. Max 2 saturated colors — more creates visual noise.
- Create depth with the neutral scale: page bg slightly tinted (not pure #FFFFFF — use #F8FAFC, #FAFAF9, or #F5F3FF for subtle warmth/coolness), card surface pure white, text dark but not black (#0F172A reads softer than #000000).
- Contrast is law: text on backgrounds must pass WCAG AA (4.5:1 for body, 3:1 for large text). Test your accent color on both light and dark surfaces.
- Use color temperature to set mood: blue/slate = trust/tech, warm gray/amber = friendly/creative, emerald/teal = growth/health, violet/indigo = premium/creative.
- Gradient use: subtle gradients on hero backgrounds (30-degree angle, 2 related hues) add polish. Avoid rainbow gradients or harsh color jumps.
- Borders should be barely visible (#E2E8F0 on white, rgba(255,255,255,0.1) on dark) — they separate, not decorate.
- Dark themes: background #0F172A or #18181B (never pure black), surface #1E293B, text #F8FAFC. Accent colors should be slightly lighter/more saturated than in light themes.
- Avoid the "AI palette" trap: not every design needs blue primary + white cards. Match colors to the product domain — fintech can use deep navy, health apps can use sage green, creative tools can use coral/amber.`
