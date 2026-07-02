import { describe, it, expect, beforeEach } from 'vitest';
import {
  coerceEnum,
  coerceNonEmptyArray,
  coerceNumberArray,
  coerceStringArray,
  coerceNonEmptyString,
  getCoerceWarnings,
  clearCoerceWarnings,
} from '../element-builders/coerce-params.js';

describe('coerce-params', () => {
  beforeEach(() => clearCoerceWarnings());

  describe('coerceEnum', () => {
    const VALID = ['a', 'b', 'c'] as const;

    it('returns valid value unchanged', () => {
      expect(coerceEnum('b', VALID, 'a', 'B', 'p')).toBe('b');
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('returns default silently for undefined / null', () => {
      expect(coerceEnum(undefined, VALID, 'a', 'B', 'p')).toBe('a');
      expect(coerceEnum(null, VALID, 'a', 'B', 'p')).toBe('a');
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('warns + returns default for invalid enum string', () => {
      expect(coerceEnum('z', VALID, 'a', 'B', 'p')).toBe('a');
      const warnings = getCoerceWarnings();
      expect(warnings).toHaveLength(1);
      expect(warnings[0].builder).toBe('B');
      expect(warnings[0].param).toBe('p');
      expect(warnings[0].given).toBe('z');
      expect(warnings[0].fallback).toBe('a');
    });

    it('warns + returns default for non-string', () => {
      expect(coerceEnum(42, VALID, 'a', 'B', 'p')).toBe('a');
      expect(getCoerceWarnings()).toHaveLength(1);
    });
  });

  describe('coerceNonEmptyArray', () => {
    it('returns array unchanged when non-empty', () => {
      expect(coerceNonEmptyArray([1, 2], [99], 'B', 'p')).toEqual([1, 2]);
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('warns + returns fallback for empty array', () => {
      expect(coerceNonEmptyArray([], ['x'], 'B', 'p')).toEqual(['x']);
      expect(getCoerceWarnings()[0].reason).toContain('empty');
    });

    it('warns + returns fallback for undefined / null / non-array', () => {
      expect(coerceNonEmptyArray(undefined, ['x'], 'B', 'p')).toEqual(['x']);
      expect(coerceNonEmptyArray(null, ['x'], 'B', 'p')).toEqual(['x']);
      expect(coerceNonEmptyArray('oops', ['x'], 'B', 'p')).toEqual(['x']);
      expect(getCoerceWarnings()).toHaveLength(3);
    });
  });

  describe('coerceNumberArray', () => {
    it('returns finite numbers unchanged', () => {
      expect(coerceNumberArray([1, 2, 3], [99], 'B', 'p')).toEqual([1, 2, 3]);
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('filters out non-finite entries', () => {
      expect(coerceNumberArray([1, NaN, 2, Infinity, 3], [99], 'B', 'p')).toEqual([1, 2, 3]);
    });

    it('warns + returns fallback when no finite numbers remain', () => {
      expect(coerceNumberArray([NaN, Infinity], [42], 'B', 'p')).toEqual([42]);
      expect(getCoerceWarnings()).toHaveLength(1);
    });

    it('warns + returns fallback for non-array', () => {
      expect(coerceNumberArray(undefined, [42], 'B', 'p')).toEqual([42]);
      expect(coerceNumberArray('oops', [42], 'B', 'p')).toEqual([42]);
      expect(getCoerceWarnings()).toHaveLength(2);
    });
  });

  describe('coerceStringArray', () => {
    it('returns non-empty strings unchanged', () => {
      expect(coerceStringArray(['a', 'b'], ['fb'], 'B', 'p')).toEqual(['a', 'b']);
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('filters out empty / non-string entries', () => {
      expect(coerceStringArray(['a', '', 1, null, 'b'], ['fb'], 'B', 'p')).toEqual(['a', 'b']);
    });

    it('warns + returns fallback when array empty after filter', () => {
      expect(coerceStringArray([1, 2, ''], ['fb'], 'B', 'p')).toEqual(['fb']);
      expect(getCoerceWarnings()).toHaveLength(1);
    });

    it('warns + returns fallback for non-array', () => {
      expect(coerceStringArray(undefined, ['fb'], 'B', 'p')).toEqual(['fb']);
      expect(coerceStringArray('oops', ['fb'], 'B', 'p')).toEqual(['fb']);
      expect(getCoerceWarnings()).toHaveLength(2);
    });
  });

  describe('coerceNonEmptyString', () => {
    it('returns valid string unchanged', () => {
      expect(coerceNonEmptyString('hello', 'fb', 'B', 'p')).toBe('hello');
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('returns fallback silently for undefined / null / empty', () => {
      expect(coerceNonEmptyString(undefined, 'fb', 'B', 'p')).toBe('fb');
      expect(coerceNonEmptyString(null, 'fb', 'B', 'p')).toBe('fb');
      expect(coerceNonEmptyString('', 'fb', 'B', 'p')).toBe('fb');
      expect(coerceNonEmptyString('   ', 'fb', 'B', 'p')).toBe('fb');
      // whitespace-only currently warns (it's a non-empty string of length>0
      // but trim() is empty); ensure all silent paths warn 0 times
      expect(getCoerceWarnings()).toHaveLength(0);
    });

    it('warns + returns fallback for non-string', () => {
      expect(coerceNonEmptyString(42, 'fb', 'B', 'p')).toBe('fb');
      expect(getCoerceWarnings()).toHaveLength(1);
    });
  });

  describe('warning sink lifecycle', () => {
    it('accumulates across calls until cleared', () => {
      coerceEnum('z', ['a'] as const, 'a', 'B', 'p');
      coerceEnum('y', ['a'] as const, 'a', 'B', 'p');
      expect(getCoerceWarnings()).toHaveLength(2);
      clearCoerceWarnings();
      expect(getCoerceWarnings()).toHaveLength(0);
    });
  });
});
