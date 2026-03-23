import { describe, it, expect, beforeEach } from 'vitest'
import { resolveSkills } from '../engine/resolve-skills'
import { setSkillRegistry } from '../engine/loader'
import type { SkillRegistryEntry } from '../engine/types'

const mkSkill = (
  name: string,
  phase: string[],
  opts: Partial<SkillRegistryEntry['meta']> = {},
  content = `content of ${name}`
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
})

describe('resolveSkills', () => {
  beforeEach(() => {
    setSkillRegistry([
      mkSkill('decomposition', ['planning'], { priority: 0 }),
      mkSkill('schema', ['generation'], { priority: 0 }),
      mkSkill('layout', ['generation'], { priority: 10 }),
      mkSkill('landing', ['generation'], { trigger: { keywords: ['landing'] }, priority: 50, category: 'domain' }),
      mkSkill('cjk', ['generation'], { trigger: { keywords: ['chinese'] }, priority: 25, category: 'domain' }),
      mkSkill('variables', ['generation'], { trigger: { flags: ['hasVariables'] }, priority: 45 }),
      mkSkill('vision', ['validation'], { priority: 0 }),
      mkSkill('local-edit', ['maintenance'], { priority: 0 }),
    ])
  })

  it('filters by phase', () => {
    const ctx = resolveSkills('planning', 'build a landing page')
    expect(ctx.skills.map(s => s.meta.name)).toEqual(['decomposition'])
    expect(ctx.phase).toBe('planning')
  })

  it('matches keywords in generation phase', () => {
    const ctx = resolveSkills('generation', 'build a landing page')
    expect(ctx.skills.map(s => s.meta.name)).toContain('landing')
  })

  it('does not include keyword-triggered skills when message does not match', () => {
    const ctx = resolveSkills('generation', 'build a dashboard')
    expect(ctx.skills.map(s => s.meta.name)).not.toContain('landing')
  })

  it('includes flag-triggered skills when flag is set', () => {
    const ctx = resolveSkills('generation', 'build something', { flags: { hasVariables: true } })
    expect(ctx.skills.map(s => s.meta.name)).toContain('variables')
  })

  it('excludes flag-triggered skills when flag is not set', () => {
    const ctx = resolveSkills('generation', 'build something')
    expect(ctx.skills.map(s => s.meta.name)).not.toContain('variables')
  })

  it('returns AgentContext with correct structure', () => {
    const ctx = resolveSkills('validation', 'check design')
    expect(ctx.role).toBe('general')
    expect(ctx.phase).toBe('validation')
    expect(ctx.budget.max).toBe(3000)
    expect(ctx.budget.used).toBeGreaterThanOrEqual(0)
    expect(ctx.memory).toEqual({})
  })

  it('injects dynamic content into placeholders', () => {
    setSkillRegistry([
      mkSkill('design-md', ['generation'], { trigger: { flags: ['hasDesignMd'] } }, 'Theme: {{designMdContent}}'),
    ])
    const ctx = resolveSkills('generation', 'build', {
      flags: { hasDesignMd: true },
      dynamicContent: { designMdContent: 'Dark modern' },
    })
    expect(ctx.skills[0].content).toBe('Theme: Dark modern')
  })

  it('respects budgetOverride', () => {
    const ctx = resolveSkills('generation', 'test', { budgetOverride: 500 })
    expect(ctx.budget.max).toBe(500)
  })
})
