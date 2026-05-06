import { defineEventHandler, setResponseHeaders } from 'h3';
import { getMcpServerStatus } from '../../utils/mcp-server-manager';

/** GET /api/mcp/server — Returns 当前 MCP 服务器状态。 */
export default defineEventHandler((event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' });
  return getMcpServerStatus();
});
