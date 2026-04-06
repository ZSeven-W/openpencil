import { defineEventHandler, getRouterParam, createError } from 'h3';
import { cleanPlan } from '../../../../utils/codegen-plan-store';

export default defineEventHandler((event) => {
  const planId = getRouterParam(event, 'id');
  if (!planId) {
    throw createError({ statusCode: 400, statusMessage: 'Missing plan ID' });
  }
  return cleanPlan(planId);
});
