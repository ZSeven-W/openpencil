import { describe, it, expect } from 'vitest';
import { buildSettingRow } from '../element-builders/setting-row.js';
import { buildSettingRowV1 } from '../element-builders/setting-row-v1.js';

function getRoot(tree: Record<string, unknown>): Record<string, unknown> {
  return tree as Record<string, unknown>;
}
function getChildren(node: Record<string, unknown>): Record<string, unknown>[] {
  return node.children as Record<string, unknown>[];
}
function getFill(node: Record<string, unknown>): string | undefined {
  const fill = node.fill as Array<{ color: string }> | undefined;
  return fill?.[0]?.color;
}

const BASIC = { title: 'Notifications', subtitle: 'Push, email, in-app', leading_icon: 'bell' };
const WITH_SWITCH = { ...BASIC, trailing: { kind: 'switch' as const, on: true } };
const WITH_BADGE = { title: 'Plan', trailing: { kind: 'badge' as const, value: 'Pro' } };
const WITH_VALUE = { title: 'Language', trailing: { kind: 'value' as const, value: 'English' } };

describe('buildSettingRowV1 — byte-parity with v0 (light)', () => {
  it('layout/gap/padding identical to v0', () => {
    const v0 = buildSettingRow(BASIC) as Record<string, unknown>;
    const v1 = buildSettingRowV1(BASIC) as Record<string, unknown>;
    expect(v1.layout).toBe(v0.layout);
    expect(v1.gap).toBe(v0.gap);
    expect(JSON.stringify(v1.padding)).toBe(JSON.stringify(v0.padding));
  });

  it('title: fontSize=15, fontWeight=500, fill=#0F172A — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(BASIC) as Record<string, unknown>);
    // Text stack is at index 1 (after leading icon)
    const textStack = getChildren(v1)[1];
    const title = getChildren(textStack)[0];
    expect(title.fontSize).toBe(15);
    expect(title.fontWeight).toBe(500);
    expect(getFill(title)).toBe('#0F172A');
  });

  it('subtitle fill = #64748B (MUTED) — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(BASIC) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('#64748B');
  });

  it('leading icon fill = #0F172A — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(BASIC) as Record<string, unknown>);
    const icon = getChildren(v1)[0];
    expect(getFill(icon)).toBe('#0F172A');
  });

  it('switch on=true: fill = #2563EB (accent) — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(WITH_SWITCH) as Record<string, unknown>);
    const children = getChildren(v1);
    const sw = children[children.length - 1];
    expect(getFill(sw)).toBe('#2563EB');
  });

  it('switch knob always #FFFFFF — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(WITH_SWITCH) as Record<string, unknown>);
    const children = getChildren(v1);
    const sw = children[children.length - 1];
    const knob = getChildren(sw)[0];
    expect(getFill(knob)).toBe('#FFFFFF');
  });

  it('badge bg=#DBEAFE, fg=#1D4ED8 in light — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(WITH_BADGE) as Record<string, unknown>);
    const children = getChildren(v1);
    const badge = children[children.length - 1];
    expect(getFill(badge)).toBe('#DBEAFE');
    const text = getChildren(badge)[0];
    expect(getFill(text)).toBe('#1D4ED8');
  });

  it('chevron fill = #64748B (MUTED) — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1({ title: 'Settings' }) as Record<string, unknown>);
    const children = getChildren(v1);
    const chevron = children[children.length - 1];
    expect(getFill(chevron)).toBe('#64748B');
  });

  it('value trailing fill = #64748B (MUTED) — v0 parity', () => {
    const v1 = getRoot(buildSettingRowV1(WITH_VALUE) as Record<string, unknown>);
    const children = getChildren(v1);
    const val = children[children.length - 1];
    expect(getFill(val)).toBe('#64748B');
  });
});

describe('buildSettingRowV1 — dark mode', () => {
  it('title fill = #F1F5F9 (dark textPrimary)', () => {
    const v1 = getRoot(buildSettingRowV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const title = getChildren(textStack)[0];
    expect(getFill(title)).toBe('#F1F5F9');
  });

  it('subtitle fill = #94A3B8 (dark textMuted)', () => {
    const v1 = getRoot(buildSettingRowV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('#94A3B8');
  });

  it('switch on=true: fill = #60A5FA (dark accent)', () => {
    const v1 = getRoot(
      buildSettingRowV1({ ...WITH_SWITCH, theme: 'dark' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const sw = children[children.length - 1];
    expect(getFill(sw)).toBe('#60A5FA');
  });
});

describe('buildSettingRowV1 — system mode emits refs', () => {
  it('title fill = $color-text-primary', () => {
    const v1 = getRoot(buildSettingRowV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const title = getChildren(textStack)[0];
    expect(getFill(title)).toBe('$color-text-primary');
  });

  it('subtitle fill = $color-text-muted', () => {
    const v1 = getRoot(buildSettingRowV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('$color-text-muted');
  });

  it('switch on=true: fill = $color-accent', () => {
    const v1 = getRoot(
      buildSettingRowV1({ ...WITH_SWITCH, theme: 'system' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const sw = children[children.length - 1];
    expect(getFill(sw)).toBe('$color-accent');
  });

  it('badge bg=$color-info-bg in system', () => {
    const v1 = getRoot(
      buildSettingRowV1({ ...WITH_BADGE, theme: 'system' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const badge = children[children.length - 1];
    expect(getFill(badge)).toBe('$color-info-bg');
  });
});
