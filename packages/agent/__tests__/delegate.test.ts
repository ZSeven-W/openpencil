import { describe, it, expect } from 'vitest'
import { createDelegateTool } from '../src/tools/delegate'

describe('createDelegateTool', () => {
  it('creates a tool with orchestrate level', () => {
    const tool = createDelegateTool(['designer', 'reviewer'])
    expect(tool.name).toBe('delegate')
    expect(tool.level).toBe('orchestrate')
  })

  it('schema validates member names', () => {
    const tool = createDelegateTool(['designer', 'reviewer'])
    const result = tool.schema.safeParse({ member: 'designer', task: 'do thing' })
    expect(result.success).toBe(true)
  })

  it('schema rejects unknown members', () => {
    const tool = createDelegateTool(['designer', 'reviewer'])
    const result = tool.schema.safeParse({ member: 'unknown', task: 'do thing' })
    expect(result.success).toBe(false)
  })
})
