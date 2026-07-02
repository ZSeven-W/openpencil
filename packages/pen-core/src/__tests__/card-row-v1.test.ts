import { describe, it, expect } from 'vitest';
import { buildCardRow } from '../element-builders/card-row.js';
import { buildCardRowV1 } from '../element-builders/card-row-v1.js';

const ITEMS = [{ title: 'Card A', subtitle: 'Sub' }, { title: 'Card B' }];

function getFirstCard(tree: Record<string, unknown>): Record<string, unknown> {
  const scroll = tree as { children: Array<{ children: Record<string, unknown>[] }> };
  return scroll.children[0].children[0] as Record<string, unknown>;
}

describe('buildCardRowV1 — byte-parity with v0 (light)', () => {
  it('scroll wrapper structure identical (id excluded)', () => {
    const v0 = buildCardRow({ items: ITEMS }) as Record<string, unknown>;
    const v1 = buildCardRowV1({ items: ITEMS }) as Record<string, unknown>;
    // Top-level scroll wrapper
    expect((v1 as { type: string }).type).toBe('frame');
    expect((v1 as { name: string }).name).toBe((v0 as { name: string }).name);
  });

  it('first card: cornerRadius=20, width=140, height=160 — v0 parity', () => {
    const v0 = buildCardRow({ items: ITEMS }) as Record<string, unknown>;
    const v1 = buildCardRowV1({ items: ITEMS }) as Record<string, unknown>;
    const c0 = getFirstCard(v0);
    const c1 = getFirstCard(v1);
    expect(c1.cornerRadius).toBe(c0.cornerRadius); // 20
    expect(c1.width).toBe(c0.width); // 140
    expect(c1.height).toBe(c0.height); // 160
  });

  it('title: fontSize=16, fontWeight=600, no fill in light — v0 parity', () => {
    const v1 = buildCardRowV1({ items: ITEMS }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const title = (card.children as Record<string, unknown>[])[0];
    expect(title.fontSize).toBe(16);
    expect(title.fontWeight).toBe(600);
    expect(title.fill).toBeUndefined();
  });

  it('subtitle: fontSize=13, fontWeight=400, no fill in light — v0 parity', () => {
    const v1 = buildCardRowV1({ items: ITEMS }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const subtitle = (card.children as Record<string, unknown>[])[1];
    expect(subtitle.fontSize).toBe(13);
    expect(subtitle.fontWeight).toBe(400);
    expect(subtitle.fill).toBeUndefined();
  });

  it('card: no fill in light — v0 parity', () => {
    const v1 = buildCardRowV1({ items: ITEMS }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    expect(card.fill).toBeUndefined();
  });
});

describe('buildCardRowV1 — dark mode', () => {
  it('card fill = #1E293B (dark surface) in dark mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'dark' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const fill = card.fill as Array<{ color: string }>;
    expect(fill?.[0]?.color).toBe('#1E293B');
  });

  it('title fill = #F1F5F9 (dark textPrimary) in dark mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'dark' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const title = (card.children as Array<{ fill: Array<{ color: string }> }>)[0];
    expect(title.fill?.[0]?.color).toBe('#F1F5F9');
  });

  it('subtitle fill = #94A3B8 (dark textMuted) in dark mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'dark' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const subtitle = (card.children as Array<{ fill: Array<{ color: string }> }>)[1];
    expect(subtitle.fill?.[0]?.color).toBe('#94A3B8');
  });
});

describe('buildCardRowV1 — system mode emits refs', () => {
  it('card fill = $color-surface in system mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'system' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const fill = card.fill as Array<{ color: string }>;
    expect(fill?.[0]?.color).toBe('$color-surface');
  });

  it('title fill = $color-text-primary in system mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'system' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const title = (card.children as Array<{ fill: Array<{ color: string }> }>)[0];
    expect(title.fill?.[0]?.color).toBe('$color-text-primary');
  });

  it('subtitle fill = $color-text-muted in system mode', () => {
    const v1 = buildCardRowV1({ items: ITEMS, theme: 'system' }) as Record<string, unknown>;
    const card = getFirstCard(v1);
    const subtitle = (card.children as Array<{ fill: Array<{ color: string }> }>)[1];
    expect(subtitle.fill?.[0]?.color).toBe('$color-text-muted');
  });
});
