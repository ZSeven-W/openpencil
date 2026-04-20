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

describe('needsElementTools — browser-safe env access (no ReferenceError)', () => {
  // Codex stop-hook regression guard: this helper is imported by
  // orchestrator-sub-agent.ts which runs in the Vite browser bundle.
  // Vite does not polyfill `process` by default; a bare
  // `process.env.X` read there raises `ReferenceError: process is
  // not defined` BEFORE the flag check can return the safe default.
  // The helper must swallow that failure mode.

  it('returns false when `process` is temporarily undefined (simulated browser)', () => {
    const originalProcess = globalThis.process;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (globalThis as any).process = undefined;
    try {
      expect(needsElementTools(profile('basic'))).toBe(false);
    } finally {
      globalThis.process = originalProcess;
    }
  });

  it('returns false when `process.env` is missing (simulated sandbox)', () => {
    const originalEnv = globalThis.process?.env;
    if (globalThis.process) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (globalThis.process as any).env = undefined;
    }
    try {
      expect(needsElementTools(profile('basic'))).toBe(false);
    } finally {
      if (globalThis.process && originalEnv) {
        globalThis.process.env = originalEnv;
      }
    }
  });

  it('returns false when reading `process.env` throws (simulated Deno/workerd)', () => {
    const originalEnv = globalThis.process?.env;
    if (globalThis.process) {
      Object.defineProperty(globalThis.process, 'env', {
        configurable: true,
        get() {
          throw new Error('env access denied in this sandbox');
        },
      });
    }
    try {
      expect(needsElementTools(profile('basic'))).toBe(false);
    } finally {
      if (globalThis.process && originalEnv) {
        Object.defineProperty(globalThis.process, 'env', {
          configurable: true,
          writable: true,
          value: originalEnv,
        });
      }
    }
  });
});
