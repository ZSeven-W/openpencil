import { describe, it, expect } from 'vitest';
import { clearCoerceWarnings, getCoerceWarnings } from '../element-builders/coerce-params.js';
import { buildActivityLog } from '../element-builders/activity-log.js';
import { buildActivityLogV1 } from '../element-builders/activity-log-v1.js';

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

const BASIC = {
  actor: 'Sarah Lee',
  action: 'merged pull request #142',
  timestamp: '2h ago',
  icon: 'git-merge',
  tone: 'success' as const,
};
const NO_ICON = { actor: 'Bob', action: 'commented', timestamp: '1h ago' };

describe('buildActivityLogV1 — byte-parity with v0 (light)', () => {
  it('root layout/gap/padding/name identical to v0', () => {
    const v0 = buildActivityLog(BASIC) as Record<string, unknown>;
    const v1 = buildActivityLogV1(BASIC) as Record<string, unknown>;
    expect(v1.layout).toBe(v0.layout);
    expect(v1.gap).toBe(v0.gap);
    expect(JSON.stringify(v1.padding)).toBe(JSON.stringify(v0.padding));
    expect(v1.name).toBe(v0.name);
  });

  it('icon dot bg = #DCFCE7 (success light bg) — v0 parity', () => {
    const v1 = getRoot(buildActivityLogV1(BASIC) as Record<string, unknown>);
    const dot = getChildren(v1)[0];
    expect(getFill(dot)).toBe('#DCFCE7');
  });

  it('icon fg = #166534 (success light fg) — v0 parity', () => {
    const v1 = getRoot(buildActivityLogV1(BASIC) as Record<string, unknown>);
    const dot = getChildren(v1)[0];
    const icon = getChildren(dot)[0];
    expect(getFill(icon)).toBe('#166534');
  });

  it('body line fill = #475569 (ACTION_FG) — v0 parity', () => {
    const v1 = getRoot(buildActivityLogV1(BASIC) as Record<string, unknown>);
    const body = getChildren(v1)[1];
    const line = getChildren(body)[0];
    expect(getFill(line)).toBe('#475569');
  });

  it('timestamp fill = #94A3B8 (TS_FG) — v0 parity', () => {
    const v1 = getRoot(buildActivityLogV1(BASIC) as Record<string, unknown>);
    const ts = getChildren(v1)[2];
    expect(getFill(ts)).toBe('#94A3B8');
  });

  it('actor segment fill = #0F172A (ACTOR_FG) — v0 parity', () => {
    const v1 = getRoot(buildActivityLogV1(BASIC) as Record<string, unknown>);
    const body = getChildren(v1)[1];
    const line = getChildren(body)[0];
    const content = line.content as Array<{ fill: string }>;
    expect(content[0].fill).toBe('#0F172A');
  });

  it('coerces invalid tone to default info and emits warning', () => {
    clearCoerceWarnings();
    const out = buildActivityLogV1({ ...BASIC, tone: 'critical' as never });
    expect(out).toBeDefined();
    const warnings = getCoerceWarnings();
    expect(warnings.length).toBeGreaterThan(0);
    expect(warnings[0].builder).toBe('buildActivityLogV1');
    expect(warnings[0].param).toBe('tone');
    expect(warnings[0].given).toBe('critical');
  });

  it('no icon: 3 children (no icon dot)', () => {
    const v1 = getRoot(buildActivityLogV1(NO_ICON) as Record<string, unknown>);
    // body + timestamp
    expect(getChildren(v1).length).toBe(2);
  });
});

describe('buildActivityLogV1 — dark mode', () => {
  it('body line fill = $color-text-body dark', () => {
    const v1 = getRoot(buildActivityLogV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const body = getChildren(v1)[1];
    const line = getChildren(body)[0];
    // dark textBody = #CBD5E1
    expect(getFill(line)).toBe('#CBD5E1');
  });

  it('timestamp fill = dark textSubtle (#64748B)', () => {
    const v1 = getRoot(buildActivityLogV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const ts = getChildren(v1)[2];
    expect(getFill(ts)).toBe('#64748B');
  });

  it('success icon dot bg = dark palette successBg (#14532D)', () => {
    const v1 = getRoot(buildActivityLogV1({ ...BASIC, theme: 'dark' }) as Record<string, unknown>);
    const dot = getChildren(v1)[0];
    expect(getFill(dot)).toBe('#14532D');
  });
});

describe('buildActivityLogV1 — system mode emits refs', () => {
  it('body fill = $color-text-body', () => {
    const v1 = getRoot(
      buildActivityLogV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>,
    );
    const body = getChildren(v1)[1];
    const line = getChildren(body)[0];
    expect(getFill(line)).toBe('$color-text-body');
  });

  it('timestamp fill = $color-text-subtle', () => {
    const v1 = getRoot(
      buildActivityLogV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>,
    );
    const ts = getChildren(v1)[2];
    expect(getFill(ts)).toBe('$color-text-subtle');
  });

  it('success icon dot bg = $color-success-bg', () => {
    const v1 = getRoot(
      buildActivityLogV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>,
    );
    const dot = getChildren(v1)[0];
    expect(getFill(dot)).toBe('$color-success-bg');
  });

  it('success icon dot fg = $color-success-text', () => {
    const v1 = getRoot(
      buildActivityLogV1({ ...BASIC, theme: 'system' }) as Record<string, unknown>,
    );
    const dot = getChildren(v1)[0];
    const icon = getChildren(dot)[0];
    expect(getFill(icon)).toBe('$color-success-text');
  });
});
