import type { PenNode } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// Sizing 解析器（由布局引擎和文本高度估计共享）
// ---------------------------------------------------------------------------

/** Parse 大小值。 Handles 数字、“fit_content”、“fill_container”和括号形式。 */
export function parseSizing(value: unknown): number | 'fit' | 'fill' {
  if (typeof value === 'number') return value;
  if (typeof value !== 'string') return 0;
  if (value.startsWith('fill_container')) return 'fill';
  if (value.startsWith('fit_content')) {
    const match = value.match(/\((\d+(?:\.\d+)?)\)/);
    if (match) return parseFloat(match[1]);
    return 'fit';
  }
  const n = parseFloat(value);
  return isNaN(n) ? 0 : n;
}

// ---------------------------------------------------------------------------
// Default 行高 — 所有模块的单一事实来源
// ---------------------------------------------------------------------------

/**
 * 当文本节点没有显式值时，
 * Canonical 默认为 lineHeight。 Display/heading 文本 (>=28px) 间距更紧；正文变得更加宽松。
 * All 模块（工厂、布局引擎、文本估计、AI 生成） MUST 使用此函数而不是硬编码的后备。
 *
 */
export function defaultLineHeight(fontSize: number): number {
  if (fontSize >= 40) return 1.0; // Display 文本：紧前导（匹配 Pencil 0.9-1.0）
  if (fontSize >= 28) return 1.15; // Heading 文本：中等（匹配 Pencil 1.0-1.2）
  if (fontSize >= 20) return 1.2; // Subheading
  return 1.5; // Body 文字：阅读舒适
}

// ---------------------------------------------------------------------------
// CJK 检测
// ---------------------------------------------------------------------------

export function isCjkCodePoint(code: number): boolean {
  return (
    (code >= 0x4e00 && code <= 0x9fff) || // CJK Unified Ideographs
    (code >= 0x3400 && code <= 0x4dbf) || // CJK Extension a
    (code >= 0x3040 && code <= 0x30ff) || // Hiragana + Katakana
    (code >= 0xac00 && code <= 0xd7af) || // Hangul
    (code >= 0x3000 && code <= 0x303f) || // CJK symbols/punctuation
    (code >= 0xff00 && code <= 0xffef)
  ); // Full-宽度形式
}

export function hasCjkText(text: string): boolean {
  for (const ch of text) {
    const code = ch.codePointAt(0) ?? 0;
    if (isCjkCodePoint(code)) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Glyph / 线宽估计
// ---------------------------------------------------------------------------

/**
 * Font 权重乘数 —
 * bold/semibold 文本比常规文本更宽。 Values 基于典型的比例字体宽度缩放。
 */
function fontWeightFactor(fontWeight?: string | number): number {
  const w = typeof fontWeight === 'string' ? parseInt(fontWeight, 10) : (fontWeight ?? 400);
  if (isNaN(w) || w <= 400) return 1.0;
  if (w <= 500) return 1.03;
  if (w <= 600) return 1.06;
  if (w <= 700) return 1.09;
  return 1.12;
}

export function estimateGlyphWidth(
  ch: string,
  fontSize: number,
  fontWeight?: string | number,
): number {
  if (ch === '\n' || ch === '\r') return 0;
  if (ch === '\t') return fontSize * 1.2;
  if (ch === ' ') return fontSize * 0.33;

  const wf = fontWeightFactor(fontWeight);
  const code = ch.codePointAt(0) ?? 0;
  if (isCjkCodePoint(code)) return fontSize * 1.12 * wf;
  if (/[A-Z]/.test(ch)) return fontSize * 0.62 * wf;
  if (/[a-z]/.test(ch)) return fontSize * 0.56 * wf;
  if (/[0-9]/.test(ch)) return fontSize * 0.56 * wf;
  return fontSize * 0.58 * wf;
}

export function estimateLineWidth(
  text: string,
  fontSize: number,
  letterSpacing = 0,
  fontWeight?: string | number,
): number {
  let width = 0;
  let visibleChars = 0;
  for (const ch of text) {
    width += estimateGlyphWidth(ch, fontSize, fontWeight);
    if (ch !== '\n' && ch !== '\r') visibleChars += 1;
  }
  if (visibleChars > 1 && letterSpacing !== 0) {
    width += (visibleChars - 1) * letterSpacing;
  }
  return Math.max(0, width);
}

export function widthSafetyFactor(text: string): number {
  // Latin 字体因 weight/family 变化很大；使用较大的安全裕度以避免低估宽度并导致意外缠绕。
  return hasCjkText(text) ? 1.06 : 1.14;
}

export function estimateTextWidth(
  text: string,
  fontSize: number,
  letterSpacing = 0,
  fontWeight?: string | number,
): number {
  const lines = text.split(/\r?\n/);
  const maxLine = lines.reduce((max, line) => {
    const lineWidth = estimateLineWidth(line, fontSize, letterSpacing, fontWeight);
    const safeLineWidth = lineWidth * widthSafetyFactor(line);
    return Math.max(max, safeLineWidth);
  }, 0);
  return maxLine;
}

/**
 * Estimate
 * 文字宽度 WITHOUT 安全系数。 Used 用于布局居中，其中安全边距导致文本出
 * 现偏离中心（居中时高估的宽度使文本框向左移动）。 For wrapping/sizing 决策时，使用 estimateTextWidth()
 * 其中包括安全系数。
 */
export function estimateTextWidthPrecise(
  text: string,
  fontSize: number,
  letterSpacing = 0,
  fontWeight?: string | number,
): number {
  const lines = text.split(/\r?\n/);
  return lines.reduce((max, line) => {
    return Math.max(max, estimateLineWidth(line, fontSize, letterSpacing, fontWeight));
  }, 0);
}

// ---------------------------------------------------------------------------
// Text 内容助手
// ---------------------------------------------------------------------------

export function resolveTextContent(node: PenNode): string {
  if (node.type !== 'text') return '';
  if (typeof node.content === 'string') return node.content;
  if (Array.isArray(node.content)) return node.content.map((s) => s.text).join('');
  // Fallback：MCP/CLI 节点可以使用 `text` 而不是 `content`
  if (typeof (node as unknown as Record<string, unknown>).text === 'string') {
    return (node as unknown as Record<string, unknown>).text as string;
  }
  return '';
}

export function countExplicitTextLines(text: string): number {
  if (!text) return 1;
  return Math.max(1, text.split(/\r?\n/).length);
}

// ---------------------------------------------------------------------------
// Optical 居中单行文本的垂直校正
// ---------------------------------------------------------------------------

/**
 * Optical
 * 居中单行文本的垂直校正。 Within Fabric 文本边界框 (fontSize * 1.13)，由于
 * ascent/descent 不对称，字形墨迹位于数学中心稍上方。 We 按比例向下微调以进行补偿。
 *
 */
export function getTextOpticalCenterYOffset(node: PenNode): number {
  if (node.type !== 'text') return 0;
  const text = resolveTextContent(node).trim();
  if (!text) return 0;
  if (countExplicitTextLines(text) > 1) return 0;

  const fontSize = node.fontSize ?? 16;
  const hasCjk = hasCjkText(text);

  // CJK 字形在 em 框中的位置比 Latin 字形更高
  const ratio = hasCjk ? 0.06 : 0.03;
  const offset = fontSize * ratio;
  return Math.max(0, Math.min(Math.round(fontSize * 0.05), Math.round(offset)));
}

// ---------------------------------------------------------------------------
// Wrapped 行计数 — 可注入 browser/non-browser 环境
// ---------------------------------------------------------------------------

/**
 * Count 使用字符宽度
 * 估计回退换行。 This 是纯（非浏览器）实现。
 */
export function countWrappedLinesFallback(
  rawLines: string[],
  wrapWidth: number,
  fontSize: number,
  letterSpacing: number,
  fontWeight: string | number | undefined,
): number {
  return rawLines.reduce((sum, line) => {
    const lineWidth =
      estimateLineWidth(line, fontSize, letterSpacing, fontWeight) * widthSafetyFactor(line);
    return sum + Math.max(1, Math.ceil(lineWidth / wrapWidth));
  }, 0);
}

/**
 * Injectable
 * 换行计数器。 Browser 环境可以将其替换为基于 Canvas 2d 的实现，以实现准确的自动换行预测。
 */
export type WrappedLineCounter = (
  rawLines: string[],
  wrapWidth: number,
  fontSize: number,
  fontWeight: string | number | undefined,
  fontFamily: string,
  letterSpacing: number,
) => number;

let _wrappedLineCounter: WrappedLineCounter | null = null;

/** Set 自定义换行计数器（例如 Canvas 基于 2d）。 */
export function setWrappedLineCounter(counter: WrappedLineCounter): void {
  _wrappedLineCounter = counter;
}

// ---------------------------------------------------------------------------
// Text 高度估计（多行换行感知）
// ---------------------------------------------------------------------------

/** Estimate 文本高度，包括已知可用宽度时的多行换行。 */
export function estimateTextHeight(node: PenNode, availableWidth?: number): number {
  // Access 通过 Record 的文本特定属性以避免联合类型问题
  const n = node as unknown as Record<string, unknown>;
  const fontSize = typeof n.fontSize === 'number' ? n.fontSize : 16;
  const lineHeight = typeof n.lineHeight === 'number' ? n.lineHeight : defaultLineHeight(fontSize);
  // Fabric.js 使用 _fontsizemult = 1.13 作为单行的字形高度。 lineHeight
  // 间距适用于*行之间，而不是最后一行下方。
  const FABRIC_FONT_MULT = 1.13;
  const glyphH = fontSize * FABRIC_FONT_MULT;
  const lineStep = fontSize * lineHeight;

  // Get 文字内容
  const rawContent = n.content;
  const content =
    typeof rawContent === 'string'
      ? rawContent
      : Array.isArray(rawContent)
        ? rawContent.map((s: { text: string }) => s.text).join('')
        : '';
  if (!content) return glyphH;

  // Determine 用于换行估计的有效文本宽度
  let textWidth = 0;
  if ('width' in node) {
    const w = parseSizing(node.width);
    if (typeof w === 'number' && w > 0) textWidth = w;
    else if (w === 'fill' && availableWidth && availableWidth > 0) textWidth = availableWidth;
  }

  // If 没有已知的宽度约束，仍然计算显式换行符
  if (textWidth <= 0) {
    const explicitLines = content.split(/\r?\n/).length;
    const n2 = Math.max(1, explicitLines);
    return Math.round(n2 <= 1 ? glyphH : (n2 - 1) * lineStep + glyphH);
  }

  // Use 自定义换行计数器（如果设置）（例如 Canvas 2d），否则回退
  const fontWeight = n.fontWeight as string | number | undefined;
  const fontFamily =
    (typeof n.fontFamily === 'string' ? n.fontFamily : '') ||
    'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif';
  const letterSpacing = typeof n.letterSpacing === 'number' ? n.letterSpacing : 0;
  const rawLines = content.split(/\r?\n/);
  // Add 与渲染器的 wrapLine 容差匹配 (w + fontSize * 0.2)
  const wrapWidth = textWidth + fontSize * 0.2;

  const wrappedLineCount = _wrappedLineCounter
    ? _wrappedLineCounter(rawLines, wrapWidth, fontSize, fontWeight, fontFamily, letterSpacing)
    : countWrappedLinesFallback(rawLines, wrapWidth, fontSize, letterSpacing, fontWeight);

  const totalLines = Math.max(1, wrappedLineCount);
  return Math.round(totalLines <= 1 ? glyphH : (totalLines - 1) * lineStep + glyphH);
}
