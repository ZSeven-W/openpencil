import { describe, it, expectTypeOf } from 'vitest'
import type {
  Phase,
  SkillTrigger,
  AgentContext,
} from '../engine/types'

describe('engine types', () => {
  it('Phase is a string union', () => {
    expectTypeOf<Phase>().toEqualTypeOf<
      'planning' | 'generation' | 'validation' | 'maintenance'
    >()
  })

  it('SkillTrigger covers all trigger shapes', () => {
    const alwaysOn: SkillTrigger = null
    const keywords: SkillTrigger = { keywords: ['landing'] }
    const flags: SkillTrigger = { flags: ['hasVariables'] }
    expectTypeOf(alwaysOn).toMatchTypeOf<SkillTrigger>()
    expectTypeOf(keywords).toMatchTypeOf<SkillTrigger>()
    expectTypeOf(flags).toMatchTypeOf<SkillTrigger>()
  })

  it('AgentContext has role field for future differentiation', () => {
    expectTypeOf<AgentContext>().toHaveProperty('role')
    expectTypeOf<AgentContext>().toHaveProperty('phase')
    expectTypeOf<AgentContext>().toHaveProperty('skills')
    expectTypeOf<AgentContext>().toHaveProperty('memory')
    expectTypeOf<AgentContext>().toHaveProperty('budget')
  })
})
