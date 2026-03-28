import { defineEventHandler, setResponseHeaders } from 'h3';
import { getSyncSelection } from '../../utils/mcp-sync-state';

/** GET /api/mcp/selection — Returns the current canvas selection for MCP to read. */
export default defineEventHandler((event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' });
  return getSyncSelection();
});
