import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { startMcpHttpServer, stopMcpHttpServer } from '../../utils/mcp-server-manager'

const MCP_DEFAULT_PORT = 3100

interface PostBody {
  action: 'start' | 'stop'
  port?: number
}

/** POST /api/mcp/server — Start or stop the standalone MCP HTTP server. */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })
  const body = await readBody<PostBody>(event)

  if (!body?.action || !['start', 'stop'].includes(body.action)) {
    return { error: 'Invalid action. Use "start" or "stop".' }
  }

  if (body.action === 'start') {
    const port = body.port ?? MCP_DEFAULT_PORT
    return startMcpHttpServer(port)
  }

  return stopMcpHttpServer()
})
