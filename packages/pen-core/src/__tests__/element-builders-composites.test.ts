import { describe, it, expect } from 'vitest';
import {
  buildActivityRing,
  buildAlert,
  buildCalendarGrid,
  buildCheckbox,
  buildCodeBlock,
  buildEmptyState,
  buildProgressBar,
  buildQuoteBlock,
  buildRadio,
  buildTimeline,
} from '../element-builders/index.js';

/**
 * Vertical / multi-row composites with layout invariants.
 * These tests guard structural rules the AI can't easily
 * recover from (layout=none bugs, connector collapse, fit_content
 * circular deps, pen-core-specific RING pattern).
 */

describe('buildEmptyState', () => {
  it('title-only → 1 child; with icon/subtitle/CTA → 4 children', () => {
    const minimal = buildEmptyState({ title: 'No items' }) as Record<string, unknown>;
    expect(minimal.children as unknown[]).toHaveLength(1);
    const full = buildEmptyState({
      title: 'No items',
      subtitle: 'Add one to get started',
      icon: 'inbox',
      cta_label: 'Create new',
    }) as Record<string, unknown>;
    expect(full.children as unknown[]).toHaveLength(4);
  });
  it('centered layout + padding [48,24]', () => {
    const e = buildEmptyState({ title: 'X' }) as Record<string, unknown>;
    expect(e.layout).toBe('vertical');
    expect(e.alignItems).toBe('center');
    expect(e.padding).toEqual([48, 24]);
  });
});

describe('buildAlert', () => {
  it('message only → single child', () => {
    const a = buildAlert({ message: 'Saved' }) as Record<string, unknown>;
    expect(a.children as unknown[]).toHaveLength(1);
  });
  it('icon + message + dismissible close → 3 children', () => {
    const a = buildAlert({
      message: 'Saved',
      icon: 'check',
      dismissible: true,
    }) as Record<string, unknown>;
    const children = a.children as Array<{ role?: string; iconFontName?: string }>;
    expect(children).toHaveLength(3);
    expect(children[0].iconFontName).toBe('check');
    expect(children[2].role).toBe('alert-close');
  });
  it('width=fill_container (banner stretches)', () => {
    const a = buildAlert({ message: 'X' }) as Record<string, unknown>;
    expect(a.width).toBe('fill_container');
  });
});

describe('buildCheckbox', () => {
  it('unchecked → empty fill + 1.5px stroke, no check icon', () => {
    const c = buildCheckbox({ label: 'Accept' }) as Record<string, unknown>;
    const [box] = c.children as Array<{
      fill?: unknown[];
      stroke?: { thickness: number };
      children?: unknown[];
    }>;
    expect(box.fill).toEqual([]);
    expect(box.stroke?.thickness).toBe(1.5);
    expect(box.children).toHaveLength(0);
  });
  it('checked → fill + check icon inside', () => {
    const c = buildCheckbox({ label: 'Accept', checked: true }) as Record<string, unknown>;
    const [box] = c.children as Array<{
      fill: Array<{ color: string }>;
      children: Array<{ iconFontName: string }>;
    }>;
    expect(box.fill[0].color).toBe('#2563EB');
    expect(box.children[0].iconFontName).toBe('check');
  });
});

describe('buildRadio', () => {
  it('unselected → no inner dot', () => {
    const r = buildRadio({ label: 'Small' }) as Record<string, unknown>;
    const [ring] = r.children as Array<{ children: unknown[] }>;
    expect(ring.children).toHaveLength(0);
  });
  it('selected → inner dot (10×10 cornerRadius 5)', () => {
    const r = buildRadio({ label: 'Medium', selected: true }) as Record<string, unknown>;
    const [ring] = r.children as Array<{
      children: Array<{ width: number; cornerRadius: number }>;
    }>;
    expect(ring.children).toHaveLength(1);
    expect(ring.children[0].width).toBe(10);
    expect(ring.children[0].cornerRadius).toBe(5);
  });
});

describe('buildActivityRing', () => {
  it('emits frame+cornerRadius (NOT ellipse) per layout.md §RING', () => {
    const r = buildActivityRing({ center_text: '42%' }) as Record<string, unknown>;
    expect(r.type).toBe('frame');
    expect(r.cornerRadius).toBe(40); // size/2 = 80/2
    // Centered layout
    expect(r.layout).toBe('horizontal');
    expect(r.alignItems).toBe('center');
    expect(r.justifyContent).toBe('center');
    // Stroke-only (fill empty)
    expect(r.fill as unknown[]).toEqual([]);
    const stroke = r.stroke as { thickness: number };
    expect(stroke.thickness).toBe(8);
    // Text child
    const [text] = r.children as Array<{ content: string }>;
    expect(text.content).toBe('42%');
  });
  it('custom size updates cornerRadius + stroke stays from param', () => {
    const r = buildActivityRing({
      center_text: '100',
      size: 120,
      thickness: 12,
    }) as Record<string, unknown>;
    expect(r.cornerRadius).toBe(60);
    expect((r.stroke as { thickness: number }).thickness).toBe(12);
  });
});

describe('buildProgressBar', () => {
  it('track width matches bar_width; fill width = value/100 * bar_width', () => {
    const p = buildProgressBar({ value: 60, bar_width: 200 }) as Record<string, unknown>;
    expect(p.width).toBe(200);
    const [fill] = p.children as Array<{ width: number; role: string }>;
    expect(fill.role).toBe('progress-bar-fill');
    expect(fill.width).toBe(120);
  });
  it('value=0 → NO fill child (cleaner than 0-width rect)', () => {
    const p = buildProgressBar({ value: 0 }) as Record<string, unknown>;
    expect(p.children as unknown[]).toHaveLength(0);
  });
  it('value clamped to [0, 100]', () => {
    const over = buildProgressBar({ value: 250, bar_width: 100 }) as Record<string, unknown>;
    const [fill] = over.children as Array<{ width: number }>;
    expect(fill.width).toBe(100); // 100/100 × 100
  });
});

describe('buildQuoteBlock', () => {
  it('quote only → 1 child', () => {
    const q = buildQuoteBlock({ quote: 'Stay hungry.' }) as Record<string, unknown>;
    expect(q.children as unknown[]).toHaveLength(1);
  });
  it('with author → 2 children; author prefixed "— "', () => {
    const q = buildQuoteBlock({ quote: 'Stay hungry.', author: 'SJ' }) as Record<string, unknown>;
    const [, author] = q.children as Array<{ content: string; role: string }>;
    expect(author.role).toBe('quote-author');
    expect(author.content).toBe('— SJ');
  });
  it('quote text has fill_container + fixed-width for wrap', () => {
    const q = buildQuoteBlock({ quote: 'Wrap me.' }) as Record<string, unknown>;
    const [quote] = q.children as Array<{ width: string; textGrowth: string }>;
    expect(quote.width).toBe('fill_container');
    expect(quote.textGrowth).toBe('fixed-width');
  });
});

describe('buildCodeBlock', () => {
  it('preserves newlines in code', () => {
    const c = buildCodeBlock({ code: 'a\nb\nc' }) as Record<string, unknown>;
    const [text] = c.children as Array<{ content: string }>;
    expect(text.content).toBe('a\nb\nc');
  });
  it('language appears in frame name only', () => {
    const c = buildCodeBlock({ code: 'x', language: 'typescript' }) as Record<string, unknown>;
    expect(c.name).toBe('Code Block (typescript)');
  });
  it('no language → plain name', () => {
    const c = buildCodeBlock({ code: 'x' }) as Record<string, unknown>;
    expect(c.name).toBe('Code Block');
  });
});

describe('buildTimeline', () => {
  it('empty items throws', () => {
    expect(() => buildTimeline({ items: [] })).toThrow(/at least one entry/);
  });
  it('3 items: first active with connector, last drops connector', () => {
    const t = buildTimeline({
      items: [
        { title: 'Order', subtitle: '10:00', active: true },
        { title: 'Ship' },
        { title: 'Deliver' },
      ],
    }) as Record<string, unknown>;
    const rows = t.children as Array<{ children: Array<{ children: unknown[] }> }>;
    expect(rows).toHaveLength(3);
    // Row 0: icon column has dot + connector (2 children)
    const row0IconCol = rows[0].children[0];
    expect(row0IconCol.children).toHaveLength(2);
    const dot0 = row0IconCol.children[0] as { role: string };
    expect(dot0.role).toBe('timeline-dot-active');
    // Last row: icon column has dot only (no connector)
    const row2IconCol = rows[2].children[0];
    expect(row2IconCol.children).toHaveLength(1);
  });
  it('icon col is fit_content (drives row height via dot+connector sum)', () => {
    const t = buildTimeline({ items: [{ title: 'X' }, { title: 'Y' }] }) as Record<string, unknown>;
    const rows = t.children as Array<{ children: Array<{ height: string }> }>;
    expect(rows[0].children[0].height).toBe('fit_content');
  });
});

describe('buildCalendarGrid', () => {
  it('default 30-day month Sun-start → 1 header row + 5 week rows', () => {
    const g = buildCalendarGrid({}) as Record<string, unknown>;
    const rows = g.children as Array<{ role: string; children: unknown[] }>;
    expect(rows).toHaveLength(6); // 1 header + 5 weeks
    expect(rows[0].role).toBe('calendar-header-row');
    expect(rows[0].children).toHaveLength(7);
  });
  it('start_day_offset=3 → first 3 cells blank, then day 1', () => {
    const g = buildCalendarGrid({ days_in_month: 30, start_day_offset: 3 }) as Record<
      string,
      unknown
    >;
    const rows = g.children as Array<{ children: unknown[] }>;
    const week1 = rows[1].children as Array<{
      role: string;
      children?: Array<{ content: string }>;
    }>;
    expect(week1[0].role).toBe('calendar-day-empty');
    expect(week1[1].role).toBe('calendar-day-empty');
    expect(week1[2].role).toBe('calendar-day-empty');
    expect(week1[3].role).toBe('calendar-day');
    expect(week1[3].children?.[0].content).toBe('1');
  });
  it('selected_day wins over today on overlap', () => {
    const g = buildCalendarGrid({
      days_in_month: 30,
      today: 15,
      selected_day: 15,
    }) as Record<string, unknown>;
    const rows = g.children as Array<{
      children: Array<{ role: string; fill?: Array<{ color: string }> }>;
    }>;
    // day 15 at offset 0 = index 14 = row 2 (zero-indexed), col 0; header at row 0, so actual index 3
    const day15 = rows[3].children[0];
    expect(day15.role).toBe('calendar-day-selected');
    expect(day15.fill?.[0].color).toBe('#2563EB');
  });
  it('days_in_month clamped to [1, 31]', () => {
    const g = buildCalendarGrid({ days_in_month: 100 }) as Record<string, unknown>;
    const rows = g.children as unknown[];
    // 31 + 0 = 31 → ceil(31/7) = 5 week rows + 1 header
    expect(rows).toHaveLength(6);
  });
});
