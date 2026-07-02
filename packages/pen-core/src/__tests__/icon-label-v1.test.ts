import { describe, it, expect } from 'vitest';
import { buildIconLabel } from '../element-builders/icon-label.js';
import { buildIconLabelV1 } from '../element-builders/icon-label-v1.js';

function stripTheme<T extends Record<string, unknown>>(obj: T): Omit<T, 'theme'> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { theme: _t, ...rest } = obj;
  return rest;
}

const BASIC = { icon: 'star', label: 'Favorites' };

describe('buildIconLabelV1 — byte-parity with v0 (light)', () => {
  it('default params match v0', () => {
    const v0 = buildIconLabel(BASIC) as Record<string, unknown>;
    const v1 = buildIconLabelV1(BASIC) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('custom gap matches v0', () => {
    const v0 = buildIconLabel({ ...BASIC, gap: 12 }) as Record<string, unknown>;
    const v1 = buildIconLabelV1({ ...BASIC, gap: 12 }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('explicit theme=light still matches v0', () => {
    const v0 = buildIconLabel(BASIC) as Record<string, unknown>;
    const v1 = buildIconLabelV1({ ...BASIC, theme: 'light' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('icon child: icon_font with lucide family, 16×16', () => {
    const v1 = buildIconLabelV1(BASIC) as Record<string, unknown>;
    const children = v1.children as Array<Record<string, unknown>>;
    const icon = children[0];
    expect(icon.type).toBe('icon_font');
    expect(icon.iconFontFamily).toBe('lucide');
    expect(icon.width).toBe(16);
    expect(icon.height).toBe(16);
  });

  it('label child: fontSize=14, fontWeight=500', () => {
    const v1 = buildIconLabelV1(BASIC) as Record<string, unknown>;
    const children = v1.children as Array<Record<string, unknown>>;
    const label = children[1];
    expect(label.fontSize).toBe(14);
    expect(label.fontWeight).toBe(500);
  });

  it('default gap=8', () => {
    const v1 = buildIconLabelV1(BASIC) as Record<string, unknown>;
    expect(v1.gap).toBe(8);
  });
});

describe('buildIconLabelV1 — dark mode (no-color tool, identical to light)', () => {
  it('theme=dark output identical to theme=light', () => {
    const light = buildIconLabelV1(BASIC) as Record<string, unknown>;
    const dark = buildIconLabelV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });
});

describe('buildIconLabelV1 — system mode (no-color tool, identical to light)', () => {
  it('theme=system output identical to theme=light', () => {
    const light = buildIconLabelV1(BASIC) as Record<string, unknown>;
    const system = buildIconLabelV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(system)));
  });

  it('no $color-* refs emitted', () => {
    const system = JSON.stringify(buildIconLabelV1({ ...BASIC, theme: 'system' }));
    expect(system).not.toContain('$color-');
  });
});
