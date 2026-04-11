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

  return parts.join('\n\n');
}
