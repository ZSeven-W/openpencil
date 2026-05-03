import { describe, it, expect } from 'vitest';
import {
  buildAvatar,
  buildBadge,
  buildBodyText,
  buildColorSwatch,
  buildDivider,
  buildFab,
  buildHeading,
  buildIconButton,
  buildIconLabel,
  buildKbd,
  buildLink,
  buildPrice,
  buildSwitch,
  buildTextButton,
  buildToast,
} from '../element-builders/index.js';

/**
 * Atoms — single-node or 2-4 children primitives. Tests lock
 * shape + key invariants so the builder output can't silently
 * drift when someone rewrites internals.
 *
 * Common assertions: role, type, dimensions, layout metadata.
 * Skip color / font-weight unless the value encodes behavior
 * (e.g. CJK dispatch or active-state theming).
 */

describe('buildDivider', () => {
  it('horizontal default → rectangle fill_container × 1', () => {
    const d = buildDivider({}) as Record<string, unknown>;
    expect(d.type).toBe('rectangle');
    expect(d.role).toBe('divider');
    expect(d.width).toBe('fill_container');
    expect(d.height).toBe(1);
  });
  it('vertical → thickness × fill_container', () => {
    const d = buildDivider({ orientation: 'vertical', thickness: 2 }) as Record<string, unknown>;
    expect(d.width).toBe(2);
    expect(d.height).toBe('fill_container');
  });
});

describe('buildBadge', () => {
  it('pill shape (cornerRadius 999) + role="badge" + child label', () => {
    const b = buildBadge({ label: 'NEW' }) as Record<string, unknown>;
    expect(b.role).toBe('badge');
    expect(b.cornerRadius).toBe(999);
    const children = b.children as Array<{ content: string; role: string }>;
    expect(children).toHaveLength(1);
    expect(children[0].content).toBe('NEW');
    expect(children[0].role).toBe('label');
  });
});

describe('buildAvatar', () => {
  it('default size 40 → 40×40 cornerRadius 20, no child when no initial', () => {
    const a = buildAvatar({}) as Record<string, unknown>;
    expect(a.width).toBe(40);
    expect(a.cornerRadius).toBe(20);
    expect(a.children as unknown[]).toHaveLength(0);
  });
  it('with initial scales font size to 40% of frame', () => {
    const a = buildAvatar({ initial: 'JD', size: 80 }) as Record<string, unknown>;
    expect(a.width).toBe(80);
    expect(a.cornerRadius).toBe(40);
    const child = (a.children as Array<{ fontSize: number; content: string }>)[0];
    expect(child.content).toBe('JD');
    expect(child.fontSize).toBe(32); // 80 * 0.4
  });
});

describe('buildIconButton', () => {
  it('default 44×44 hit target with centered 24 icon', () => {
    const b = buildIconButton({ icon: 'x' }) as Record<string, unknown>;
    expect(b.width).toBe(44);
    expect(b.height).toBe(44);
    expect(b.justifyContent).toBe('center');
    expect(b.alignItems).toBe('center');
    const child = (b.children as Array<{ iconFontName: string; width: number }>)[0];
    expect(child.iconFontName).toBe('x');
    expect(child.width).toBe(24);
  });
});

describe('buildIconLabel', () => {
  it('icon leads label with alignItems=center + fit_content', () => {
    const n = buildIconLabel({ icon: 'info', label: 'Learn more' }) as Record<string, unknown>;
    expect(n.role).toBe('icon-label');
    expect(n.width).toBe('fit_content');
    const [icon, label] = n.children as Array<{
      iconFontName?: string;
      content?: string;
      role?: string;
    }>;
    expect(icon.iconFontName).toBe('info');
    expect(label.content).toBe('Learn more');
  });
});

describe('buildLink', () => {
  it('text-only when no trailing_icon', () => {
    const l = buildLink({ label: 'Learn more' }) as Record<string, unknown>;
    expect(l.children as unknown[]).toHaveLength(1);
  });
  it('trailing icon appended when provided', () => {
    const l = buildLink({ label: 'Learn more', trailing_icon: 'arrow-right' }) as Record<
      string,
      unknown
    >;
    const children = l.children as Array<{ iconFontName?: string; role?: string }>;
    expect(children).toHaveLength(2);
    expect(children[1].role).toBe('link-icon');
    expect(children[1].iconFontName).toBe('arrow-right');
  });
});

describe('buildKbd', () => {
  it('single key → one cell + no separator', () => {
    const k = buildKbd({ keys: ['⌘'] }) as Record<string, unknown>;
    expect(k.children as unknown[]).toHaveLength(1);
  });
  it('three keys joined by default + separator', () => {
    const k = buildKbd({ keys: ['Ctrl', 'Shift', 'P'] }) as Record<string, unknown>;
    const children = k.children as Array<{ content?: string; role: string }>;
    expect(children).toHaveLength(5); // 3 keys + 2 separators
    expect(children[1].role).toBe('kbd-separator');
    expect(children[1].content).toBe('+');
  });
  it('explicit empty separator → no separator text between keys', () => {
    const k = buildKbd({ keys: ['⌘', 'K'], separator: '' }) as Record<string, unknown>;
    expect(k.children as unknown[]).toHaveLength(2);
  });
  it('empty keys throws', () => {
    expect(() => buildKbd({ keys: [] })).toThrow(/at least one non-empty key/);
  });
});

describe('buildPrice', () => {
  it('default currency "$" + no period → 2 children', () => {
    const p = buildPrice({ amount: '29' }) as Record<string, unknown>;
    const children = p.children as Array<{ content: string; role: string }>;
    expect(children).toHaveLength(2);
    expect(children[0].content).toBe('$');
    expect(children[1].content).toBe('29');
  });
  it('currency + amount + period → 3 children', () => {
    const p = buildPrice({ amount: '29', period: '/month' }) as Record<string, unknown>;
    const children = p.children as Array<{ content: string }>;
    expect(children).toHaveLength(3);
    expect(children[2].content).toBe('/month');
  });
});

describe('buildColorSwatch', () => {
  it('accepts hex — emits fill via solid color on square', () => {
    const s = buildColorSwatch({ color: '#2563EB' }) as Record<string, unknown>;
    const [square] = s.children as Array<{ fill: Array<{ color: string }> }>;
    expect(square.fill[0].color).toBe('#2563EB');
  });
  it('accepts $variable ref — passes through to fill color', () => {
    const s = buildColorSwatch({ color: '$color-primary' }) as Record<string, unknown>;
    const [square] = s.children as Array<{ fill: Array<{ color: string }> }>;
    expect(square.fill[0].color).toBe('$color-primary');
  });
  it('label appends a second child', () => {
    const s = buildColorSwatch({ color: '#fff', label: 'Primary' }) as Record<string, unknown>;
    expect(s.children as unknown[]).toHaveLength(2);
  });
});

describe('buildFab', () => {
  it('default 56×56 circle with icon at ~43%', () => {
    const f = buildFab({ icon: 'plus' }) as Record<string, unknown>;
    expect(f.width).toBe(56);
    expect(f.cornerRadius).toBe(28);
    const icon = (f.children as Array<{ width: number; iconFontName: string }>)[0];
    expect(icon.iconFontName).toBe('plus');
    expect(icon.width).toBe(24); // round(56 * 0.43)
  });
});

describe('buildToast', () => {
  it('fit_content pill with dark fill + white text', () => {
    const t = buildToast({ message: 'Copied' }) as Record<string, unknown>;
    expect(t.width).toBe('fit_content');
    expect(t.cornerRadius).toBe(24);
    const [text] = t.children as Array<{ content: string; fill: Array<{ color: string }> }>;
    expect(text.content).toBe('Copied');
    expect(text.fill[0].color).toBe('#FFFFFF');
  });
});

describe('buildHeading', () => {
  it('Latin h2 default → Space Grotesk-compatible preset', () => {
    const h = buildHeading({ content: 'Welcome' }) as Record<string, unknown>;
    expect(h.role).toBe('heading');
    expect(h.fontSize).toBe(24);
    expect(h.fontWeight).toBe(600);
    expect(h.lineHeight).toBe(1.2);
    // Latin: no fontFamily dispatch
    expect((h as { fontFamily?: string }).fontFamily).toBeUndefined();
  });
  it('Chinese heading → Noto Sans SC + CJK lineHeight 1.35', () => {
    const h = buildHeading({ content: '你好世界', level: 'h2' }) as Record<string, unknown>;
    expect(h.fontFamily).toBe('Noto Sans SC');
    expect(h.lineHeight).toBe(1.35);
    expect((h as { letterSpacing?: number }).letterSpacing).toBeUndefined();
  });
  it('Japanese → Noto Sans JP (hiragana trigger)', () => {
    const h = buildHeading({ content: 'こんにちは' }) as Record<string, unknown>;
    expect(h.fontFamily).toBe('Noto Sans JP');
  });
  it('Korean → Noto Sans KR', () => {
    const h = buildHeading({ content: '안녕하세요' }) as Record<string, unknown>;
    expect(h.fontFamily).toBe('Noto Sans KR');
  });
  it('Latin display has negative letterSpacing, CJK display does not', () => {
    const latin = buildHeading({ content: 'Hero', level: 'display' }) as Record<string, unknown>;
    expect(latin.letterSpacing).toBe(-0.5);
    const cjk = buildHeading({ content: '标题', level: 'display' }) as Record<string, unknown>;
    expect((cjk as { letterSpacing?: number }).letterSpacing).toBeUndefined();
  });
});

describe('buildBodyText', () => {
  it('Latin → Inter + lineHeight 1.5 + no letterSpacing', () => {
    const b = buildBodyText({ content: 'Lorem ipsum dolor sit.' }) as Record<string, unknown>;
    expect(b.fontFamily).toBe('Inter');
    expect(b.lineHeight).toBe(1.5);
    expect(b.width).toBe('fill_container');
    expect(b.textGrowth).toBe('fixed-width');
    expect((b as { letterSpacing?: number }).letterSpacing).toBeUndefined();
  });
  it('CJK → Inter (body stays Inter!) + lineHeight 1.6 + letterSpacing 0', () => {
    const b = buildBodyText({ content: '你好，这是正文。' }) as Record<string, unknown>;
    expect(b.fontFamily).toBe('Inter'); // body ALWAYS Inter — only heading dispatches
    expect(b.lineHeight).toBe(1.6);
    expect(b.letterSpacing).toBe(0);
  });
});

describe('buildTextButton', () => {
  it('no leading icon → single text child', () => {
    const b = buildTextButton({ label: 'Submit' }) as Record<string, unknown>;
    expect(b.children as unknown[]).toHaveLength(1);
  });
  it('leading icon → icon + text', () => {
    const b = buildTextButton({ label: 'Add', leading_icon: 'plus' }) as Record<string, unknown>;
    const [icon, text] = b.children as Array<{ iconFontName?: string; content?: string }>;
    expect(icon.iconFontName).toBe('plus');
    expect(text.content).toBe('Add');
  });
});

describe('buildSwitch', () => {
  it('off → gray track + thumb at flex-start', () => {
    const s = buildSwitch({}) as Record<string, unknown>;
    expect(s.justifyContent).toBe('flex-start');
    const fill = s.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#E5E5EA');
  });
  it('active → iOS green + thumb at flex-end', () => {
    const s = buildSwitch({ active: true }) as Record<string, unknown>;
    expect(s.justifyContent).toBe('flex-end');
    const fill = s.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#34C759');
  });
});
