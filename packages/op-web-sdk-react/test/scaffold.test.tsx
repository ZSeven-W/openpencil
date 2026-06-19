import { describe, it, expect } from 'vitest';
import { VERSION } from '../src/index.js';
describe('react adapter scaffold', () => {
  it('exposes VERSION', () => { expect(typeof VERSION).toBe('string'); });
});
