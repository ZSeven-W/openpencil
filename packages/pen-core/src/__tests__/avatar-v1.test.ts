import { describe, it, expect } from 'vitest';
import { buildAvatar } from '../element-builders/avatar.js';
import { buildAvatarV1 } from '../element-builders/avatar-v1.js';

function stripTheme<T extends Record<string, unknown>>(obj: T): Omit<T, 'theme'> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { theme: _t, ...rest } = obj;
  return rest;
}

describe('buildAvatarV1 — byte-parity with v0 (light)', () => {
  it('no initial: frame shape matches v0', () => {
    const v0 = buildAvatar({}) as Record<string, unknown>;
    const v1 = buildAvatarV1({}) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('with initial: full tree matches v0', () => {
    const v0 = buildAvatar({ initial: 'SL', size: 48 }) as Record<string, unknown>;
    const v1 = buildAvatarV1({ initial: 'SL', size: 48 }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('explicit theme=light still matches v0', () => {
    const v0 = buildAvatar({ initial: 'A' }) as Record<string, unknown>;
    const v1 = buildAvatarV1({ initial: 'A', theme: 'light' }) as Record<string, unknown>;
    expect(JSON.stringify(stripTheme(v1))).toBe(JSON.stringify(v0));
  });

  it('cornerRadius = size/2', () => {
    const v1 = buildAvatarV1({ size: 60 }) as Record<string, unknown>;
    expect(v1.cornerRadius).toBe(30);
  });

  it('initial fontSize = max(12, round(size * 0.4))', () => {
    const v1 = buildAvatarV1({ initial: 'X', size: 40 }) as Record<string, unknown>;
    const children = v1.children as Array<Record<string, unknown>>;
    expect(children[0].fontSize).toBe(16); // max(12, round(40 * 0.4)) = 16
  });
});

describe('buildAvatarV1 — dark mode (no-color tool, identical to light)', () => {
  it('theme=dark output identical to theme=light', () => {
    const light = buildAvatarV1({ initial: 'SL', size: 48 }) as Record<string, unknown>;
    const dark = buildAvatarV1({ initial: 'SL', size: 48, theme: 'dark' }) as Record<
      string,
      unknown
    >;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(dark)));
  });
});

describe('buildAvatarV1 — system mode (no-color tool, identical to light)', () => {
  it('theme=system output identical to theme=light', () => {
    const light = buildAvatarV1({ initial: 'SL', size: 48 }) as Record<string, unknown>;
    const system = buildAvatarV1({ initial: 'SL', size: 48, theme: 'system' }) as Record<
      string,
      unknown
    >;
    expect(JSON.stringify(stripTheme(light))).toBe(JSON.stringify(stripTheme(system)));
  });

  it('no $color-* refs emitted (theme param is accepted but has no effect)', () => {
    const system = JSON.stringify(buildAvatarV1({ initial: 'SL', theme: 'system' }));
    expect(system).not.toContain('$color-');
  });
});
