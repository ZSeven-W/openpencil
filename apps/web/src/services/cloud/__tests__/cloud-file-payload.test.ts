import { describe, expect, it, vi } from 'vitest';
import {
  CloudFilePayloadTooLargeError,
  stringifyCloudFilePayload,
} from '../cloud-file-payload';

describe('stringifyCloudFilePayload', () => {
  it('returns JSON when the encoded payload is within the byte limit', () => {
    expect(stringifyCloudFilePayload({ name: 'ok' }, 32)).toBe('{"name":"ok"}');
  });

  it('throws a typed error when the encoded payload is too large', () => {
    expect(() => stringifyCloudFilePayload({ content: 'abcdefghij' }, 8)).toThrow(
      CloudFilePayloadTooLargeError,
    );

    try {
      stringifyCloudFilePayload({ content: 'abcdefghij' }, 8);
    } catch (err) {
      expect(err).toBeInstanceOf(CloudFilePayloadTooLargeError);
      expect((err as CloudFilePayloadTooLargeError).maxBytes).toBe(8);
      expect((err as CloudFilePayloadTooLargeError).sizeBytes).toBeGreaterThan(8);
    }
  });

  it('wraps browser string allocation failures as a typed size error', () => {
    const stringify = vi.spyOn(JSON, 'stringify').mockImplementationOnce(() => {
      throw new RangeError('Invalid string length');
    });

    let caught: unknown;
    try {
      stringifyCloudFilePayload({ document: { children: [] } });
    } catch (err) {
      caught = err;
    } finally {
      stringify.mockRestore();
    }

    expect(caught).toBeInstanceOf(CloudFilePayloadTooLargeError);
    expect((caught as CloudFilePayloadTooLargeError).sizeBytes).toBeUndefined();
  });
});
