import { describe, it, expect } from 'vitest';
import { VERSION } from '../src/index.js';

describe('package scaffold', () => {
  it('exposes a VERSION string', () => {
    expect(typeof VERSION).toBe('string');
  });
});
