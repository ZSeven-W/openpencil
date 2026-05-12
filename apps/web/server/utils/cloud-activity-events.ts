import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type { CloudActivityEventType } from '../../src/types/cloud';

export interface RecordCloudActivityInput {
  supabase: SupabaseClient;
  ownerId: string;
  actorId: string;
  type: CloudActivityEventType;
  fileId?: string | null;
  generationId?: string | null;
  metadata?: Record<string, unknown>;
}

export async function recordCloudActivity(input: RecordCloudActivityInput) {
  const inserted = await input.supabase.from('activity_events').insert({
    owner_id: input.ownerId,
    actor_id: input.actorId,
    file_id: input.fileId ?? null,
    generation_id: input.generationId ?? null,
    type: input.type,
    metadata: input.metadata ?? {},
  });

  if (inserted.error) {
    throw createError({ statusCode: 500, statusMessage: inserted.error.message });
  }
}
