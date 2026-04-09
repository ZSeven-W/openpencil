// Re-export pure functions from @zseven-w/pen-core
export {
  parseSizing,
  defaultLineHeight,
  isCjkCodePoint,
  hasCjkText,
  estimateGlyphWidth,
  estimateLineWidth,
  widthSafetyFactor,
  estimateTextWidth,
  estimateTextWidthPrecise,
  resolveTextContent,
  countExplicitTextLines,
  getTextOpticalCenterYOffset,
  estimateTextHeight,
  setWrappedLineCounter,
} from '@zseven-w/pen-core';

import {
  isCjkCodePoint,
  estimateLineWidth,
  widthSafetyFactor,
  setWrappedLineCounter,
} from '@zseven-w/pen-core';
import { cssFontFamily } from './font-utils';

// ---------------------------------------------------------------------------
// Canvas 2D measurement context (lazy singleton, browser-only)
// Wire up the browser-based wrapped line counter at module load time.
// ---------------------------------------------------------------------------

let _textMeasureCtx: CanvasRenderingContext2D | null = null;
function getTextMeasureCtx(): CanvasRenderingContext2D | null {
  if (typeof document === 'undefined') return null;
  if (!_textMeasureCtx) {
    const c = document.createElement('canvas');
    _textMeasureCtx = c.getContext('2d');
  }
  return _textMeasureCtx;
}

function countWrappedLinesCanvas2D(
  rawLines: string[],
  wrapWidth: number,
  fontSize: number,
  fontWeight: string | number | undefined,
  fontFamily: string,
  letterSpacing: number,
): number {
  const ctx = getTextMeasureCtx();
  if (!ctx) {
    return rawLines.reduce((sum, line) => {
      const lineWidth =
        estimateLineWidth(line, fontSize, letterSpacing, fontWeight) * widthSafetyFactor(line);
      return sum + Math.max(1, Math.ceil(lineWidth / wrapWidth));
    }, 0);
  }

  const fw = typeof fontWeight === 'number' ? String(fontWeight) : (fontWeight ?? '400');
  ctx.font = `${fw} ${fontSize}px ${cssFontFamily(fontFamily)}`;

  let total = 0;
  for (const rawLine of rawLines) {
    if (!rawLine) {
      total += 1;
      continue;
    }
    if (ctx.measureText(rawLine).width <= wrapWidth) {
      total += 1;
      continue;
    }
    let lineCount = 0;
    let current = '';
    let i = 0;
    while (i < rawLine.length) {
      const ch = rawLine[i];
      if (isCjkCodePoint(ch.codePointAt(0) ?? 0)) {
        const test = current + ch;
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++;
          current = ch;
        } else {
          current = test;
        }
        i++;
      } else if (ch === ' ') {
        const test = current + ch;
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++;
          current = '';
        } else {
          current = test;
        }
        i++;
      } else {
        let word = '';
        while (
          i < rawLine.length &&
          rawLine[i] !== ' ' &&
          !isCjkCodePoint(rawLine[i].codePointAt(0) ?? 0)
        ) {
          word += rawLine[i];
          i++;
        }
        const test = current + word;
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++;
          current = word;
        } else {
          current = test;
        }
      }
    }
    if (current) lineCount++;
    total += Math.max(1, lineCount);
  }
  return total;
}

// Register the Canvas 2D counter so pen-core's estimateTextHeight uses it
setWrappedLineCounter(countWrappedLinesCanvas2D);
