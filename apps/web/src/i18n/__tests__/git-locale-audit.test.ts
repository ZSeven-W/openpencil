import { describe, it, expect } from 'vitest';
import en from '../locales/en';

// Keys in this set bypass the audit rules below.
// Add an entry here ONLY for a string that is genuinely future-roadmap copy
// (i.e. the feature has not shipped and "Phase N" text is intentional).
// Do NOT use the allowlist to paper over stale copy — fix the copy instead.
const GIT_COPY_AUDIT_ALLOWLIST = new Set<string>();

// ── helpers ──────────────────────────────────────────────────────────────────

type Violation = { key: string; value: string; reason: string };

const gitEntries = Object.entries(en).filter(([key]) => key.startsWith('git.'));

// Rule 1: no value mentions "coming in Phase <digit>" for a completed phase.
function ruleComingInPhase(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key, value]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return /coming in Phase \d/i.test(value);
    })
    .map(([key, value]) => ({ key, value, reason: 'value contains "coming in Phase N"' }));
}

// Rule 2: no key under git.placeholder.* (these are dead UI scaffolding labels).
function rulePlaceholderKey(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return key.startsWith('git.placeholder.');
    })
    .map(([key, value]) => ({
      key,
      value,
      reason: 'key matches git.placeholder.* (dead scaffolding — delete the key)',
    }));
}

// Rule 3: no value that contains the word "placeholder" (case-insensitive).
// UI strings should describe real behavior, not mark a future slot.
function rulePlaceholderValue(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key, value]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return /placeholder/i.test(value);
    })
    .map(([key, value]) => ({ key, value, reason: 'value contains "placeholder"' }));
}

// ── tests ─────────────────────────────────────────────────────────────────────

describe('Git locale audit (en.ts)', () => {
  it('no git.* value says "coming in Phase N"', () => {
    const violations = ruleComingInPhase(gitEntries);
    expect(violations).toEqual([]);
  });

  it('no git.placeholder.* key exists (dead keys must be deleted)', () => {
    const violations = rulePlaceholderKey(gitEntries);
    expect(violations).toEqual([]);
  });

  it('no git.* value contains the word "placeholder"', () => {
    const violations = rulePlaceholderValue(gitEntries);
    expect(violations).toEqual([]);
  });
});
