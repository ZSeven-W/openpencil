import { defineEventHandler, getRouterParam, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../../utils/cloud-supabase';
import {
  SHARE_SELECT,
  assertOwnedCloudFile,
} from '../../../../../utils/cloud-file-management';
import { mapFileShare } from '../../../../../utils/cloud-file-mappers';
import { recordCloudActivity } from '../../../../../utils/cloud-activity-events';

const shareSchema = z.object({
  email: z.string().trim().email().optional(),
  userId: z.string().uuid().optional(),
  role: z.enum(['viewer', 'editor']).optional().default('viewer'),
}).superRefine((value, ctx) => {
  if (!value.email && !value.userId) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'email or userId is required',
    });
  }
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const parsed = shareSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid share payload', parsed.error.flatten());
  }

  await assertOwnedCloudFile({ supabase, userId: user.id, fileId: id });
  const email = parsed.data.email?.toLowerCase() ?? null;
  if (email && user.email && email === user.email.toLowerCase()) {
    throw toApiError(400, 'validation_error', 'Cannot share a file with yourself');
  }
  if (parsed.data.userId === user.id) {
    throw toApiError(400, 'validation_error', 'Cannot share a file with yourself');
  }

  const created = await supabase
    .from('design_file_shares')
    .insert({
      file_id: id,
      owner_id: user.id,
      shared_with_user_id: parsed.data.userId ?? null,
      shared_with_email: email,
      role: parsed.data.role,
    })
    .select(SHARE_SELECT)
    .single();
  if (created.error || !created.data) {
    throw createError({
      statusCode: 500,
      statusMessage: created.error?.message ?? 'Failed to share cloud file',
    });
  }

  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId: id,
    type: 'file_shared',
    metadata: {
      shareId: created.data.id,
      role: created.data.role,
      sharedWithUserId: created.data.shared_with_user_id,
      sharedWithEmail: created.data.shared_with_email,
    },
  });

  setResponseStatus(event, 201);
  return { data: mapFileShare(created.data) };
});
