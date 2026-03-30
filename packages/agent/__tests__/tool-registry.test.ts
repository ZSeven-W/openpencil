import { describe, it, expect } from 'vitest';
import { jsonSchema } from 'ai';
import { createToolRegistry } from '../src/tools/tool-registry';

/** Simple JSON schema for tests — avoids zod ESM resolution issue in vitest. */
const emptySchema = jsonSchema({ type: 'object', properties: {} });
const pathSchema = jsonSchema({
  type: 'object',
  properties: { path: { type: 'string' } },
  required: ['path'],
});
const insertSchema = jsonSchema({
  type: 'object',
  properties: { parent: { type: 'string' }, data: { type: 'object' } },
  required: ['parent', 'data'],
});

describe('createToolRegistry', () => {
  it('registers and retrieves a tool', () => {
    const registry = createToolRegistry();
    registry.register({
      name: 'read_file',
      description: 'Read a file',
      schema: pathSchema,
      level: 'read',
    });
    expect(registry.get('read_file')).toBeDefined();
    expect(registry.get('read_file')!.level).toBe('read');
  });

  it('lists all registered tools', () => {
    const registry = createToolRegistry();
    registry.register({ name: 'tool_a', description: 'A', schema: emptySchema, level: 'read' });
    registry.register({ name: 'tool_b', description: 'B', schema: emptySchema, level: 'modify' });
    expect(registry.list()).toHaveLength(2);
  });

  it('converts to Vercel AI SDK format', () => {
    const registry = createToolRegistry();
    registry.register({
      name: 'insert_node',
      description: 'Insert a node',
      schema: insertSchema,
      level: 'create',
    });
    const sdkTools = registry.toAISDKFormat();
    expect(sdkTools).toHaveProperty('insert_node');
    expect(sdkTools.insert_node).toHaveProperty('description', 'Insert a node');
    expect(sdkTools.insert_node).toHaveProperty('inputSchema');
  });

  it('throws on duplicate registration', () => {
    const registry = createToolRegistry();
    const tool = { name: 'dup', description: 'D', schema: emptySchema, level: 'read' as const };
    registry.register(tool);
    expect(() => registry.register(tool)).toThrow();
  });

  // --- New lifecycle methods ---

  describe('unregister', () => {
    it('removes a registered tool and returns true', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'rm_me', description: 'X', schema: emptySchema, level: 'read' });
      expect(registry.unregister('rm_me')).toBe(true);
      expect(registry.get('rm_me')).toBeUndefined();
    });

    it('returns false for a non-existent tool', () => {
      const registry = createToolRegistry();
      expect(registry.unregister('ghost')).toBe(false);
    });

    it('allows re-registration after unregister', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'temp', description: 'T', schema: emptySchema, level: 'read' });
      registry.unregister('temp');
      // Should not throw on re-register
      registry.register({
        name: 'temp',
        description: 'T2',
        schema: emptySchema,
        level: 'modify',
      });
      expect(registry.get('temp')!.level).toBe('modify');
    });
  });

  describe('replace', () => {
    it('atomically replaces an existing tool', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'swap', description: 'Old', schema: emptySchema, level: 'read' });
      registry.replace({ name: 'swap', description: 'New', schema: emptySchema, level: 'modify' });
      expect(registry.get('swap')!.description).toBe('New');
      expect(registry.get('swap')!.level).toBe('modify');
    });

    it('throws if tool does not exist', () => {
      const registry = createToolRegistry();
      expect(() =>
        registry.replace({
          name: 'nope',
          description: 'X',
          schema: emptySchema,
          level: 'read',
        }),
      ).toThrow('not registered');
    });

    it('does not expose intermediate state (tool count stays the same)', () => {
      const registry = createToolRegistry();
      registry.register({ name: 't', description: 'V1', schema: emptySchema, level: 'read' });
      const countBefore = registry.list().length;
      registry.replace({ name: 't', description: 'V2', schema: emptySchema, level: 'read' });
      expect(registry.list().length).toBe(countBefore);
    });
  });

  describe('snapshot', () => {
    it('returns a copy of all tools', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'a', description: 'A', schema: emptySchema, level: 'read' });
      registry.register({ name: 'b', description: 'B', schema: emptySchema, level: 'modify' });
      const snap = registry.snapshot();
      expect(snap.size).toBe(2);
      expect(snap.get('a')!.description).toBe('A');
    });

    it('mutations to snapshot do not affect the registry', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'x', description: 'X', schema: emptySchema, level: 'read' });
      const snap = registry.snapshot();
      snap.delete('x');
      expect(registry.get('x')).toBeDefined();
    });
  });

  describe('getByPlugin', () => {
    it('returns tools matching a pluginName', () => {
      const registry = createToolRegistry();
      registry.register({
        name: 'p1_a',
        description: 'A',
        schema: emptySchema,
        level: 'read',
        pluginName: 'plugin-one',
      });
      registry.register({
        name: 'p1_b',
        description: 'B',
        schema: emptySchema,
        level: 'modify',
        pluginName: 'plugin-one',
      });
      registry.register({
        name: 'p2_a',
        description: 'C',
        schema: emptySchema,
        level: 'read',
        pluginName: 'plugin-two',
      });
      const result = registry.getByPlugin('plugin-one');
      expect(result).toHaveLength(2);
      expect(result.map((t) => t.name).sort()).toEqual(['p1_a', 'p1_b']);
    });

    it('returns empty array when no tools match', () => {
      const registry = createToolRegistry();
      registry.register({ name: 'x', description: 'X', schema: emptySchema, level: 'read' });
      expect(registry.getByPlugin('nonexistent')).toHaveLength(0);
    });
  });
});
