import { defineEventHandler, getRouterParam, getQuery, createError } from 'h3';
import { assemblePlan } from '../../../../utils/codegen-plan-store';
import type { Framework } from '@zseven-w/pen-types';

export default defineEventHandler((event) => {
  const planId = getRouterParam(event, 'id');
  const query = getQuery(event);
  const framework = (query.framework as Framework) || 'react';

  if (!planId) {
    throw createError({ statusCode: 400, statusMessage: 'Missing plan ID' });
  }

  try {
    return assemblePlan(planId, framework);
  } catch (err) {
    throw createError({
      statusCode: 404,
      statusMessage: err instanceof Error ? err.message : String(err),
    });
  }
});
