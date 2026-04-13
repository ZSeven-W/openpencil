import { defineEventHandler, readBody, createError } from 'h3';
import { connectAcp, disconnectAcp } from '../../utils/acp-connection-manager';
import type { AcpAgentConfig } from '../../../src/types/agent-settings';

export default defineEventHandler(async (event) => {
  const body = await readBody<{
    action: 'connect' | 'disconnect';
    agentId: string;
    config?: AcpAgentConfig;
  }>(event);

  if (!body?.agentId || !body.action) {
    throw createError({ statusCode: 400, message: 'Missing: agentId, action' });
  }

  if (body.action === 'connect') {
    if (!body.config) {
      throw createError({ statusCode: 400, message: 'Missing: config (required for connect)' });
    }
    const result = await connectAcp(body.agentId, body.config);
    return result;
  }

  if (body.action === 'disconnect') {
    await disconnectAcp(body.agentId);
    return { connected: false };
  }

  throw createError({ statusCode: 400, message: `Unknown action: ${body.action}` });
});
