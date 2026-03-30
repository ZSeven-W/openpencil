import { describe, it, expect } from 'vitest';
import { createDelegateTool } from '../src/tools/delegate';

describe('createDelegateTool', () => {
  it('creates a tool with orchestrate level', () => {
    const tool = createDelegateTool(['designer', 'reviewer']);
    expect(tool.name).toBe('delegate');
    expect(tool.level).toBe('orchestrate');
  });

  it('schema validates member names', async () => {
    const tool = createDelegateTool(['designer', 'reviewer']);
    const schema = tool.schema as { validate?: (v: unknown) => unknown };
    const result = await schema.validate?.({ member: 'designer', task: 'do thing' });
    expect((result as { success: boolean }).success).toBe(true);
  });

  it('schema rejects unknown members', async () => {
    const tool = createDelegateTool(['designer', 'reviewer']);
    const schema = tool.schema as { validate?: (v: unknown) => unknown };
    const result = await schema.validate?.({ member: 'unknown', task: 'do thing' });
    expect((result as { success: boolean }).success).toBe(false);
  });
});
