import { describe, expect, it } from 'vitest';
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
});
