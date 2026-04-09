import { describe, it, expect } from 'vitest';
import { matchTrigger, filterByIntent } from '../engine/resolver';
import type { SkillRegistryEntry } from '../engine/types';

const skill = (
  name: string,
  trigger: null | { keywords: string[] } | { flags: string[] },
  priority = 50,
): SkillRegistryEntry => ({
  meta: {
    name,
    description: '',
    phase: ['generation'],
    trigger,
    priority,
    budget: 2000,
    category: 'domain',
  },
  content: `content of ${name}`,
});

describe('matchTrigger', () => {
  it('null trigger always matches', () => {
    expect(matchTrigger(null, 'any message', {})).toBe(true);
  });

  it('keyword trigger matches case-insensitively', () => {
    expect(matchTrigger({ keywords: ['landing'] }, 'Build a Landing Page', {})).toBe(true);
    expect(matchTrigger({ keywords: ['landing'] }, 'Build a dashboard', {})).toBe(false);
  });

  it('keyword trigger matches if any keyword matches', () => {
    expect(matchTrigger({ keywords: ['dashboard', 'table'] }, 'Create a table view', {})).toBe(
      true,
    );
  });

  it('flag trigger matches when all flags are true', () => {
    expect(matchTrigger({ flags: ['hasVariables'] }, '', { hasVariables: true })).toBe(true);
    expect(matchTrigger({ flags: ['hasVariables'] }, '', { hasVariables: false })).toBe(false);
    expect(matchTrigger({ flags: ['hasVariables'] }, '', {})).toBe(false);
  });

  it('flag trigger requires ALL flags to be true', () => {
    expect(
      matchTrigger({ flags: ['hasVariables', 'hasDesignMd'] }, '', {
        hasVariables: true,
        hasDesignMd: false,
      }),
    ).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Word-boundary matching for ASCII keywords (regression: Codex caught the
  // substring matcher false-triggering `form` on `platform`/`information`)
  // -------------------------------------------------------------------------

  it('does NOT false-match `form` against `platform`', () => {
    // The exact bug Codex caught: a SaaS platform prompt was loading
    // form-ui because the matcher used String.includes(), which made
    // `form` a substring of `platform`. With word-boundary matching,
    // `\bform\b` only matches the standalone word.
    expect(matchTrigger({ keywords: ['form'] }, 'Design a SaaS platform', {})).toBe(false);
  });

  it('does NOT false-match `form` against `information` / `format` / `perform`', () => {
    expect(matchTrigger({ keywords: ['form'] }, 'Show information clearly', {})).toBe(false);
    expect(matchTrigger({ keywords: ['form'] }, 'Display formatted text', {})).toBe(false);
    expect(matchTrigger({ keywords: ['form'] }, 'Performance dashboard', {})).toBe(false);
    expect(matchTrigger({ keywords: ['form'] }, 'Transform the data', {})).toBe(false);
  });

  it('STILL matches `form` as a standalone word', () => {
    expect(matchTrigger({ keywords: ['form'] }, 'Design a contact form', {})).toBe(true);
    expect(matchTrigger({ keywords: ['form'] }, 'Build a form-heavy page', {})).toBe(true);
    expect(matchTrigger({ keywords: ['form'] }, 'A form for user input', {})).toBe(true);
  });

  it('matches multi-word keywords with spaces (e.g. `sign up`)', () => {
    expect(matchTrigger({ keywords: ['sign up'] }, 'Build a sign up screen', {})).toBe(true);
    // Counter-test: "designing up" must NOT match "sign up" — even though
    // the substring "sign up" appears inside "designing up", the word
    // boundary before "sign" requires a non-word char, and "esi" -> "sign"
    // has no boundary.
    expect(matchTrigger({ keywords: ['sign up'] }, 'A designing up north scene', {})).toBe(false);
  });

  it('matches hyphenated keywords (e.g. `react-native`)', () => {
    expect(matchTrigger({ keywords: ['react-native'] }, 'Build with react-native', {})).toBe(true);
    // Different hyphenation: "react native" (space) should also match
    // because the regex word boundaries treat both `-` and ` ` as
    // non-word chars... actually they DO NOT match the same string.
    // The regex `\breact-native\b` looks for literal "react-native".
    // "react native" is a different string. This documents the
    // current behavior — register both forms in the keyword list if
    // both should match.
    expect(matchTrigger({ keywords: ['react-native'] }, 'Build with react native', {})).toBe(false);
  });

  it('keeps substring matching for non-ASCII keywords (CJK)', () => {
    // Chinese characters have no whitespace word boundaries, so the
    // word-boundary path can't apply. The matcher must fall back to
    // substring matching for non-ASCII keywords.
    expect(matchTrigger({ keywords: ['表单'] }, '设计一个登录表单', {})).toBe(true);
    expect(matchTrigger({ keywords: ['登录'] }, '设计一个登录页面', {})).toBe(true);
    // Counter: a non-matching CJK message
    expect(matchTrigger({ keywords: ['表单'] }, '设计一个仪表盘', {})).toBe(false);
  });

  it('keyword case-insensitive even with word boundaries', () => {
    expect(matchTrigger({ keywords: ['login'] }, 'A LOGIN screen', {})).toBe(true);
    expect(matchTrigger({ keywords: ['LOGIN'] }, 'a login form', {})).toBe(true);
  });
});

describe('filterByIntent', () => {
  it('includes always-on skills', () => {
    const skills = [skill('base', null), skill('landing', { keywords: ['landing'] })];
    const result = filterByIntent(skills, 'build a dashboard', {});
    expect(result.map((s) => s.meta.name)).toEqual(['base']);
  });

  it('includes keyword-matched skills', () => {
    const skills = [skill('base', null), skill('landing', { keywords: ['landing'] })];
    const result = filterByIntent(skills, 'build a landing page', {});
    expect(result.map((s) => s.meta.name)).toEqual(['base', 'landing']);
  });

  it('sorts by priority', () => {
    const skills = [skill('b', null, 50), skill('a', null, 10), skill('c', null, 30)];
    const result = filterByIntent(skills, '', {});
    expect(result.map((s) => s.meta.name)).toEqual(['a', 'c', 'b']);
  });
});
