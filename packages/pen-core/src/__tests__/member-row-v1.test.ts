import { describe, it, expect } from 'vitest';
import { buildMemberRow } from '../element-builders/member-row.js';
import { buildMemberRowV1 } from '../element-builders/member-row-v1.js';

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

const BASIC = { name: 'Sarah Lee', subtitle: 'sarah@acme.com', initial: 'SL' };
const WITH_BADGE = {
  ...BASIC,
  trailing: { kind: 'role_badge' as const, value: 'Owner' },
};
const WITH_MENU = { ...BASIC, trailing: { kind: 'menu' as const } };
const WITH_STATUS = {
  ...BASIC,
  trailing: { kind: 'status_dot' as const, tone: 'online' as const },
};

describe('buildMemberRowV1 — byte-parity with v0 (light)', () => {
  it('layout/gap/padding/name identical to v0', () => {
    const v0 = buildMemberRow(BASIC) as Record<string, unknown>;
    const v1 = buildMemberRowV1(BASIC) as Record<string, unknown>;
    expect(v1.layout).toBe(v0.layout);
    expect(v1.gap).toBe(v0.gap);
    expect(JSON.stringify(v1.padding)).toBe(JSON.stringify(v0.padding));
    expect(v1.name).toBe(v0.name);
  });

  it('avatar: size=40, cornerRadius=20, fill=default blue, initial #FFFFFF — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(BASIC) as Record<string, unknown>);
    const avatar = getChildren(v1)[0];
    expect(avatar.width).toBe(40);
    expect(avatar.cornerRadius).toBe(20);
    expect(getFill(avatar)).toBe('#3B82F6');
    const initial = getChildren(avatar)[0];
    expect(getFill(initial)).toBe('#FFFFFF');
  });

  it('name: fontSize=15, fontWeight=500, fill=#0F172A — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(BASIC) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const name = getChildren(textStack)[0];
    expect(name.fontSize).toBe(15);
    expect(name.fontWeight).toBe(500);
    expect(getFill(name)).toBe('#0F172A');
  });

  it('subtitle fill = #64748B (SUBTITLE_FG) — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(BASIC) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('#64748B');
  });

  it('role badge: bg=#F1F5F9, fg=#334155 — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(WITH_BADGE) as Record<string, unknown>);
    const children = getChildren(v1);
    const badge = children[children.length - 1];
    expect(getFill(badge)).toBe('#F1F5F9');
    const text = getChildren(badge)[0];
    expect(getFill(text)).toBe('#334155');
  });

  it('menu icon fill = #94A3B8 — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(WITH_MENU) as Record<string, unknown>);
    const children = getChildren(v1);
    const menu = children[children.length - 1];
    expect(getFill(menu)).toBe('#94A3B8');
  });

  it('status dot online = #10B981 — v0 parity', () => {
    const v1 = getRoot(buildMemberRowV1(WITH_STATUS) as Record<string, unknown>);
    const children = getChildren(v1);
    const dot = children[children.length - 1];
    expect(getFill(dot)).toBe('#10B981');
  });

  it('throws on invalid status tone', () => {
    expect(() =>
      buildMemberRowV1({
        name: 'X',
        trailing: { kind: 'status_dot' as const, tone: 'ghost' as never },
      }),
    ).toThrow(/invalid trailing\.tone/);
  });
});

describe('buildMemberRowV1 — dark mode', () => {
  it('name fill = #F1F5F9 (dark textPrimary)', () => {
    const v1 = getRoot(buildMemberRowV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const name = getChildren(textStack)[0];
    expect(getFill(name)).toBe('#F1F5F9');
  });

  it('subtitle fill = #94A3B8 (dark textMuted)', () => {
    const v1 = getRoot(buildMemberRowV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('#94A3B8');
  });

  it('role badge bg = #334155 (dark surface2)', () => {
    const v1 = getRoot(
      buildMemberRowV1({ ...WITH_BADGE, theme: 'dark' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const badge = children[children.length - 1];
    expect(getFill(badge)).toBe('#334155');
  });
});

describe('buildMemberRowV1 — system mode emits refs', () => {
  it('name fill = $color-text-primary', () => {
    const v1 = getRoot(buildMemberRowV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const name = getChildren(textStack)[0];
    expect(getFill(name)).toBe('$color-text-primary');
  });

  it('subtitle fill = $color-text-muted', () => {
    const v1 = getRoot(buildMemberRowV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>);
    const textStack = getChildren(v1)[1];
    const subtitle = getChildren(textStack)[1];
    expect(getFill(subtitle)).toBe('$color-text-muted');
  });

  it('role badge bg = $color-surface-2', () => {
    const v1 = getRoot(
      buildMemberRowV1({ ...WITH_BADGE, theme: 'system' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const badge = children[children.length - 1];
    expect(getFill(badge)).toBe('$color-surface-2');
  });

  it('menu icon fill = $color-text-subtle', () => {
    const v1 = getRoot(
      buildMemberRowV1({ ...WITH_MENU, theme: 'system' }) as Record<string, unknown>,
    );
    const children = getChildren(v1);
    const menu = children[children.length - 1];
    expect(getFill(menu)).toBe('$color-text-subtle');
  });
});
