import { describe, it, expect } from 'vitest';
import { buildBodyText } from '../element-builders/body-text.js';
import { buildBodyTextV1 } from '../element-builders/body-text-v1.js';

function stripTheme<T extends Record<string, unknown>>(obj: T): Omit<T, 'theme'> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { theme: _t, ...rest } = obj;
  return rest;
}

const LATIN_TEXT = 'The quick brown fox jumps over the lazy dog.';
const CJK_TEXT = '快速的棕色狐狸跳过了懒惰的狗。';

describe('buildBodyTextV1 — byte-parity with v0 (light)', () => {
  it('Latin content matches v0', () => {
    const v0 = buildBodyText({ content: LATIN_TEXT }) as Record<string, unknown>;
    const v1 = buildBodyTextV1({ content: LATIN_TEXT }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('CJK content matches v0 (lineHeight=1.6, letterSpacing=0)', () => {
    const v0 = buildBodyText({ content: CJK_TEXT }) as Record<string, unknown>;
    const v1 = buildBodyTextV1({ content: CJK_TEXT }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('explicit theme=light still matches v0', () => {
    const v0 = buildBodyText({ content: LATIN_TEXT }) as Record<string, unknown>;
    const v1 = buildBodyTextV1({ content: LATIN_TEXT, theme: 'light' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('Latin: lineHeight=1.5, no letterSpacing', () => {
    const v1 = buildBodyTextV1({ content: LATIN_TEXT }) as Record<string, unknown>;
    expect(v1.lineHeight).toBe(1.5);
    expect(v1.letterSpacing).toBeUndefined();
  });

  it('CJK: lineHeight=1.6, letterSpacing=0', () => {
    const v1 = buildBodyTextV1({ content: CJK_TEXT }) as Record<string, unknown>;
    expect(v1.lineHeight).toBe(1.6);
    expect(v1.letterSpacing).toBe(0);
  });

  it('always fontFamily=Inter, width=fill_container, textGrowth=fixed-width', () => {
    const v1 = buildBodyTextV1({ content: LATIN_TEXT }) as Record<string, unknown>;
    expect(v1.fontFamily).toBe('Inter');
    expect(v1.width).toBe('fill_container');
    expect(v1.textGrowth).toBe('fixed-width');
  });
});

describe('buildBodyTextV1 — dark mode (no-color tool, identical to light)', () => {
  it('theme=dark output identical to theme=light (Latin)', () => {
    const light = buildBodyTextV1({ content: LATIN_TEXT }) as Record<string, unknown>;
    const dark = buildBodyTextV1({ content: LATIN_TEXT, theme: 'dark' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });

  it('theme=dark output identical to theme=light (CJK)', () => {
    const light = buildBodyTextV1({ content: CJK_TEXT }) as Record<string, unknown>;
    const dark = buildBodyTextV1({ content: CJK_TEXT, theme: 'dark' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });
});

describe('buildBodyTextV1 — system mode (no-color tool, identical to light)', () => {
  it('theme=system output identical to theme=light', () => {
    const light = buildBodyTextV1({ content: LATIN_TEXT }) as Record<string, unknown>;
    const system = buildBodyTextV1({ content: LATIN_TEXT, theme: 'system' }) as Record<
      string,
      unknown
    >;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(system)));
  });

  it('no $color-* refs emitted', () => {
    const system = JSON.stringify(buildBodyTextV1({ content: LATIN_TEXT, theme: 'system' }));
    expect(system).not.toContain('$color-');
  });
});
