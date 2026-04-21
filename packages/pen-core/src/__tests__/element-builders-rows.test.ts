import { describe, it, expect } from 'vitest';
import {
  buildBottomNav,
  buildBreadcrumb,
  buildCardRow,
  buildCarouselDots,
  buildChartBars,
  buildFormField,
  buildListRow,
  buildMetricRow,
  buildNavChipRow,
  buildRatingStars,
  buildSearchBar,
  buildSectionHeader,
  buildSegmentedControl,
  buildStatGrid,
  buildStepper,
  buildTabs,
  buildTopNavBar,
} from '../element-builders/index.js';

/**
 * Row-shaped builders — horizontal stacks of items or fixed multi-
 * child structures. Tests lock item-count math + specific item
 * invariants (active state, fill_container vs fit_content).
 */

describe('buildCardRow', () => {
  it('wrapper + inner row + one card per item', () => {
    const t = buildCardRow({
      items: [{ title: 'A' }, { title: 'B' }, { title: 'C' }],
    }) as Record<string, unknown>;
    expect(t.role).toBe('scroll-row-wrapper');
    const inner = (t.children as Array<{ role: string; children: unknown[] }>)[0];
    expect(inner.role).toBe('scroll-row');
    expect(inner.children).toHaveLength(3);
  });
  it('card width override propagates', () => {
    const t = buildCardRow({
      items: [{ title: 'A' }],
      card_width: 200,
    }) as Record<string, unknown>;
    const inner = (t.children as Array<{ children: Array<{ width: number }> }>)[0];
    expect(inner.children[0].width).toBe(200);
  });
});

describe('buildMetricRow', () => {
  it('tiles get metric-tile role + value/label stack', () => {
    const t = buildMetricRow({
      items: [{ label: 'Steps', value: '8,432', icon: 'activity' }],
    }) as Record<string, unknown>;
    const inner = (t.children as Array<{ children: Array<{ role: string }> }>)[0];
    expect(inner.children[0].role).toBe('metric-tile');
  });
});

describe('buildNavChipRow', () => {
  it('active chip gets nav-chip-active role + bolder weight', () => {
    const t = buildNavChipRow({
      items: [{ label: 'All', active: true }, { label: 'Videos' }],
    }) as Record<string, unknown>;
    const inner = (t.children as Array<{ children: Array<{ role: string }> }>)[0];
    expect(inner.children[0].role).toBe('nav-chip-active');
    expect(inner.children[1].role).toBe('nav-chip');
  });
});

describe('buildBottomNav', () => {
  it('3 tabs + space_around distribution', () => {
    const t = buildBottomNav({
      items: [
        { title: 'Home', icon: 'home', active: true },
        { title: 'Search', icon: 'search' },
        { title: 'Profile', icon: 'user' },
      ],
    }) as Record<string, unknown>;
    expect(t.role).toBe('bottom-tab-bar');
    expect(t.justifyContent).toBe('space_around');
    expect(t.children as unknown[]).toHaveLength(3);
    const [first] = t.children as Array<{ role: string }>;
    expect(first.role).toBe('nav-item-active');
  });
});

describe('buildSectionHeader', () => {
  it('title only → 1 child wrapper', () => {
    const h = buildSectionHeader({ title: 'Recent' }) as Record<string, unknown>;
    expect(h.children as unknown[]).toHaveLength(1);
  });
  it('title + action → 2 children (title wrapper + action)', () => {
    const h = buildSectionHeader({
      title: 'Recent',
      action: { label: 'See all', icon: 'arrow-right' },
    }) as Record<string, unknown>;
    expect(h.children as unknown[]).toHaveLength(2);
    // Action group has icon after label
    const action = (h.children as Array<{ children?: unknown[]; role?: string }>)[1];
    expect(action.role).toBe('section-header-action');
    expect(action.children as unknown[]).toHaveLength(2); // label + icon
  });
});

describe('buildTopNavBar', () => {
  it('no leading icon → leading spacer (44×44) so title stays centered', () => {
    const bar = buildTopNavBar({ title: 'Settings' }) as Record<string, unknown>;
    const [leading] = bar.children as Array<{ role: string; width: number }>;
    expect(leading.role).toBe('nav-spacer');
    expect(leading.width).toBe(44);
  });
  it('leading icon fills the 44×44 slot as icon-button', () => {
    const bar = buildTopNavBar({
      title: 'Settings',
      leading_icon: 'chevron-left',
    }) as Record<string, unknown>;
    const [leading] = bar.children as Array<{ role: string }>;
    expect(leading.role).toBe('icon-button');
  });
});

describe('buildStatGrid', () => {
  it('every cell width=fill_container (critical: no overflow)', () => {
    const g = buildStatGrid({
      items: [
        { value: '1', label: 'A' },
        { value: '2', label: 'B' },
        { value: '3', label: 'C' },
      ],
    }) as Record<string, unknown>;
    const cells = g.children as Array<{ width: string }>;
    expect(cells).toHaveLength(3);
    cells.forEach((c) => expect(c.width).toBe('fill_container'));
  });
});

describe('buildTabs', () => {
  it('active tab gets underline sibling + tab-active role', () => {
    const t = buildTabs({
      items: [{ label: 'A', active: true }, { label: 'B' }, { label: 'C' }],
    }) as Record<string, unknown>;
    const tabs = t.children as Array<{ role: string; children: unknown[] }>;
    expect(tabs[0].role).toBe('tab-active');
    expect(tabs[0].children).toHaveLength(2); // inner + underline
    expect(tabs[1].children).toHaveLength(1); // inner only, no underline
  });
  it('every tab width=fill_container so bar splits evenly', () => {
    const t = buildTabs({ items: [{ label: 'A' }, { label: 'B' }] }) as Record<string, unknown>;
    const tabs = t.children as Array<{ width: string }>;
    tabs.forEach((tab) => expect(tab.width).toBe('fill_container'));
  });
});

describe('buildSegmentedControl', () => {
  it('active segment gets white fill; inactive stays transparent', () => {
    const s = buildSegmentedControl({
      items: [{ label: 'Day' }, { label: 'Week', active: true }, { label: 'Month' }],
    }) as Record<string, unknown>;
    const segs = s.children as Array<{ fill: Array<{ color: string }> }>;
    expect(segs[1].fill[0].color).toBe('#FFFFFF');
    expect(segs[0].fill).toHaveLength(0);
    expect(segs[2].fill).toHaveLength(0);
  });
  it('container 32px high + gray-100 background', () => {
    const s = buildSegmentedControl({ items: [{ label: 'A' }] }) as Record<string, unknown>;
    expect(s.height).toBe(32);
    const fill = s.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#F3F4F6');
  });
});

describe('buildBreadcrumb', () => {
  it('last item auto-active; N items → 2N-1 children (N items + N-1 separators)', () => {
    const b = buildBreadcrumb({
      items: [{ label: 'Home' }, { label: 'Settings' }, { label: 'Billing' }],
    }) as Record<string, unknown>;
    const children = b.children as Array<{ role: string; content?: string; fontWeight?: number }>;
    expect(children).toHaveLength(5); // 3 items + 2 separators
    expect(children[4].role).toBe('breadcrumb-item-active');
    expect(children[4].fontWeight).toBe(600);
    // Separators interleave at odd indices (1, 3)
    expect(children[1].role).toBe('breadcrumb-separator');
    expect(children[3].role).toBe('breadcrumb-separator');
  });
  it('explicit active overrides last-wins default', () => {
    const b = buildBreadcrumb({
      items: [{ label: 'A', active: true }, { label: 'B' }, { label: 'C' }],
    }) as Record<string, unknown>;
    const children = b.children as Array<{ role: string }>;
    expect(children[0].role).toBe('breadcrumb-item-active');
    expect(children[4].role).toBe('breadcrumb-item-active'); // last also active
  });
});

describe('buildStepper', () => {
  it('3 steps, current=1 → steps 0+1 active, connector 0 active, step 2 + connector 1 pending', () => {
    const s = buildStepper({ total: 3, current: 1 }) as Record<string, unknown>;
    const children = s.children as Array<{ role: string; content?: string }>;
    expect(children).toHaveLength(5); // 3 steps + 2 connectors
    expect(children[0].role).toBe('step-active');
    expect(children[1].role).toBe('step-connector-active');
    expect(children[2].role).toBe('step-active');
    expect(children[3].role).toBe('step-connector');
    expect(children[4].role).toBe('step');
  });
  it('total=1 → single step, no connector', () => {
    const s = buildStepper({ total: 1 }) as Record<string, unknown>;
    expect(s.children as unknown[]).toHaveLength(1);
  });
  it('current clamped to total-1', () => {
    const s = buildStepper({ total: 3, current: 99 }) as Record<string, unknown>;
    const kids = s.children as Array<{ role: string }>;
    // All active
    expect(kids.filter((k) => k.role === 'step-active')).toHaveLength(3);
  });
});

describe('buildRatingStars', () => {
  it('4 of 5 filled + 1 empty', () => {
    const r = buildRatingStars({ filled: 4 }) as Record<string, unknown>;
    const stars = r.children as Array<{ role: string; iconFontName: string }>;
    expect(stars).toHaveLength(5);
    expect(stars.filter((s) => s.role === 'star-filled')).toHaveLength(4);
    expect(stars[4].role).toBe('star-empty');
    stars.forEach((s) => expect(s.iconFontName).toBe('star'));
  });
  it('filled clamped to [0, total]', () => {
    const over = buildRatingStars({ filled: 10, total: 3 }) as Record<string, unknown>;
    expect((over.children as Array<{ role: string }>).every((s) => s.role === 'star-filled')).toBe(
      true,
    );
  });
});

describe('buildCarouselDots', () => {
  it('active dot is 16×6 pill; inactive are 6×6', () => {
    const d = buildCarouselDots({ total: 4, current: 1 }) as Record<string, unknown>;
    const dots = d.children as Array<{ width: number; role: string }>;
    expect(dots).toHaveLength(4);
    expect(dots[1].role).toBe('dot-active');
    expect(dots[1].width).toBe(16);
    expect(dots[0].role).toBe('dot');
    expect(dots[0].width).toBe(6);
  });
});

describe('buildListRow', () => {
  it('text stack is a vertical wrapper (critical for multi-line title wrap)', () => {
    const r = buildListRow({ title: 'Notifications', subtitle: 'Push, email' }) as Record<
      string,
      unknown
    >;
    const children = r.children as Array<{ role?: string; layout?: string }>;
    const textStack = children.find((c) => c.role === 'list-row-text');
    expect(textStack).toBeDefined();
    expect(textStack?.layout).toBe('vertical');
  });
  it('leading + trailing icons optional', () => {
    const r = buildListRow({
      title: 'X',
      leading_icon: 'bell',
      trailing_icon: 'chevron-right',
    }) as Record<string, unknown>;
    expect(r.children as unknown[]).toHaveLength(3); // leading + text stack + trailing
    const minimal = buildListRow({ title: 'X' }) as Record<string, unknown>;
    expect(minimal.children as unknown[]).toHaveLength(1); // just text stack
  });
});

describe('buildSearchBar', () => {
  it('44×44 pill (cornerRadius 22) + default search icon + placeholder', () => {
    const s = buildSearchBar({}) as Record<string, unknown>;
    expect(s.height).toBe(44);
    expect(s.cornerRadius).toBe(22);
    const [icon, placeholder] = s.children as Array<{
      iconFontName?: string;
      content?: string;
    }>;
    expect(icon.iconFontName).toBe('search');
    expect(placeholder.content).toBe('Search...');
  });
});

describe('buildFormField', () => {
  it('required=true appends "*" to label', () => {
    const f = buildFormField({ label: 'Email', required: true }) as Record<string, unknown>;
    const [label] = f.children as Array<{ content: string }>;
    expect(label.content).toBe('Email *');
  });
  it('input is width=fill_container + height=48 + cornerRadius=8', () => {
    const f = buildFormField({ label: 'X' }) as Record<string, unknown>;
    const [, input] = f.children as Array<{
      width?: string;
      height?: number;
      cornerRadius?: number;
    }>;
    expect(input.width).toBe('fill_container');
    expect(input.height).toBe(48);
    expect(input.cornerRadius).toBe(8);
  });
});

describe('buildChartBars', () => {
  it('heights scale to max(values); zero gets 2px floor', () => {
    const c = buildChartBars({ values: [1, 2, 4, 0], chart_height: 100 }) as Record<
      string,
      unknown
    >;
    const bars = c.children as Array<{ height: number }>;
    expect(bars).toHaveLength(4);
    expect(bars[2].height).toBe(100); // max
    expect(bars[1].height).toBe(50); // half
    expect(bars[3].height).toBe(2); // floor
  });
  it('negative + non-finite clamp to 0', () => {
    const c = buildChartBars({
      values: [-5, Number.NaN, 10],
      chart_height: 100,
    }) as Record<string, unknown>;
    const bars = c.children as Array<{ height: number }>;
    expect(bars[0].height).toBe(2);
    expect(bars[1].height).toBe(2);
    expect(bars[2].height).toBe(100);
  });
  it('empty values throws', () => {
    expect(() => buildChartBars({ values: [] })).toThrow(/at least one/);
  });
});
