export interface StyleGuideValues {
  colors: {
    background?: string;
    surface?: string;
    accent?: string;
    textPrimary?: string;
    textSecondary?: string;
    textMuted?: string;
    border?: string;
  };
  typography: {
    displayFont?: string;
    bodyFont?: string;
    dataFont?: string;
  };
  radius: {
    card?: number;
    button?: number;
  };
}

/* ------------------------------------------------------------------ */
/*  Section splitter                                                   */
/* ------------------------------------------------------------------ */

function getSections(content: string): Map<string, string> {
  const sections = new Map<string, string>();
  const parts = content.split(/^## /m);
  for (const part of parts) {
    const newline = part.indexOf('\n');
    if (newline === -1) continue;
    const heading = part.slice(0, newline).trim().toLowerCase();
    const body = part.slice(newline + 1);
    sections.set(heading, body);
  }
  return sections;
}

/* ------------------------------------------------------------------ */
/*  Hex extraction helpers                                             */
/* ------------------------------------------------------------------ */

const HEX_RE = /#[0-9A-Fa-f]{6}\b/;

/** Return the first hex color on a line matching `label` (case-insensitive). */
function hexNear(text: string, label: RegExp): string | undefined {
  for (const line of text.split('\n')) {
    if (label.test(line)) {
      const m = line.match(HEX_RE);
      if (m) return m[0].toUpperCase();
    }
  }
  return undefined;
}

/* ------------------------------------------------------------------ */
/*  Color extraction                                                   */
/* ------------------------------------------------------------------ */

function extractColors(sections: Map<string, string>): StyleGuideValues['colors'] {
  const colorSection = sections.get('color system') ?? '';

  const background = hexNear(colorSection, /page\s+background|root.*background/i);
  const surface = hexNear(colorSection, /card\s+surface|surface/i);
  const border = hexNear(colorSection, /default\s+border|border|divider/i);

  // Accent: look in the "Accent Colors" subsection first, fall back to any "Primary Accent" line
  const accentSubIdx = colorSection.toLowerCase().indexOf('accent color');
  const accentBlock = accentSubIdx !== -1 ? colorSection.slice(accentSubIdx) : colorSection;
  const accent = hexNear(accentBlock, /primary\s+accent|accent/i);

  // Text colors subsection
  const textSubIdx = colorSection.toLowerCase().indexOf('text color');
  const textBlock = textSubIdx !== -1 ? colorSection.slice(textSubIdx) : colorSection;

  const textPrimary = hexNear(textBlock, /primary\s+text/i);
  const textSecondary = hexNear(textBlock, /secondary\s+text/i);
  const textMuted = hexNear(textBlock, /muted\s+text|tertiary\s+text/i);

  return { background, surface, accent, textPrimary, textSecondary, textMuted, border };
}

/* ------------------------------------------------------------------ */
/*  Typography extraction                                              */
/* ------------------------------------------------------------------ */

function extractTypography(sections: Map<string, string>): StyleGuideValues['typography'] {
  const typoSection = sections.get('typography') ?? '';

  // Find the "Font Families" subsection
  const famIdx = typoSection.toLowerCase().indexOf('font families');
  if (famIdx === -1) return {};

  const famBlock = typoSection.slice(famIdx);
  // Stop at the next ### heading or ## heading
  const endIdx = famBlock.indexOf('\n###', 10);
  const familyText = endIdx !== -1 ? famBlock.slice(0, endIdx) : famBlock;

  // Parse table rows: | Role | Family | Usage |
  // We expect up to 3 rows: display, body, data/mono
  const rows: { role: string; family: string }[] = [];
  for (const line of familyText.split('\n')) {
    const cells = line
      .split('|')
      .map((c) => c.trim())
      .filter(Boolean);
    if (cells.length < 2) continue;
    // Skip header and divider rows
    if (cells[0].toLowerCase() === 'role' || cells[0].startsWith('-')) continue;
    rows.push({ role: cells[0].toLowerCase(), family: cells[1] });
  }

  let displayFont: string | undefined;
  let bodyFont: string | undefined;
  let dataFont: string | undefined;

  for (const row of rows) {
    if (!displayFont && /display|logo|heading|serif/i.test(row.role)) {
      displayFont = row.family;
    } else if (!bodyFont && /body|functional|ui/i.test(row.role)) {
      bodyFont = row.family;
    } else if (!dataFont && /data|mono/i.test(row.role)) {
      dataFont = row.family;
    }
  }

  // If we didn't match by role keywords, fall back to positional order
  if (!displayFont && rows.length >= 1) displayFont = rows[0].family;
  if (!bodyFont && rows.length >= 2) bodyFont = rows[1].family;
  if (!dataFont && rows.length >= 3) dataFont = rows[2].family;

  return { displayFont, bodyFont, dataFont };
}

/* ------------------------------------------------------------------ */
/*  Corner radius extraction                                           */
/* ------------------------------------------------------------------ */

function extractRadius(sections: Map<string, string>): StyleGuideValues['radius'] {
  const radiusSection = sections.get('corner radius') ?? '';
  if (!radiusSection) return {};

  let card: number | undefined;
  let button: number | undefined;

  for (const line of radiusSection.split('\n')) {
    const numMatch = line.match(/(\d+)px/);
    if (!numMatch) continue;
    const value = parseInt(numMatch[1], 10);

    const lower = line.toLowerCase();
    if (!card && /card|standard|container|primary.*radius/i.test(lower)) {
      card = value;
    }
    if (!button && /button|input|small/i.test(lower)) {
      button = value;
    }
  }

  // If "Everything" is 0px (brutalist style), both are 0
  if (card === undefined && button === undefined) {
    const everythingLine = radiusSection
      .split('\n')
      .find((l) => /everything/i.test(l) && /\d+px/.test(l));
    if (everythingLine) {
      const m = everythingLine.match(/(\d+)px/);
      if (m) {
        const v = parseInt(m[1], 10);
        card = v;
        button = v;
      }
    }
  }

  return { card, button };
}

/* ------------------------------------------------------------------ */
/*  Public API                                                         */
/* ------------------------------------------------------------------ */

/**
 * Extract structured values from a style guide's markdown content.
 * Parses Color System, Typography, and Corner Radius sections.
 */
export function extractStyleGuideValues(content: string): StyleGuideValues {
  const sections = getSections(content);
  return {
    colors: extractColors(sections),
    typography: extractTypography(sections),
    radius: extractRadius(sections),
  };
}
