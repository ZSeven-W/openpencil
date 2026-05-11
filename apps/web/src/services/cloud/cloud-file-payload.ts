import { MAX_CLOUD_FILE_PAYLOAD_BYTES } from '@/constants/cloud';

export class CloudFilePayloadTooLargeError extends Error {
  sizeBytes: number;
  maxBytes: number;

  constructor(sizeBytes: number, maxBytes = MAX_CLOUD_FILE_PAYLOAD_BYTES) {
    super('Cloud file payload is too large.');
    this.name = 'CloudFilePayloadTooLargeError';
    this.sizeBytes = sizeBytes;
    this.maxBytes = maxBytes;
  }
}

export function stringifyCloudFilePayload<T>(
  payload: T,
  maxBytes = MAX_CLOUD_FILE_PAYLOAD_BYTES,
): string {
  const body = JSON.stringify(payload);
  const sizeBytes = new TextEncoder().encode(body).byteLength;
  if (sizeBytes > maxBytes) {
    throw new CloudFilePayloadTooLargeError(sizeBytes, maxBytes);
  }
  return body;
}
