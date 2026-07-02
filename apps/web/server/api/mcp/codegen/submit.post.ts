import { defineEventHandler, readBody, createError } from 'h3';
import { submitChunkResult } from '../../../utils/codegen-plan-store';
import type { ChunkResult } from '@zseven-w/pen-types';

interface SubmitBody {
  planId: string;
  result: ChunkResult;
  status?: 'failed' | 'skipped';
}

export default defineEventHandler(async (event) => {
  const body = await readBody<SubmitBody>(event);
  if (!body?.planId || !body?.result) {
    throw createError({ statusCode: 400, statusMessage: 'Missing planId or result' });
  }

  try {
    return submitChunkResult(body.planId, body.result, body.status);
  } catch (err) {
    throw createError({
      statusCode: 404,
      statusMessage: err instanceof Error ? err.message : String(err),
    });
  }
});
