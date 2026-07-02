import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import {
  parseSizing,
  defaultLineHeight,
  isCjkCodePoint,
  hasCjkText,
  estimateGlyphWidth,
  estimateLineWidth,
  estimateTextWidth,
  resolveTextContent,
  countExplicitTextLines,
  estimateTextHeight,
} from '../layout/text-measure';

describe('text-measure', () => {
  describe('parseSizing', () => {
    it('returns number for number input', () => {
      expect(parseSizing(100)).toBe(100);
    });

    it('returns "fill" for fill_container', () => {
      expect(parseSizing('fill_container')).toBe('fill');
    });

    it('returns "fit" for fit_content', () => {
      expect(parseSizing('fit_content')).toBe('fit');
    });

    it('uses numeric hints from fit_content(N)', () => {
      expect(parseSizing('fit_content(600)')).toBe(600);
    });

    it('parses numeric strings', () => {
      expect(parseSizing('200')).toBe(200);
    });

    it('returns 0 for non-parseable', () => {
      expect(parseSizing(undefined)).toBe(0);
      expect(parseSizing('abc')).toBe(0);
    });
  });

  describe('defaultLineHeight', () => {
    it('returns tight leading for display text', () => {
      expect(defaultLineHeight(48)).toBe(1.0);
    });

    it('returns comfortable leading for body text', () => {
      expect(defaultLineHeight(14)).toBe(1.5);
    });
  });

  describe('CJK detection', () => {
    it('detects CJK code points', () => {
      expect(isCjkCodePoint('中'.codePointAt(0)!)).toBe(true);
      expect(isCjkCodePoint('あ'.codePointAt(0)!)).toBe(true);
      expect(isCjkCodePoint('A'.codePointAt(0)!)).toBe(false);
    });

    it('hasCjkText detects CJK in strings', () => {
      expect(hasCjkText('Hello 世界')).toBe(true);
      expect(hasCjkText('Hello World')).toBe(false);
    });
  });

  describe('estimateGlyphWidth', () => {
    it('returns 0 for newline', () => {
      expect(estimateGlyphWidth('\n', 16)).toBe(0);
    });

    it('estimates CJK wider than Latin', () => {
      const cjk = estimateGlyphWidth('中', 16);
      const latin = estimateGlyphWidth('a', 16);
      expect(cjk).toBeGreaterThan(latin);
    });

    it('estimates uppercase wider than lowercase', () => {
      const upper = estimateGlyphWidth('A', 16);
      const lower = estimateGlyphWidth('a', 16);
      expect(upper).toBeGreaterThan(lower);
    });
  });

  describe('estimateLineWidth', () => {
    it('estimates width of a line', () => {
      const width = estimateLineWidth('Hello', 16);
      expect(width).toBeGreaterThan(0);
    });

    it('adds letter spacing', () => {
      const base = estimateLineWidth('AB', 16, 0);
      const spaced = estimateLineWidth('AB', 16, 5);
      expect(spaced).toBeGreaterThan(base);
    });
  });

  describe('estimateTextWidth', () => {
    it('returns the widest line', () => {
      const width = estimateTextWidth('short\nmuch longer line', 16);
      const singleWidth = estimateTextWidth('much longer line', 16);
      // Multi-line should return width of longest line
      expect(width).toBeCloseTo(singleWidth, 0);
    });
  });

  describe('resolveTextContent', () => {
    it('resolves string content', () => {
      const node: PenNode = { id: '1', type: 'text', content: 'Hello' };
      expect(resolveTextContent(node)).toBe('Hello');
    });

    it('resolves styled segment content', () => {
      const node: PenNode = {
        id: '1',
        type: 'text',
        content: [{ text: 'Hello ' }, { text: 'World' }],
      };
      expect(resolveTextContent(node)).toBe('Hello World');
    });

    it('returns empty for non-text nodes', () => {
      const node: PenNode = { id: '1', type: 'rectangle' };
      expect(resolveTextContent(node)).toBe('');
    });
  });

  describe('countExplicitTextLines', () => {
    it('counts newlines', () => {
      expect(countExplicitTextLines('a\nb\nc')).toBe(3);
    });

    it('returns 1 for empty string', () => {
      expect(countExplicitTextLines('')).toBe(1);
    });
  });

  describe('estimateTextHeight', () => {
    it('estimates height for single-line text', () => {
      const node: PenNode = { id: '1', type: 'text', content: 'Hello', fontSize: 16 };
      const height = estimateTextHeight(node);
      expect(height).toBeGreaterThan(0);
      expect(height).toBeLessThan(50);
    });

    it('estimates taller for multi-line text', () => {
      const single: PenNode = { id: '1', type: 'text', content: 'Hello', fontSize: 16 };
      const multi: PenNode = { id: '2', type: 'text', content: 'Hello\nWorld', fontSize: 16 };
      expect(estimateTextHeight(multi)).toBeGreaterThan(estimateTextHeight(single));
    });
  });
});
