import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { needsElementTools } from '../model-profiles';
import type { ModelProfile } from '../model-profiles';

/**
 * Feature-flag gate for the N-tool embedded-orchestrator integration.
 *
 * Contract (per plan §3.1 — openpencil-docs
 * superpowers/plans/2026-04-21-element-tools-orchestrator-integration.md):
 *   needsElementTools(p) === true
 *     iff  VITE_ENABLE_ELEMENT_TOOLS env var is truthy
 *     AND  p.tier is 'basic' or 'standard'
 *
 * Why the tier gate: A/B v1 showed ceiling regression on the one
 * full-tier weak model sampled (Kimi K2.5 Δ M1 -12.5pp). Until a
 * follow-up RCA explains that regression, full-tier models are OFF
 * by default even when the global flag is on.
 */

const FLAG = 'VITE_ENABLE_ELEMENT_TOOLS';

function profile(tier: ModelProfile['tier']): ModelProfile {
  return { match: '', tier, label: `Test ${tier}` };
}

/**
 * Stub the flag in BOTH `process.env` AND `import.meta.env`. `vi.stubEnv`
 * only modifies `process.env` under vitest's Node test runner, but our
 * helper also falls back to `import.meta.env` — and THAT gets populated
 * at module-transform time from `.env.local`, which a dev with the
 * feature enabled will have at `VITE_ENABLE_ELEMENT_TOOLS=1`. So we
 * also write directly to the metadata object. Pass `undefined` to
 * simulate "unset" (we stub empty string rather than un-stub because
 * `vi.unstubAllEnvs` restores the original `.env.local`-sourced value).
 */
function setFlag(value: string | undefined): void {
  const v = value ?? '';
  vi.stubEnv(FLAG, v);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const viteEnv = (import.meta as any).env as Record<string, string> | undefined;
  if (viteEnv) viteEnv[FLAG] = v;
}

beforeEach(() => {
  setFlag(undefined);
});

afterEach(() => {
  // Drop all stubs so later test files don't inherit ours. The
  // *next* `beforeEach` in another file will re-stub as needed.
  vi.unstubAllEnvs();
});

describe('needsElementTools — flag OFF (default production state)', () => {
  it('returns false for every tier when the env var is unset', () => {
    expect(needsElementTools(profile('basic'))).toBe(false);
    expect(needsElementTools(profile('standard'))).toBe(false);
    expect(needsElementTools(profile('full'))).toBe(false);
  });

  it('returns false when env var is the literal string "false"', () => {
    setFlag('false');
    expect(needsElementTools(profile('basic'))).toBe(false);
    expect(needsElementTools(profile('standard'))).toBe(false);
  });

  it('returns false when env var is the literal string "0"', () => {
    setFlag('0');
    expect(needsElementTools(profile('basic'))).toBe(false);
  });

  it('returns false on empty string (edge: `export FOO=` style)', () => {
    setFlag('');
    expect(needsElementTools(profile('basic'))).toBe(false);
  });

  it('returns false on whitespace-only value', () => {
    setFlag('   ');
    expect(needsElementTools(profile('basic'))).toBe(false);
  });
});

describe('needsElementTools — flag ON, tier-gated', () => {
  beforeEach(() => {
    setFlag('1');
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
      setFlag(truthy);
      expect(needsElementTools(profile('basic'))).toBe(true);
    });
  }

  for (const falsy of ['2', 'enabled', 'off', 'no']) {
    it(`treats ${JSON.stringify(falsy)} as disabled (strict allow-list)`, () => {
      setFlag(falsy);
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
  //
  // Safety contract under test: **the helper must not throw** when
  // `process` / `process.env` is absent or errors on access. We don't
  // assert the returned boolean here because vitest's Node test runner
  // can't stub `import.meta.env` from the test file's module across
  // into the module under test (per-module import.meta), and the
  // user's `.env.local` may inline `'1'` at the source level. The
  // no-throw behavior is what the Codex stop-hook flagged — that's
  // the actual regression to guard against.
  //
  // Separately, the "flag OFF" suite above proves the boolean path
  // via `process.env` stubs, so we don't duplicate that here.

  it('does not throw when `process` is temporarily undefined (simulated browser)', () => {
    const originalProcess = globalThis.process;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (globalThis as any).process = undefined;
    try {
      expect(() => needsElementTools(profile('basic'))).not.toThrow();
    } finally {
      globalThis.process = originalProcess;
    }
  });

  it('does not throw when `process.env` is missing (simulated sandbox)', () => {
    const originalEnv = globalThis.process?.env;
    if (globalThis.process) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (globalThis.process as any).env = undefined;
    }
    try {
      expect(() => needsElementTools(profile('basic'))).not.toThrow();
    } finally {
      if (globalThis.process && originalEnv) {
        globalThis.process.env = originalEnv;
      }
    }
  });

  it('does not throw when reading `process.env` throws (simulated Deno/workerd)', () => {
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
      expect(() => needsElementTools(profile('basic'))).not.toThrow();
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
