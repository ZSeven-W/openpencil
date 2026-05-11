import { assertBodySize, type H3Event } from 'h3';
import {
  MAX_CLOUD_FILE_PAYLOAD_BYTES,
  createCloudFilePayloadTooLargeMessage,
} from '../../src/constants/cloud';
import { toApiError } from './cloud-supabase';

export async function assertCloudFileBodySize(event: H3Event): Promise<void> {
  try {
    await assertBodySize(event, MAX_CLOUD_FILE_PAYLOAD_BYTES);
  } catch {
    throw toApiError(
      413,
      'payload_too_large',
      createCloudFilePayloadTooLargeMessage(undefined, MAX_CLOUD_FILE_PAYLOAD_BYTES),
    );
  }
}
