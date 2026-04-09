import { describe, it, expect, beforeEach } from 'vitest';
import { resolveSkills } from '../engine/resolve-skills';
import { setSkillRegistry } from '../engine/loader';
import type { SkillRegistryEntry } from '../engine/types';

const mkSkill = (
  name: string,
  phase: string[],
  opts: Partial<SkillRegistryEntry['meta']> = {},
  content = `content of ${name}`,
): SkillRegistryEntry => ({
  meta: {
    name,
    description: '',
    phase: phase as any[],
    trigger: null,
    priority: 50,
    budget: 2000,
    category: 'base',
    ...opts,
  },
  content,
});

describe('resolveSkills', () => {
  beforeEach(() => {
    setSkillRegistry([
      mkSkill('decomposition', ['planning'], { priority: 0 }),
      mkSkill('schema', ['generation'], { priority: 0 }),
      mkSkill('layout', ['generation'], { priority: 10 }),
      mkSkill('landing', ['generation'], {
        trigger: { keywords: ['landing'] },
        priority: 50,
        category: 'domain',
      }),
      mkSkill('cjk', ['generation'], {
        trigger: { keywords: ['chinese'] },
        priority: 25,
        category: 'domain',
      }),
      mkSkill('variables', ['generation'], { trigger: { flags: ['hasVariables'] }, priority: 45 }),
      mkSkill('vision', ['validation'], { priority: 0 }),
      mkSkill('local-edit', ['maintenance'], { priority: 0 }),
    ]);
  });

  it('filters by phase', () => {
    const ctx = resolveSkills('planning', 'build a landing page');
    expect(ctx.skills.map((s) => s.meta.name)).toEqual(['decomposition']);
    expect(ctx.phase).toBe('planning');
  });

  it('matches keywords in generation phase', () => {
    const ctx = resolveSkills('generation', 'build a landing page');
    expect(ctx.skills.map((s) => s.meta.name)).toContain('landing');
  });

  it('does not include keyword-triggered skills when message does not match', () => {
    const ctx = resolveSkills('generation', 'build a dashboard');
    expect(ctx.skills.map((s) => s.meta.name)).not.toContain('landing');
  });

  it('includes flag-triggered skills when flag is set', () => {
    const ctx = resolveSkills('generation', 'build something', { flags: { hasVariables: true } });
    expect(ctx.skills.map((s) => s.meta.name)).toContain('variables');
  });

  it('excludes flag-triggered skills when flag is not set', () => {
    const ctx = resolveSkills('generation', 'build something');
    expect(ctx.skills.map((s) => s.meta.name)).not.toContain('variables');
  });

  it('returns AgentContext with correct structure', () => {
    const ctx = resolveSkills('validation', 'check design');
    expect(ctx.role).toBe('general');
    expect(ctx.phase).toBe('validation');
    expect(ctx.budget.max).toBe(3000);
    expect(ctx.budget.used).toBeGreaterThanOrEqual(0);
    expect(ctx.memory).toEqual({});
  });

  it('injects dynamic content into placeholders', () => {
    setSkillRegistry([
      mkSkill(
        'design-md',
        ['generation'],
        { trigger: { flags: ['hasDesignMd'] } },
        'Theme: {{designMdContent}}',
      ),
    ]);
    const ctx = resolveSkills('generation', 'build', {
      flags: { hasDesignMd: true },
      dynamicContent: { designMdContent: 'Dark modern' },
    });
    expect(ctx.skills[0].content).toBe('Theme: Dark modern');
  });

  it('respects budgetOverride', () => {
    const ctx = resolveSkills('generation', 'test', { budgetOverride: 500 });
    expect(ctx.budget.max).toBe(500);
  });
});

// ---------------------------------------------------------------------------
// Real-registry regression: codegen skills must not load during design generation
// ---------------------------------------------------------------------------

describe('resolveSkills against the REAL skill registry — codegen gate', () => {
  // The previous bug surfaced via the cross-provider empty-response
  // diagnostic: a fitness app prompt was loading 14 skills including
  // `codegen-flutter`, `design-code`, and `form-ui`, padding the
  // sub-agent system prompt to 31.9K chars. The codegen skills were
  // tagged `phase: [generation]` with overly broad keyword triggers
  // (e.g. codegen-flutter listed `mobile` as a keyword), and
  // design-code had `trigger: null` so it always loaded.
  //
  // The fix: every codegen-* and design-code skill now uses
  // `flags: [isCodeGen]` so they only load during actual code
  // generation, never during design generation. form-ui's keyword
  // list was also pruned of generic terms (mobile, button, card,
  // nav, mockup) that pulled it into every UI prompt.
  //
  // This test uses the REAL generated registry (not a mock) so it
  // catches future regressions in the .md frontmatter.
  beforeEach(async () => {
    // Reset to the REAL generated registry that the previous test
    // suite swapped out via setSkillRegistry().
    const { skillRegistry } = await import('../_generated/skill-registry');
    setSkillRegistry(skillRegistry);
  });

  const FITNESS_PROMPT =
    'Design a health and fitness tracking mobile app homepage. ' +
    'Activity rings, heart rate card, weekly workout summary, ' +
    'upcoming workouts cards, bottom tab bar with 4 tabs.';

  it('does NOT load codegen-flutter for a mobile fitness app prompt', () => {
    const ctx = resolveSkills('generation', FITNESS_PROMPT);
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('codegen-flutter');
  });

  it('does NOT load any platform codegen skill for a design generation prompt', () => {
    const ctx = resolveSkills('generation', FITNESS_PROMPT);
    const names = ctx.skills.map((s) => s.meta.name);
    const platformCodegen = [
      'codegen-react',
      'codegen-flutter',
      'codegen-react-native',
      'codegen-vue',
      'codegen-svelte',
      'codegen-html',
      'codegen-swiftui',
      'codegen-compose',
    ];
    for (const skill of platformCodegen) {
      expect(names, `expected ${skill} not to load`).not.toContain(skill);
    }
  });

  it('does NOT load design-code for a design generation prompt (it is codegen-only)', () => {
    const ctx = resolveSkills('generation', FITNESS_PROMPT);
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('design-code');
  });

  it('does NOT load form-ui for a fitness app prompt (no form keywords)', () => {
    // form-ui used to over-trigger on `mobile`, `button`, `card`,
    // `nav`, `mockup` etc. Now its trigger keywords are restricted
    // to genuine form/auth/checkout terms.
    const ctx = resolveSkills('generation', FITNESS_PROMPT);
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('form-ui');
  });

  it('STILL loads platform codegen when isCodeGen flag is set (the actual codegen path)', () => {
    // Counter-test: the codegen pipeline does pass `isCodeGen: true`
    // and expects the skills to load. If the fix accidentally made
    // them unreachable, the codegen flow would silently lose its
    // platform-specific guidance.
    const ctx = resolveSkills('generation', FITNESS_PROMPT, {
      flags: { isCodeGen: true },
    });
    const names = ctx.skills.map((s) => s.meta.name);
    // At least design-code should appear; the platform-specific
    // codegen-* skills are loaded by name elsewhere via getSkillByName,
    // but design-code goes through resolveSkills.
    expect(names).toContain('design-code');
  });

  it('STILL loads form-ui for a real form prompt (counter-test)', () => {
    const ctx = resolveSkills('generation', 'Design a login form with email and password');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  // The substring-matcher false-positive Codex caught: a SaaS platform
  // dashboard prompt was loading form-ui because the matcher's
  // String.includes() saw `form` as a substring of `platform`.
  it('does NOT load form-ui for a SaaS platform prompt (substring false-positive)', () => {
    const ctx = resolveSkills(
      'generation',
      'Design a SaaS analytics platform dashboard with information cards',
    );
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('form-ui');
  });

  it('does NOT load form-ui for an "information architecture" prompt', () => {
    const ctx = resolveSkills('generation', 'Information architecture review screen');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('form-ui');
  });

  it('STILL loads form-ui for a Chinese form prompt (CJK substring path)', () => {
    const ctx = resolveSkills('generation', '设计一个用户注册表单');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  // ---- Search / input keyword coverage (Codex regression: 01fa48e
  // accidentally dropped 搜索 along with `input` and `email`, breaking
  // Chinese search-bar prompts) ------------------------------------
  it('STILL loads form-ui for a Chinese search prompt (搜索)', () => {
    const ctx = resolveSkills('generation', '设计一个搜索界面');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  it('STILL loads form-ui for a Chinese 搜索框 prompt', () => {
    const ctx = resolveSkills('generation', '页面顶部需要一个搜索框');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  it('STILL loads form-ui for a Chinese 输入框 prompt', () => {
    const ctx = resolveSkills('generation', '请添加一个输入框收集邮箱');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  it('STILL loads form-ui for an English "search bar" prompt', () => {
    const ctx = resolveSkills('generation', 'Add a search bar to the navigation');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  it('STILL loads form-ui for an English "input field" prompt', () => {
    const ctx = resolveSkills('generation', 'A profile screen with an input field for the bio');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('form-ui');
  });

  it('does NOT load form-ui for a "research" prompt (counter-test for word boundaries)', () => {
    // `search` alone would substring-match "research", which is why
    // we avoided adding it as a single-word keyword. Multi-word
    // `search bar` / `search input` are word-boundary safe.
    const ctx = resolveSkills('generation', 'Design a research data dashboard');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('form-ui');
  });

  it('does NOT load form-ui for a generic "input data table" prompt (counter-test)', () => {
    // `input` alone would over-match. `input field` (multi-word) is
    // narrower and doesn't match "input data table".
    const ctx = resolveSkills('generation', 'A table that lists input data records');
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('form-ui');
  });
});
