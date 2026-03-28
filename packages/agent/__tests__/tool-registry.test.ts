import { describe, it, expect } from 'vitest'
import { z } from 'zod'
import { createToolRegistry } from '../src/tools/tool-registry'

describe('createToolRegistry', () => {
  it('registers and retrieves a tool', () => {
    const registry = createToolRegistry()
    registry.register({
      name: 'read_file',
      description: 'Read a file',
      schema: z.object({ path: z.string() }),
      level: 'read',
    })
    expect(registry.get('read_file')).toBeDefined()
    expect(registry.get('read_file')!.level).toBe('read')
  })

  it('lists all registered tools', () => {
    const registry = createToolRegistry()
    registry.register({ name: 'tool_a', description: 'A', schema: z.object({}), level: 'read' })
    registry.register({ name: 'tool_b', description: 'B', schema: z.object({}), level: 'modify' })
    expect(registry.list()).toHaveLength(2)
  })

  it('converts to Vercel AI SDK format', () => {
    const registry = createToolRegistry()
    registry.register({
      name: 'insert_node',
      description: 'Insert a node',
      schema: z.object({ parent: z.string(), data: z.object({}).passthrough() }),
      level: 'create',
    })
    const sdkTools = registry.toAISDKFormat()
    expect(sdkTools).toHaveProperty('insert_node')
    expect(sdkTools.insert_node).toHaveProperty('description', 'Insert a node')
    expect(sdkTools.insert_node).toHaveProperty('inputSchema')
  })

  it('throws on duplicate registration', () => {
    const registry = createToolRegistry()
    const tool = { name: 'dup', description: 'D', schema: z.object({}), level: 'read' as const }
    registry.register(tool)
    expect(() => registry.register(tool)).toThrow()
  })
})
