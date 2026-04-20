import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { needsElementTools } from '../model-profiles';
import type { ModelProfile } from '../model-profiles';

/**
 * Feature-flag gate for the N-tool embedded-orchestrator integration.
 *
 * Contract (per plan §3.1 — openpencil-docs
 * superpowers/plans/2026-04-21-element-tools-orchestrator-integration.md):
 *   needsElementTools(p) === true
 *     iff  ENABLE_ELEMENT_TOOLS_IN_ORCHESTRATOR env var is truthy
 *     AND  p.tier is 'basic' or 'standard'
 *
 * Why the tier gate: A/B v1 showed ceiling regression on the one
 * full-tier weak model sampled (Kimi K2.5 Δ M1 -12.5pp). Until a
 * follow-up RCA explains that regression, full-tier models are OFF
 * by default even when the global flag is on.
 */

const FLAG = 'ENABLE_ELEMENT_TOOLS_IN_ORCHESTRATOR';

function profile(tier: ModelProfile['tier']): ModelProfile {
  return { match: '', tier, label: `Test ${tier}` };
}

const originalValue = process.env[FLAG];

beforeEach(() => {
  delete process.env[FLAG];
});

afterEach(() => {
  if (originalValue === undefined) delete process.env[FLAG];
  else process.env[FLAG] = originalValue;
});

describe('needsElementTools — flag OFF (default production state)', () => {
  it('returns false for every tier when the env var is unset', () => {
    expect(needsElementTools(profile('basic'))).toBe(false);
    expect(needsElementTools(profile('standard'))).toBe(false);
    expect(needsElementTools(profile('full'))).toBe(false);
  });

  it('returns false when env var is the literal string "false"', () => {
    process.env[FLAG] = 'false';
    expect(needsElementTools(profile('basic'))).toBe(false);
    expect(needsElementTools(profile('standard'))).toBe(false);
  });

  it('returns false when env var is the literal string "0"', () => {
    process.env[FLAG] = '0';
    expect(needsElementTools(profile('basic'))).toBe(false);
  });

  it('returns false on empty string (edge: `export FOO=` style)', () => {
    process.env[FLAG] = '';
    expect(needsElementTools(profile('basic'))).toBe(false);
  });

  it('returns false on whitespace-only value', () => {
    process.env[FLAG] = '   ';
    expect(needsElementTools(profile('basic'))).toBe(false);
  });
});

describe('needsElementTools — flag ON, tier-gated', () => {
  beforeEach(() => {
    process.env[FLAG] = '1';
  });

  it('returns true for basic tier', () => {
    expect(needsElementTools(profile('basic'))).toBe(true);
  });

  it('returns true for standard tier', () => {
    expect(needsElementTools(profile('standard'))).toBe(true);
  });

  it('returns FALSE for full tier (ceiling-effect guard)', () => {
    // Kimi K2.5 regressed -12.5pp in A/B v1; full tier stays off by
    // default until RCA produces a finding that changes the policy.
    expect(needsElementTools(profile('full'))).toBe(false);
  });
});

describe('needsElementTools — truthy-value parsing', () => {
  for (const truthy of ['1', 'true', 'TRUE', 'yes', 'YES', 'on', 'ON', ' true ']) {
    it(`treats ${JSON.stringify(truthy)} as enabled`, () => {
      process.env[FLAG] = truthy;
      expect(needsElementTools(profile('basic'))).toBe(true);
    });
  }

  for (const falsy of ['2', 'enabled', 'off', 'no']) {
    it(`treats ${JSON.stringify(falsy)} as disabled (strict allow-list)`, () => {
      process.env[FLAG] = falsy;
      expect(needsElementTools(profile('basic'))).toBe(false);
    });
  }
});
