import { describe, it, expect } from 'vitest';
import { buildEmptyChart, buildEmptyChartV1 } from '../index.js';

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

describe('buildEmptyChartV1 — v0 byte-parity (light)', () => {
  it('v1 light (omitted theme) == v0 output', () => {
    const v0 = buildEmptyChart({ title: 'No data', subtitle: 'Check later.' });
    const v1 = buildEmptyChartV1({ title: 'No data', subtitle: 'Check later.' });
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

  it('light mode: bg fill #F8FAFC (v0 parity)', () => {
    const t = buildEmptyChartV1({}) as unknown as Frame;
    expect(getFillColor(t)).toBe('#F8FAFC');
  });

  it('light mode: title fill #334155, fontSize=14, fontWeight=600 (v0 parity)', () => {
    const t = buildEmptyChartV1({}) as unknown as Frame;
    const title = findByRole(t, 'empty-chart-title') as unknown as {
      fontSize: unknown;
      fontWeight: unknown;
    };
    expect(getFillColor(title as unknown as Frame)).toBe('#334155');
    expect(title.fontSize).toBe(14);
    expect(title.fontWeight).toBe(600);
  });

  it('light mode: subtitle fill #64748B, fontSize=12, fontWeight=400 (v0 parity)', () => {
    const t = buildEmptyChartV1({}) as unknown as Frame;
    const sub = findByRole(t, 'empty-chart-subtitle') as unknown as {
      fontSize: unknown;
      fontWeight: unknown;
    };
    expect(getFillColor(sub as unknown as Frame)).toBe('#64748B');
    expect(sub.fontSize).toBe(12);
    expect(sub.fontWeight).toBe(400);
  });

  it('light mode: gap=8, padding=24, cornerRadius=12 (v0 parity)', () => {
    const t = buildEmptyChartV1({}) as unknown as {
      gap: unknown;
      padding: unknown;
      cornerRadius: unknown;
    };
    expect(t.gap).toBe(8);
    expect(t.padding).toBe(24);
    expect(t.cornerRadius).toBe(12);
  });
});

describe('buildEmptyChartV1 — dark mode', () => {
  it('dark mode: bg fill #1E293B (slate-800)', () => {
    const t = buildEmptyChartV1({ theme: 'dark' }) as unknown as Frame;
    expect(getFillColor(t)).toBe('#1E293B');
  });

  it('dark mode: icon fill #94A3B8 (text-muted dark)', () => {
    const t = buildEmptyChartV1({ theme: 'dark' }) as unknown as Frame;
    const icon = findByRole(t, 'empty-chart-icon')!;
    expect(getFillColor(icon)).toBe('#94A3B8');
  });
});

describe('buildEmptyChartV1 — system mode: full token coverage', () => {
  it("system: bg fill '$color-surface-2' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    expect(getFillColor(t)).toBe('$color-surface-2');
  });

  it("system: title fill '$color-text-primary' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const title = findByRole(t, 'empty-chart-title')!;
    expect(getFillColor(title)).toBe('$color-text-primary');
  });

  it("system: subtitle fill '$color-text-muted' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const sub = findByRole(t, 'empty-chart-subtitle')!;
    expect(getFillColor(sub)).toBe('$color-text-muted');
  });

  it("system: title fontSize is '$type-body-size' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const title = findByRole(t, 'empty-chart-title') as unknown as { fontSize: unknown };
    expect(title.fontSize).toBe('$type-body-size');
  });

  it("system: subtitle fontSize is '$type-caption-size' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const sub = findByRole(t, 'empty-chart-subtitle') as unknown as { fontSize: unknown };
    expect(sub.fontSize).toBe('$type-caption-size');
  });

  it("system: subtitle fontWeight is '$type-caption-weight' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const sub = findByRole(t, 'empty-chart-subtitle') as unknown as { fontWeight: unknown };
    expect(sub.fontWeight).toBe('$type-caption-weight');
  });

  it('system: title fontWeight=600 stays hardcoded (builder-private emphasis)', () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as Frame;
    const title = findByRole(t, 'empty-chart-title') as unknown as { fontWeight: unknown };
    expect(title.fontWeight).toBe(600);
  });

  it("system: gap is '$spacing-2' ref", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as { gap: unknown };
    expect(t.gap).toBe('$spacing-2');
  });

  it("system: padding is '$spacing-5' ref (default, no override)", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as { padding: unknown };
    expect(t.padding).toBe('$spacing-5');
  });

  it("system: cornerRadius is '$radius-lg' ref (default, no override)", () => {
    const t = buildEmptyChartV1({ theme: 'system' }) as unknown as { cornerRadius: unknown };
    expect(t.cornerRadius).toBe('$radius-lg');
  });

  it('system: explicit corner_radius override stays numeric', () => {
    const t = buildEmptyChartV1({ theme: 'system', corner_radius: 20 }) as unknown as {
      cornerRadius: unknown;
    };
    expect(t.cornerRadius).toBe(20);
  });
});
