import { defineEventHandler, setResponseHeaders } from 'h3';
import { getSyncSelection } from '../../utils/mcp-sync-state';

/** GET /api/mcp/selection — Returns 供 MCP 读取的当前画布选择。 */
export default defineEventHandler((event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' });
  return getSyncSelection();
});
