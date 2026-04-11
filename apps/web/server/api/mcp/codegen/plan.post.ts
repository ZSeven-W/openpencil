import { defineEventHandler, readBody, createError } from 'h3';
import { getSyncDocument } from '../../../utils/mcp-sync-state';
import { createPlan } from '../../../utils/codegen-plan-store';
import { getActivePageChildren } from '@zseven-w/pen-core';
import type { PenDocument, CodePlanFromAI } from '@zseven-w/pen-types';
import { openDocument, LIVE_CANVAS_PATH } from '@zseven-w/pen-mcp';

interface PlanBody {
  plan: CodePlanFromAI;
  filePath?: string;
  pageId?: string;
}

async function resolveDocument(filePath?: string): Promise<PenDocument> {
  if (filePath && filePath !== LIVE_CANVAS_PATH) {
    return openDocument(filePath);
  }
  const sync = getSyncDocument();
  if (!sync.doc) {
    throw createError({ statusCode: 404, statusMessage: 'No document loaded in editor' });
  }
  return sync.doc;
}

export default defineEventHandler(async (event) => {
  const body = await readBody<PlanBody>(event);
  if (!body?.plan) {
    throw createError({ statusCode: 400, statusMessage: 'Missing plan in request body' });
  }

  const doc = await resolveDocument(body.filePath);
  const pageChildren = getActivePageChildren(doc, body.pageId ?? null);

  try {
    return createPlan(body.plan, pageChildren);
  } catch (err) {
    throw createError({
      statusCode: 400,
      statusMessage: err instanceof Error ? err.message : String(err),
    });
  }
});
