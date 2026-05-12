import { defineEventHandler, getQuery, getRouterParam, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { mapActivityEvent } from '../../../../utils/cloud-file-mappers';
import { assertCloudFileReadable } from '../../../../utils/cloud-file-management';
import type { CloudActivityEventType } from '../../../../../src/types/cloud';

const ACTIVITY_SELECT =
  'id,file_id,generation_id,actor_id,owner_id,type,metadata,created_at';

const ACTIVITY_EVENT_TYPES = [
  'file_created',
  'file_saved',
  'file_restored',
  'file_renamed',
  'file_moved',
  'file_shared',
  'file_share_revoked',
  'file_deleted',
  'file_restored_from_trash',
  'file_permanently_deleted',
  'codegen_created',
  'codegen_promoted',
  'codegen_deleted',
  'version_labeled',
  'file_force_saved',
] as const;

const activityQuerySchema = z.object({
  type: z.enum(['all', ...ACTIVITY_EVENT_TYPES]).optional(),
  cursor: z.string().datetime().optional(),
  limit: z.coerce.number().int().min(1).max(100).optional().default(20),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });
  const parsed = activityQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid activity query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const access = await assertCloudFileReadable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId: id,
  });

  let query = supabase
    .from('activity_events')
    .select(ACTIVITY_SELECT)
    .eq('file_id', id)
    .eq('owner_id', access.ownerId)
    .order('created_at', { ascending: false });

  if (parsed.data.type && parsed.data.type !== 'all') {
    query = query.eq('type', parsed.data.type as CloudActivityEventType);
  }
  if (parsed.data.cursor) {
    query = query.lt('created_at', parsed.data.cursor);
  }

  const events = await query.limit(parsed.data.limit + 1);

  if (events.error) {
    throw createError({ statusCode: 500, statusMessage: events.error.message });
  }

  const rows = events.data ?? [];
  const page = rows.slice(0, parsed.data.limit);
  const nextCursor =
    rows.length > parsed.data.limit ? page[page.length - 1]?.created_at ?? null : null;
  return {
    data: page.map(mapActivityEvent),
    meta: {
      nextCursor,
      limit: parsed.data.limit,
    },
  };
});
