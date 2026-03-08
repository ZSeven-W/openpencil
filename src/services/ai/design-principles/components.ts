/**
 * Component design patterns — selective loading based on request context.
 */

export const COMPONENT_PRINCIPLES = `COMPONENT DESIGN PATTERNS:
- Buttons have weight hierarchy: primary (filled, accent color) for THE action, secondary (outlined or ghost) for alternatives, tertiary (text-only) for less important. Never two primary buttons side by side.
- Cards: consistent corner radius (12-16px), consistent padding (24-32px), consistent shadow depth. Content inside follows vertical rhythm: image → title → description → action. Separate concerns into body and footer.
- Navigation: keep it minimal — logo, 3-5 links, one CTA. The nav bar is wayfinding, not a menu explosion. Use space_between for distribution.
- Forms: all inputs same width (fill_container), consistent height (44-48px), clear labels above inputs, submit button at bottom with same width. Group related fields. Don't forget affordance icons (search, email, password).
- Hero sections: one clear headline, one supporting sentence, one or two CTAs, optional visual (image/mockup). Resist adding more — every extra element dilutes the focus.
- Feature sections: icon + title + description per card. Icons should be uniform size and style. Descriptions should be 1 sentence. Three or four cards max per row.
- Pricing cards: highlight the recommended plan with accent background or border. Keep plan names short. Use checkmark lists for features, not paragraphs.
- Testimonials: real-feeling names, short quotes (1-2 sentences), optional avatar. Three cards or a slider, not a wall of text.
- Footer: organized in 3-4 column groups (product, company, resources, social). Muted colors, smaller text. Don't cram everything from the nav here.`
