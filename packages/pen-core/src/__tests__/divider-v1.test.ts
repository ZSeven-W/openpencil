import { describe, it, expect } from 'vitest';
import { buildDivider } from '../element-builders/divider.js';
import { buildDividerV1 } from '../element-builders/divider-v1.js';

function stripTheme<T extends Record<string, unknown>>(obj: T): Omit<T, 'theme'> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { theme: _t, ...rest } = obj;
  return rest;
}

describe('buildDividerV1 — byte-parity with v0 (light)', () => {
  it('default horizontal matches v0', () => {
    const v0 = buildDivider({}) as Record<string, unknown>;
    const v1 = buildDividerV1({}) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('vertical orientation matches v0', () => {
    const v0 = buildDivider({ orientation: 'vertical' }) as Record<string, unknown>;
    const v1 = buildDividerV1({ orientation: 'vertical' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('custom thickness matches v0', () => {
    const v0 = buildDivider({ thickness: 2 }) as Record<string, unknown>;
    const v1 = buildDividerV1({ thickness: 2 }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('explicit theme=light still matches v0', () => {
    const v0 = buildDivider({ orientation: 'horizontal', thickness: 1 }) as Record<string, unknown>;
    const v1 = buildDividerV1({
      orientation: 'horizontal',
      thickness: 1,
      theme: 'light',
    }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('horizontal: width=fill_container, height=1', () => {
    const v1 = buildDividerV1({}) as Record<string, unknown>;
    expect(v1.width).toBe('fill_container');
    expect(v1.height).toBe(1);
  });

  it('vertical: width=1, height=fill_container', () => {
    const v1 = buildDividerV1({ orientation: 'vertical' }) as Record<string, unknown>;
    expect(v1.width).toBe(1);
    expect(v1.height).toBe('fill_container');
  });
});

describe('buildDividerV1 — dark mode (no-color tool, identical to light)', () => {
  it('theme=dark output identical to theme=light', () => {
    const light = buildDividerV1({ orientation: 'horizontal' }) as Record<string, unknown>;
    const dark = buildDividerV1({ orientation: 'horizontal', theme: 'dark' }) as Record<
      string,
      unknown
    >;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });
});

describe('buildDividerV1 — system mode (no-color tool, identical to light)', () => {
  it('theme=system output identical to theme=light', () => {
    const light = buildDividerV1({}) as Record<string, unknown>;
    const system = buildDividerV1({ theme: 'system' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(system)));
  });

  it('no $color-* refs emitted', () => {
    const system = JSON.stringify(buildDividerV1({ theme: 'system' }));
    expect(system).not.toContain('$color-');
  });
});
