import { describe, it, expect } from 'vitest';
import { compactSubAgentSkills } from '../orchestrator-sub-agent-compact';

/**
 * Model-tier × elements-skill injection e2e. Covers the filter
 * pipeline in `compactSubAgentSkills` specifically WRT the
 * `elements` skill, which is gated TWICE (first by `hasMcpTools`
 * flag in `resolveSkills`, then by the tier allow-list here).
 *
 * `model-profiles-element-tools.test.ts` covers the upstream flag
 * boolean. THIS file covers the downstream "when the flag lets
 * the skill through, does the compact filter preserve it?" — the
 * spot where a regression would silently drop the elements.md
 * content from the sub-agent prompt even though
 * VITE_ENABLE_ELEMENT_TOOLS=1 is set.
 *
 * Each test builds a minimal skill list that contains `elements`
 * plus the skills the compact filter cares about (screen-type
 * gates, style-guide conflicts, simplified-vs-full JSONL). We
 * assert `elements` either survives or is intentionally dropped
 * per tier/reduced-complexity state.
 */

interface MockSkill {
  meta: { name: string };
  content: string;
}

function mock(name: string): MockSkill {
  return { meta: { name }, content: `content-of-${name}` };
}

function namesOf(skills: MockSkill[]): string[] {
  return skills.map((s) => s.meta.name);
}

const CORE_SKILLS = [
  'schema',
  'jsonl-format-simplified',
  'jsonl-format',
  'layout',
  'overflow',
  'text-rules',
  'variables',
  'design-md',
  'mobile-app',
  'landing-page',
  'copywriting',
  'anti-slop',
  'icon-catalog',
  'style-defaults',
  'design-system',
  'elements',
];

describe('compactSubAgentSkills — elements skill injection', () => {
  describe('basic tier allow-list', () => {
    it('preserves elements skill for mobile screens', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'basic', true, false, false);
      expect(namesOf(out)).toContain('elements');
    });

    it('preserves elements skill for non-mobile (dashboard/landing) screens', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'basic', false, false, false);
      expect(namesOf(out)).toContain('elements');
    });

    it('preserves elements skill even when an explicit style guide is active', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'basic', true, true, false);
      expect(namesOf(out)).toContain('elements');
      // And explicit-style-guide should drop design-system
      expect(namesOf(out)).not.toContain('design-system');
    });

    it('DROPS elements skill on reducedComplexity retry (intentional)', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'basic', true, false, true);
      // Retry path strips elements — see code comment rationale:
      // first-attempt failed; elements adds ~17k chars; fallback
      // to legacy batch_design for the retry.
      expect(namesOf(out)).not.toContain('elements');
      // Other retry-allowed skills still present
      expect(namesOf(out)).toContain('schema');
      expect(namesOf(out)).toContain('layout');
    });
  });

  describe('standard tier — no allow-list filter, only screen/style-guide/jsonl gates', () => {
    it('preserves elements skill', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'standard', true, false, false);
      expect(namesOf(out)).toContain('elements');
    });

    it('preserves elements alongside mobile-specific skills on mobile screens', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'standard', true, false, false);
      const names = namesOf(out);
      expect(names).toContain('elements');
      expect(names).toContain('mobile-app');
      // landing-page dropped on mobile
      expect(names).not.toContain('landing-page');
    });

    it('reducedComplexity=true has NO effect on non-basic tiers (only basic retry strips)', () => {
      const skills = CORE_SKILLS.map(mock);
      const standard = compactSubAgentSkills(skills, 'standard', true, false, true);
      const full = compactSubAgentSkills(skills, 'full', true, false, true);
      // Both still contain elements despite reducedComplexity flag
      expect(namesOf(standard)).toContain('elements');
      expect(namesOf(full)).toContain('elements');
    });
  });

  describe('full tier — widest possible filter', () => {
    it('preserves elements skill', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'full', true, false, false);
      expect(namesOf(out)).toContain('elements');
    });
  });

  describe('jsonl-format conflict (simplified vs full)', () => {
    it('when BOTH jsonl-format and jsonl-format-simplified are present, full is dropped', () => {
      const skills = [mock('jsonl-format'), mock('jsonl-format-simplified'), mock('elements')];
      // Non-basic tier so the allow-list doesn't come into play
      const out = compactSubAgentSkills(skills, 'standard', true, false, false);
      const names = namesOf(out);
      expect(names).toContain('jsonl-format-simplified');
      expect(names).not.toContain('jsonl-format');
      expect(names).toContain('elements');
    });

    it('when ONLY jsonl-format is present, it survives', () => {
      const skills = [mock('jsonl-format'), mock('elements'), mock('schema')];
      const out = compactSubAgentSkills(skills, 'standard', true, false, false);
      const names = namesOf(out);
      expect(names).toContain('jsonl-format');
      expect(names).not.toContain('jsonl-format-simplified');
    });
  });

  describe('screen-type gates orthogonal to elements', () => {
    it('mobile screen drops landing-page + copywriting + anti-slop; elements unaffected', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'basic', true, false, false);
      const names = namesOf(out);
      expect(names).toContain('elements');
      expect(names).not.toContain('landing-page');
      expect(names).not.toContain('copywriting');
      expect(names).not.toContain('anti-slop');
      expect(names).toContain('mobile-app');
    });

    it('non-mobile screen drops mobile-app; elements unaffected', () => {
      const skills = CORE_SKILLS.map(mock);
      const out = compactSubAgentSkills(skills, 'standard', false, false, false);
      const names = namesOf(out);
      expect(names).toContain('elements');
      expect(names).not.toContain('mobile-app');
      // Non-mobile → landing-page + copywriting + anti-slop kept
      expect(names).toContain('landing-page');
      expect(names).toContain('copywriting');
    });
  });

  describe('edge cases', () => {
    it('empty skills list returns empty', () => {
      const out = compactSubAgentSkills([], 'basic', true, false, false);
      expect(out).toEqual([]);
    });

    it('elements-only list at basic tier: skill survives', () => {
      const out = compactSubAgentSkills([mock('elements')], 'basic', true, false, false);
      expect(namesOf(out)).toEqual(['elements']);
    });

    it('skill with unknown name at basic tier is dropped (allow-list only)', () => {
      const out = compactSubAgentSkills(
        [mock('elements'), mock('some-future-skill-not-in-allowlist')],
        'basic',
        true,
        false,
        false,
      );
      const names = namesOf(out);
      expect(names).toContain('elements');
      expect(names).not.toContain('some-future-skill-not-in-allowlist');
    });

    it('skill with unknown name at standard tier passes through (no allow-list)', () => {
      const out = compactSubAgentSkills(
        [mock('elements'), mock('some-future-skill-not-in-allowlist')],
        'standard',
        true,
        false,
        false,
      );
      expect(namesOf(out)).toContain('some-future-skill-not-in-allowlist');
    });
  });

  describe('determinism — filter is pure and stable', () => {
    it('same input produces same output across calls', () => {
      const skills = CORE_SKILLS.map(mock);
      const a = compactSubAgentSkills(skills, 'basic', true, false, false);
      const b = compactSubAgentSkills(skills, 'basic', true, false, false);
      expect(namesOf(a)).toEqual(namesOf(b));
    });

    it('original skills array is not mutated', () => {
      const skills = CORE_SKILLS.map(mock);
      const before = skills.map((s) => s.meta.name);
      compactSubAgentSkills(skills, 'basic', true, false, true);
      const after = skills.map((s) => s.meta.name);
      expect(after).toEqual(before);
    });
  });
});
