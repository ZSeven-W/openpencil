import { describe, it, expect } from 'vitest';
import { buildBadge } from '../element-builders/badge.js';
import { buildBadgeV1 } from '../element-builders/badge-v1.js';

function stripTheme<T extends Record<string, unknown>>(obj: T): Omit<T, 'theme'> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { theme: _t, ...rest } = obj;
  return rest;
}

describe('buildBadgeV1 — byte-parity with v0 (light)', () => {
  it('output matches v0 (no theme)', () => {
    const v0 = buildBadge({ label: 'NEW' }) as Record<string, unknown>;
    const v1 = buildBadgeV1({ label: 'NEW' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('explicit theme=light still matches v0', () => {
    const v0 = buildBadge({ label: 'BETA' }) as Record<string, unknown>;
    const v1 = buildBadgeV1({ label: 'BETA', theme: 'light' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('cornerRadius=999 (full pill)', () => {
    const v1 = buildBadgeV1({ label: '42' }) as Record<string, unknown>;
    expect(v1.cornerRadius).toBe(999);
  });

  it('label child: fontSize=11, fontWeight=600', () => {
    const v1 = buildBadgeV1({ label: 'BETA' }) as Record<string, unknown>;
    const children = v1.children as Array<Record<string, unknown>>;
    expect(children[0].fontSize).toBe(11);
    expect(children[0].fontWeight).toBe(600);
  });
});

describe('buildBadgeV1 — dark mode (no-color tool, identical to light)', () => {
  it('theme=dark output identical to theme=light', () => {
    const light = buildBadgeV1({ label: 'NEW' }) as Record<string, unknown>;
    const dark = buildBadgeV1({ label: 'NEW', theme: 'dark' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });
});

describe('buildBadgeV1 — system mode (no-color tool, identical to light)', () => {
  it('theme=system output identical to theme=light', () => {
    const light = buildBadgeV1({ label: 'NEW' }) as Record<string, unknown>;
    const system = buildBadgeV1({ label: 'NEW', theme: 'system' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(system)));
  });

  it('no $color-* refs emitted', () => {
    const system = JSON.stringify(buildBadgeV1({ label: 'TEST', theme: 'system' }));
    expect(system).not.toContain('$color-');
  });
});
