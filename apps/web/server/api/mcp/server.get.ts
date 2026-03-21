import { defineEventHandler, setResponseHeaders } from 'h3'
import { getMcpServerStatus } from '../../utils/mcp-server-manager'

/** GET /api/mcp/server — Returns the current MCP server status. */
export default defineEventHandler((event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })
  return getMcpServerStatus()
})
