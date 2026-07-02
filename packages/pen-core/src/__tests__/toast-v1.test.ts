import { describe, it, expect } from 'vitest';
import { buildToast, buildToastV1 } from '../index.js';

interface Frame {
  role?: string;
  fill?: Array<{ type: string; color: string }>;
  fontSize?: unknown;
  fontWeight?: unknown;
  gap?: unknown;
  cornerRadius?: unknown;
  padding?: unknown;
  children?: Frame[];
}

function findByRole(n: Frame, role: string): Frame | undefined {
  if (n.role === role) return n;
  for (const c of n.children ?? []) {
    const hit = findByRole(c, role);
    if (hit) return hit;
  }
  return undefined;
}

function getFillColor(n: Frame): string | undefined {
  return n.fill?.[0]?.color;
}

describe('buildToastV1 — v0 byte-parity (light)', () => {
  it('v1 light (omitted theme) == v0 output', () => {
    const v0 = buildToast({ message: 'Saved!', icon: 'check' });
    const v1 = buildToastV1({ message: 'Saved!', icon: 'check' });
    // Strip ids for structural compare
    const stripIds = (obj: unknown): unknown => {
      if (Array.isArray(obj)) return obj.map(stripIds);
      if (obj && typeof obj === 'object') {
        const out: Record<string, unknown> = {};
        for (const [k, v] of Object.entries(obj)) {
          if (k === 'id') continue;
          out[k] = stripIds(v);
        }
        return out;
      }
      return obj;
    };
    expect(stripIds(v1)).toEqual(stripIds(v0));
  });

  it('light mode: pill fill #111827 (v0 dark pill)', () => {
    const t = buildToastV1({ message: 'Saved!' }) as unknown as Frame;
    expect(getFillColor(t)).toBe('#111827');
  });

  it('light mode: message fill #FFFFFF (v0 white fg)', () => {
    const t = buildToastV1({ message: 'Saved!' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message')!;
    expect(getFillColor(msg)).toBe('#FFFFFF');
  });

  it('light mode: message fontSize=14, fontWeight=500 (v0 parity)', () => {
    const t = buildToastV1({ message: 'Saved!' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message') as unknown as {
      fontSize: unknown;
      fontWeight: unknown;
    };
    expect(msg.fontSize).toBe(14);
    expect(msg.fontWeight).toBe(500);
  });

  it('light mode: gap=8 (v0 parity)', () => {
    const t = buildToastV1({ message: 'Saved!' }) as unknown as { gap: unknown };
    expect(t.gap).toBe(8);
  });
});

describe('buildToastV1 — dark mode', () => {
  it('dark mode: pill fill #F1F5F9 (light pill, inverted)', () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'dark' }) as unknown as Frame;
    expect(getFillColor(t)).toBe('#F1F5F9');
  });

  it('dark mode: fg fill #0F172A (dark text on light pill)', () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'dark' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message')!;
    expect(getFillColor(msg)).toBe('#0F172A');
  });
});

describe('buildToastV1 — system mode: full token coverage', () => {
  it("system: pill fill is '$color-text-primary' ref (inverted bg)", () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as Frame;
    expect(getFillColor(t)).toBe('$color-text-primary');
  });

  it("system: message fill is '$color-surface' ref (inverted fg)", () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message')!;
    expect(getFillColor(msg)).toBe('$color-surface');
  });

  it("system: message fontSize is '$type-body-size' ref", () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message') as unknown as { fontSize: unknown };
    expect(msg.fontSize).toBe('$type-body-size');
  });

  it('system: message fontWeight=500 stays hardcoded (not in token system)', () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as Frame;
    const msg = findByRole(t, 'toast-message') as unknown as { fontWeight: unknown };
    expect(msg.fontWeight).toBe(500);
  });

  it("system: gap is '$spacing-2' ref", () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as { gap: unknown };
    expect(t.gap).toBe('$spacing-2');
  });

  it('system: cornerRadius=24 stays hardcoded (pill shape, builder-private)', () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as {
      cornerRadius: unknown;
    };
    expect(t.cornerRadius).toBe(24);
  });

  it('system: padding stays [12, 20] hardcoded (20 is not a token)', () => {
    const t = buildToastV1({ message: 'Saved!', theme: 'system' }) as unknown as {
      padding: unknown;
    };
    expect(t.padding).toEqual([12, 20]);
  });
});
