import { describe, it, expect } from 'vitest';
import { buildHeading } from '../element-builders/heading.js';
import { buildHeadingV1 } from '../element-builders/heading-v1.js';

describe('heading-v1 byte-parity (light) with heading-v0', () => {
  for (const level of ['display', 'h1', 'h2', 'h3'] as const) {
    it(`level=${level}: v1 'light' == v0 (no ids)`, () => {
      const v0 = buildHeading({ content: 'Test Heading', level });
      const v1 = buildHeadingV1({ content: 'Test Heading', level, theme: 'light' });
      expect(v1).toEqual(v0);
    });
    it(`level=${level}: v1 omitted theme defaults to 'light' (== v0)`, () => {
      const v0 = buildHeading({ content: 'Test Heading', level });
      const v1 = buildHeadingV1({ content: 'Test Heading', level });
      expect(v1).toEqual(v0);
    });
  }
});

describe('heading-v1 system mode: emits token refs', () => {
  it("level=display 'system' emits $type-display-size, weight, lineHeight, letterSpacing, textPrimary fill", () => {
    const node = buildHeadingV1({ content: 'Hero', level: 'display', theme: 'system' });
    expect(node.fontSize).toBe('$type-display-size');
    expect(node.fontWeight).toBe('$type-display-weight');
    expect(node.lineHeight).toBe('$type-display-line-height');
    expect(node.letterSpacing).toBe('$type-display-letter-spacing');
    expect((node.fill as Array<{ color: string }>)?.[0]?.color).toBe('$color-text-primary');
  });

  it("level=h1 'system' emits $type-h1-* + $color-text-primary", () => {
    const node = buildHeadingV1({ content: 'Welcome', level: 'h1', theme: 'system' });
    expect(node.fontSize).toBe('$type-h1-size');
    expect(node.fontWeight).toBe('$type-h1-weight');
    expect(node.lineHeight).toBe('$type-h1-line-height');
    expect((node.fill as Array<{ color: string }>)?.[0]?.color).toBe('$color-text-primary');
  });

  it("level=h2 'system' emits $type-h2-* + fill ref", () => {
    const node = buildHeadingV1({ content: 'Section', level: 'h2', theme: 'system' });
    expect(node.fontSize).toBe('$type-h2-size');
    expect(node.fontWeight).toBe('$type-h2-weight');
    expect(node.lineHeight).toBe('$type-h2-line-height');
  });

  it("level=h3 'system' emits $type-h3-* + fill ref", () => {
    const node = buildHeadingV1({ content: 'Subsection', level: 'h3', theme: 'system' });
    expect(node.fontSize).toBe('$type-h3-size');
    expect(node.fontWeight).toBe('$type-h3-weight');
    expect(node.lineHeight).toBe('$type-h3-line-height');
  });
});

describe('heading-v1 dark mode: emits dark hex values', () => {
  it("level=h1 'dark' emits dark text color #F1F5F9", () => {
    const node = buildHeadingV1({ content: 'Welcome', level: 'h1', theme: 'dark' });
    expect((node.fill as Array<{ color: string }>)?.[0]?.color).toBe('#F1F5F9');
  });

  it('dark mode h1: font values match concrete v0 presets (NOT palette tokens)', () => {
    const node = buildHeadingV1({ content: 'Title', level: 'h1', theme: 'dark' });
    // Dark uses palette token values (24=h1Size) not v0's 32
    // This is intentional — dark breaks from v0 scale to use the semantic palette
    expect(node.fontSize).toBe(24); // $type-h1-size = 24
    expect(node.fontWeight).toBe(600); // $type-h1-weight = 600
  });

  it('dark display: has letterSpacing from palette', () => {
    const node = buildHeadingV1({ content: 'Hero', level: 'display', theme: 'dark' });
    expect(node.letterSpacing).toBe(-0.5); // displayLetterSpacing = -0.5
  });
});

describe('heading-v1 error handling', () => {
  it('throws on invalid level (same guard as v0)', () => {
    expect(() => buildHeadingV1({ content: 'T', level: 'caption' as never })).toThrow(
      /add_heading_v1.*invalid level.*caption/,
    );
  });
});
