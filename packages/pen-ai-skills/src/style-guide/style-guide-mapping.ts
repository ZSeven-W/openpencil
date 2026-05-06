import type { StyleGuideValues } from './style-guide-parser';

export interface PropertyReplacement {
  fillColor?: { from: string; to: string }[];
  textColor?: { from: string; to: string }[];
  strokeColor?: { from: string; to: string }[];
  fontFamily?: { from: string; to: string }[];
  cornerRadius?: { from: number | number[]; to: number | number[] }[];
}

/**
 * Build 从一个样式指
 * 南的值到另一个样式指南的值的属性替换映射。 Used 与 replace_all_matching_properties MCP 工具用于“风格切换”。
 *
 * Routes 颜色到正确的通道： - background/surface/accent → fillColor（框架背景） -
 * textPrimary/
 * textSecondary/textMuted →
 * textColor（文本填充） - 边框 → strokeColor（边框描边） Corner
 *
 * 半径以标量数字（不是数组）形式发出，以匹配 PenNode 存储。
 */
export function buildStyleMapping(
  from: StyleGuideValues,
  to: StyleGuideValues,
): PropertyReplacement {
  const fillColor: PropertyReplacement['fillColor'] = [];
  const textColor: PropertyReplacement['textColor'] = [];
  const strokeColor: PropertyReplacement['strokeColor'] = [];
  const fontFamily: PropertyReplacement['fontFamily'] = [];
  const cornerRadius: PropertyReplacement['cornerRadius'] = [];

  // --- Fill 颜色（背景、表面、强调色） ---
  const fillKeys = ['background', 'surface', 'accent'] as const;
  for (const key of fillKeys) {
    const f = from.colors[key];
    const t = to.colors[key];
    if (f && t && f !== t) {
      fillColor.push({ from: f, to: t });
    }
  }

  // --- Text 颜色 ---
  const textKeys = ['textPrimary', 'textSecondary', 'textMuted'] as const;
  for (const key of textKeys) {
    const f = from.colors[key];
    const t = to.colors[key];
    if (f && t && f !== t) {
      textColor.push({ from: f, to: t });
    }
  }

  // --- Border/stroke 颜色 ---
  if (from.colors.border && to.colors.border && from.colors.border !== to.colors.border) {
    strokeColor.push({ from: from.colors.border, to: to.colors.border });
  }

  // --- Font 家族替代品 ---
  const fontKeys = ['displayFont', 'bodyFont', 'dataFont'] as const;
  for (const key of fontKeys) {
    const f = from.typography[key];
    const t = to.typography[key];
    if (f && t && f !== t) {
      fontFamily.push({ from: f, to: t });
    }
  }

  // --- Corner 半径替换（标量数字，匹配 PenNode 存储） ---
  const radiusKeys = ['card', 'button'] as const;
  for (const key of radiusKeys) {
    const f = from.radius[key];
    const t = to.radius[key];
    if (f !== undefined && t !== undefined && f !== t) {
      cornerRadius.push({ from: f, to: t });
    }
  }

  // Build 结果，省略空数组
  const result: PropertyReplacement = {};
  if (fillColor.length > 0) result.fillColor = fillColor;
  if (textColor.length > 0) result.textColor = textColor;
  if (strokeColor.length > 0) result.strokeColor = strokeColor;
  if (fontFamily.length > 0) result.fontFamily = fontFamily;
  if (cornerRadius.length > 0) result.cornerRadius = cornerRadius;

  return result;
}
