import { describe, it, expect } from 'vitest';
import { estimateTokens, trimByBudget } from '../engine/budget';
import type { SkillRegistryEntry } from '../engine/types';

describe('estimateTokens', () => {
  it('approximates tokens as chars / 4', () => {
    expect(estimateTokens('abcd')).toBe(1);
    expect(estimateTokens('a'.repeat(400))).toBe(100);
  });

  it('returns 0 for empty string', () => {
    expect(estimateTokens('')).toBe(0);
  });
});

describe('trimByBudget', () => {
  const skill = (
    name: string,
    category: 'base' | 'domain' | 'knowledge',
    chars: number,
  ): SkillRegistryEntry => ({
    meta: {
      name,
      description: '',
      phase: ['generation'],
      trigger: null,
      priority: 0,
      budget: 9999,
      category,
    },
    content: 'x'.repeat(chars),
  });

  it('keeps all skills when within budget', () => {
    const skills = [skill('a', 'base', 40), skill('b', 'domain', 40)];
    const result = trimByBudget(skills, 100);
    expect(result).toHaveLength(2);
    expect(result.every((s) => !s.truncated)).toBe(true);
  });

  it('never cuts base category skills', () => {
    const skills = [skill('a', 'base', 20000)];
    const result = trimByBudget(skills, 100);
    expect(result).toHaveLength(1);
    expect(result[0].meta.name).toBe('a');
  });

  it('drops knowledge skills first when over budget', () => {
    const skills = [
      skill('base1', 'base', 400),
      skill('domain1', 'domain', 400),
      skill('know1', 'knowledge', 400),
    ];
    const result = trimByBudget(skills, 250);
    expect(result).toHaveLength(2);
    expect(result.map((s) => s.meta.name)).toEqual(['base1', 'domain1']);
  });

  it('truncates domain skill content when over budget after dropping knowledge', () => {
    const skills = [skill('base1', 'base', 400), skill('domain1', 'domain', 2000)];
    const result = trimByBudget(skills, 200);
    expect(result).toHaveLength(2);
    const domain = result.find((s) => s.meta.name === 'domain1')!;
    expect(domain.truncated).toBe(true);
    expect(domain.tokenCount).toBeLessThanOrEqual(200);
  });

  it('respects per-skill budget cap', () => {
    const s = skill('a', 'domain', 8000);
    s.meta.budget = 500;
    const result = trimByBudget([s], 10000);
    expect(result[0].tokenCount).toBeLessThanOrEqual(500);
    expect(result[0].truncated).toBe(true);
  });
});
