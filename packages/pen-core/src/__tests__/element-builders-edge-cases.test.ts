import { describe, it, expect } from 'vitest';
import {
  buildBadge,
  buildBodyText,
  buildCalendarGrid,
  buildCardRow,
  buildChartBars,
  buildEmptyState,
  buildHeading,
  buildKbd,
  buildPrice,
  buildRatingStars,
  buildStepper,
  buildTimeline,
} from '../element-builders/index.js';

/**
 * Edge-case coverage gap fill: extreme values, i18n scripts,
 * minimal-input shapes. These won't usually happen in AI output
 * but the builders should still be robust — silent weird output
 * is worse than a clear reject.
 */

describe('i18n — scripts not in the primary CJK set', () => {
  it('heading with pure Arabic content stays Latin path (no CJK Noto dispatch)', () => {
    const h = buildHeading({ content: 'مرحبا بالعالم' }) as Record<string, unknown>;
    // Arabic is neither hiragana/hangul/han → null → Latin preset
    expect((h as { fontFamily?: string }).fontFamily).toBeUndefined();
  });
  it('heading with emoji-only content stays Latin', () => {
    const h = buildHeading({ content: '🎉🚀' }) as Record<string, unknown>;
    expect((h as { fontFamily?: string }).fontFamily).toBeUndefined();
  });
  it('heading with mixed Latin+hiragana resolves to Japanese (hiragana wins)', () => {
    const h = buildHeading({ content: 'Hello こんにちは World' }) as Record<string, unknown>;
    expect(h.fontFamily).toBe('Noto Sans JP');
  });
  it('heading with mixed Han+Hangul resolves to Korean (hangul checked before han)', () => {
    const h = buildHeading({ content: '안녕 世界' }) as Record<string, unknown>;
    expect(h.fontFamily).toBe('Noto Sans KR');
  });
  it('body_text always Inter even for CJK (body never dispatches)', () => {
    const j = buildBodyText({ content: 'こんにちは' }) as Record<string, unknown>;
    const k = buildBodyText({ content: '안녕하세요' }) as Record<string, unknown>;
    const c = buildBodyText({ content: '你好' }) as Record<string, unknown>;
    expect(j.fontFamily).toBe('Inter');
    expect(k.fontFamily).toBe('Inter');
    expect(c.fontFamily).toBe('Inter');
  });
});

describe('extreme numeric values', () => {
  it('chart_bars with single huge value scales max correctly', () => {
    const c = buildChartBars({ values: [1e9], chart_height: 200 }) as Record<string, unknown>;
    const bars = c.children as Array<{ height: number }>;
    expect(bars).toHaveLength(1);
    expect(bars[0].height).toBe(200); // 1e9/max(1,1e9) * 200 = 200
  });
  it('chart_bars all-zero → every bar at 2px floor', () => {
    const c = buildChartBars({ values: [0, 0, 0] }) as Record<string, unknown>;
    const bars = c.children as Array<{ height: number }>;
    bars.forEach((b) => expect(b.height).toBe(2));
  });
  it('stepper total=100 → 199 children (100 steps + 99 connectors)', () => {
    const s = buildStepper({ total: 100, current: 50 }) as Record<string, unknown>;
    expect(s.children as unknown[]).toHaveLength(199);
  });
  it('rating_stars filled higher than total clamps without duplicating children', () => {
    const r = buildRatingStars({ filled: 999, total: 3 }) as Record<string, unknown>;
    expect(r.children as unknown[]).toHaveLength(3);
  });
});

describe('minimal inputs', () => {
  it('card_row with single item still emits wrapper + inner + 1 card', () => {
    const t = buildCardRow({ items: [{ title: 'Solo' }] }) as Record<string, unknown>;
    const inner = (t.children as Array<{ children: unknown[] }>)[0];
    expect(inner.children).toHaveLength(1);
  });
  it('empty_state title-only has no subtitle / icon / CTA children', () => {
    const e = buildEmptyState({ title: 'Nothing' }) as Record<string, unknown>;
    expect(e.children as unknown[]).toHaveLength(1);
  });
  it('timeline with single item has NO connector', () => {
    const t = buildTimeline({ items: [{ title: 'Solo' }] }) as Record<string, unknown>;
    const iconCol = (
      (t.children as Array<{ children: unknown[] }>)[0].children as unknown[]
    )[0] as { children: unknown[] };
    expect(iconCol.children).toHaveLength(1); // dot only, no connector
  });
});

describe('extreme label lengths', () => {
  it('badge with 40-char label still emits single text child (no wrapping logic)', () => {
    const b = buildBadge({
      label: 'THIS IS DEFINITELY TOO LONG FOR A BADGE!!',
    }) as Record<string, unknown>;
    expect(b.children as unknown[]).toHaveLength(1);
  });
  it('price with large formatted amount preserves exact string', () => {
    const p = buildPrice({ amount: '999,999,999.99', currency: '¥' }) as Record<string, unknown>;
    const children = p.children as Array<{ content: string }>;
    expect(children[0].content).toBe('¥');
    expect(children[1].content).toBe('999,999,999.99');
  });
  it('kbd separator="" → N keys + zero separators = N children', () => {
    const k = buildKbd({ keys: ['⌃', '⌥', '⌘', 'K'], separator: '' }) as Record<string, unknown>;
    expect(k.children as unknown[]).toHaveLength(4);
  });
});

describe('calendar_grid boundary cases', () => {
  it('28-day month offset 0 → 1 header + 4 week rows', () => {
    const g = buildCalendarGrid({ days_in_month: 28 }) as Record<string, unknown>;
    expect(g.children as unknown[]).toHaveLength(1 + 4);
  });
  it('31-day month offset 6 → 1 header + 6 week rows (6×7 cells)', () => {
    const g = buildCalendarGrid({ days_in_month: 31, start_day_offset: 6 }) as Record<
      string,
      unknown
    >;
    expect(g.children as unknown[]).toHaveLength(1 + 6);
  });
  it('days_in_month lower clamp → 1 day, still emits 1 header + 1 week row', () => {
    const g = buildCalendarGrid({ days_in_month: 0 }) as Record<string, unknown>;
    expect(g.children as unknown[]).toHaveLength(1 + 1);
  });
  it('start_day_offset clamped upper bound (7 → 6)', () => {
    const g = buildCalendarGrid({
      days_in_month: 30,
      start_day_offset: 99,
    }) as Record<string, unknown>;
    // 30 + 6 = 36 → ceil(36/7) = 6 weeks
    expect(g.children as unknown[]).toHaveLength(1 + 6);
  });
  it("today=0 and selected_day=0 (invalid day numbers) don't crash + no cell marked", () => {
    const g = buildCalendarGrid({ days_in_month: 30, today: 0, selected_day: 0 }) as Record<
      string,
      unknown
    >;
    const rows = g.children as Array<{ children: Array<{ role: string }> }>;
    // First week row (rows[1]) — day 1 is the first non-empty cell
    const week1 = rows[1].children;
    const hasTodayOrSelected = week1.some(
      (c) => c.role === 'calendar-day-today' || c.role === 'calendar-day-selected',
    );
    expect(hasTodayOrSelected).toBe(false);
  });
});

describe('buildHeading — invalid level rejection', () => {
  // ab-v4 (2026-05-01) caught gpt-5.4 inventing `level: "caption"` on a
  // composite multi-tool sub-task. The preset lookup silently returned
  // undefined and the next line crashed with the cryptic message
  // `undefined is not an object (evaluating 'preset.fontSize')`,
  // failing the WHOLE composite apply mid-batch. Builder now rejects
  // unknown levels at the entry boundary so the surrounding dispatch
  // loop captures a clear per-shape error and keeps applying the rest.
  it('throws with a helpful message when level is not a known enum value', () => {
    expect(() =>
      buildHeading({ content: 'Pending invitations', level: 'caption' as never }),
    ).toThrow(/invalid level "caption"; expected one of: display, h1, h2, h3/);
  });

  it('does not throw for any of the four valid levels', () => {
    for (const level of ['display', 'h1', 'h2', 'h3'] as const) {
      expect(() => buildHeading({ content: 'ok', level })).not.toThrow();
    }
  });

  it('does not throw when level is omitted (defaults to h2)', () => {
    expect(() => buildHeading({ content: 'no level' })).not.toThrow();
  });
});
