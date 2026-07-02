import { describe, it, expect } from 'vitest';
import { validateContract } from '../utils/validate-contract';
import type { ChunkResult } from '@zseven-w/pen-types';

describe('validateContract', () => {
  const makeResult = (overrides: Partial<ChunkResult> = {}): ChunkResult => ({
    chunkId: 'c1',
    code: 'function HeroSection() { return <div />; }',
    contract: {
      chunkId: 'c1',
      componentName: 'HeroSection',
      exportedProps: [],
      slots: [],
      cssClasses: [],
      cssVariables: [],
      imports: [],
    },
    ...overrides,
  });

  it('passes valid contract', () => {
    const result = validateContract(makeResult());
    expect(result.valid).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  it('rejects invalid PascalCase componentName', () => {
    const result = validateContract(
      makeResult({
        contract: {
          ...makeResult().contract,
          componentName: 'hero-section',
        },
      }),
    );
    expect(result.valid).toBe(false);
    expect(result.issues[0]).toContain('PascalCase');
  });

  it('warns when componentName not found in code', () => {
    const result = validateContract(
      makeResult({
        code: 'export default function() {}',
      }),
    );
    expect(result.valid).toBe(false);
    expect(result.issues[0]).toContain('not found in generated code');
  });

  it('skips componentName check for SFC frameworks', () => {
    const result = validateContract(
      makeResult({
        code: '<template><div></div></template><script>export default {}</script>',
      }),
    );
    expect(result.valid).toBe(true);
  });
});
