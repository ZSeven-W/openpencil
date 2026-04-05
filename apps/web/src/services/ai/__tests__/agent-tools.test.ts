import { describe, expect, it } from 'vitest';
import { getBuiltinLeadToolDefs, getDesignToolDefs } from '../agent-tools';

describe('agent tool definitions', () => {
  it('keeps generate_design in the generic lead tool set', () => {
    const names = getDesignToolDefs().map((def) => def.name);

    expect(names).toContain('generate_design');
  });

  it('exposes direct layout tools for builtin single-agent mode', () => {
    const names = getBuiltinLeadToolDefs().map((def) => def.name);

    expect(names).toContain('plan_layout');
    expect(names).toContain('batch_insert');
    expect(names).not.toContain('generate_design');
  });
});
