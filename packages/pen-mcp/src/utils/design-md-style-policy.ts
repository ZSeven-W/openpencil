import type { DesignMdSpec } from '@zseven-w/pen-types';

/** Build a condensed design.md style policy string for AI prompt injection. */
export function buildDesignMdStylePolicy(spec: DesignMdSpec): string {
  const parts: string[] = [];

  if (spec.visualTheme) {
    const theme =
      spec.visualTheme.length > 200 ? spec.visualTheme.substring(0, 200) + '...' : spec.visualTheme;
    parts.push(`VISUAL THEME: ${theme}`);
  }

  if (spec.colorPalette?.length) {
    const colors = spec.colorPalette
      .slice(0, 10)
      .map((c) => `${c.name} (${c.hex}) — ${c.role}`)
      .join('\n- ');
    parts.push(`COLOR PALETTE:\n- ${colors}`);

    const surfaces = spec.colorPalette.filter((c) =>
      /surface|card|panel|sidebar|tile|chip/i.test(c.role || ''),
    );
    if (surfaces.length > 0) {
      const surfaceLines = surfaces
        .slice(0, 6)
        .map((c) => `${c.name} (${c.hex}) — ${c.role}`)
        .join('\n- ');
      parts.push(
        `SURFACE COLORS (use ONLY as \`fill\` on visually distinct components placed on top of the page background — cards, sidebars, floating panels, chips, badges. DO NOT fill section root frames or generic wrapper frames with these; section containers must stay transparent and inherit the page background. NEVER use these as the page/rootFrame fill):\n- ${surfaceLines}`,
      );
    }
  }

  if (spec.typography?.fontFamily) {
    parts.push(`FONT: ${spec.typography.fontFamily}`);
  }
  if (spec.typography?.headings) {
    parts.push(`Headings: ${spec.typography.headings}`);
  }
  if (spec.typography?.body) {
    parts.push(`Body: ${spec.typography.body}`);
  }

  if (spec.componentStyles) {
    const styles =
      spec.componentStyles.length > 300
        ? spec.componentStyles.substring(0, 300) + '...'
        : spec.componentStyles;
    parts.push(`COMPONENT STYLES:\n${styles}`);
  }

  if (spec.layoutPrinciples) {
    const layout =
      spec.layoutPrinciples.length > 400
        ? spec.layoutPrinciples.substring(0, 400) + '...'
        : spec.layoutPrinciples;
    parts.push(`LAYOUT PRINCIPLES:\n${layout}`);
  }

  if (spec.generationNotes) {
    const notes =
      spec.generationNotes.length > 400
        ? spec.generationNotes.substring(0, 400) + '...'
        : spec.generationNotes;
    parts.push(`GENERATION NOTES:\n${notes}`);
  }

  return parts.join('\n\n');
}
