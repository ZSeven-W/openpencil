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
/* Section 分离器 */
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
/* Hex 提取助手 */
/* ------------------------------------------------------------------ */

const HEX_RE = /#[0-9A-Fa-f]{6}\b/;

/** Return 与 `label` 匹配的行上的第一个十六进制颜色（不区分大小写）。 */
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
/* Color 提取 */
/* ------------------------------------------------------------------ */

function extractColors(sections: Map<string, string>): StyleGuideValues['colors'] {
  const colorSection = sections.get('color system') ?? '';

  const background = hexNear(colorSection, /page\s+background|root.*background/i);
  const surface = hexNear(colorSection, /card\s+surface|surface/i);
  const border = hexNear(colorSection, /default\s+border|border|divider/i);

  // Accent：首先查看“Accent Colors”小节，然后回到任何“Primary Accent”行
  const accentSubIdx = colorSection.toLowerCase().indexOf('accent color');
  const accentBlock = accentSubIdx !== -1 ? colorSection.slice(accentSubIdx) : colorSection;
  const accent = hexNear(accentBlock, /primary\s+accent|accent/i);

  // Text 颜色小节
  const textSubIdx = colorSection.toLowerCase().indexOf('text color');
  const textBlock = textSubIdx !== -1 ? colorSection.slice(textSubIdx) : colorSection;

  const textPrimary = hexNear(textBlock, /primary\s+text/i);
  const textSecondary = hexNear(textBlock, /secondary\s+text/i);
  const textMuted = hexNear(textBlock, /muted\s+text|tertiary\s+text/i);

  return { background, surface, accent, textPrimary, textSecondary, textMuted, border };
}

/* ------------------------------------------------------------------ */
/* Typography 提取 */
/* ------------------------------------------------------------------ */

function extractTypography(sections: Map<string, string>): StyleGuideValues['typography'] {
  const typoSection = sections.get('typography') ?? '';

  // Find “Font Families”小节
  const famIdx = typoSection.toLowerCase().indexOf('font families');
  if (famIdx === -1) return {};

  const famBlock = typoSection.slice(famIdx);
  // Stop 位于下一个 ### 标题或 ## 标题
  const endIdx = famBlock.indexOf('\n###', 10);
  const familyText = endIdx !== -1 ? famBlock.slice(0, endIdx) : famBlock;

  // Parse 表行： | Role | Family | Usage | We 预计最多
  // 3 行：display、body、data/mono
  const rows: { role: string; family: string }[] = [];
  for (const line of familyText.split('\n')) {
    const cells = line
      .split('|')
      .map((c) => c.trim())
      .filter(Boolean);
    if (cells.length < 2) continue;
    // Skip 标题行和分隔行
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

  // If 我们没有按角色关键字匹配，退回到位置顺序
  if (!displayFont && rows.length >= 1) displayFont = rows[0].family;
  if (!bodyFont && rows.length >= 2) bodyFont = rows[1].family;
  if (!dataFont && rows.length >= 3) dataFont = rows[2].family;

  return { displayFont, bodyFont, dataFont };
}

/* ------------------------------------------------------------------ */
/* Corner 半径提取 */
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

  // If "Everything" 为 0px（野兽派风格），两者均为 0
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
/* Public API */
/* ------------------------------------------------------------------ */

/**
 * Extract
 * 来自样式指南的 Markdown 内容的结构化值。 Parses Color System、Typography 和
 Corner Radius 部分。
 */
export function extractStyleGuideValues(content: string): StyleGuideValues {
  const sections = getSections(content);
  return {
    colors: extractColors(sections),
    typography: extractTypography(sections),
    radius: extractRadius(sections),
  };
}
