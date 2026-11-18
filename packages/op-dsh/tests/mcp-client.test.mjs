import assert from 'node:assert/strict'
import { test } from 'node:test'

const mcp = await import('../lib/mcp-client.js')

test('parses successful OpenPencil MCP text JSON', () => {
  const result = mcp.parseOpenPencilMcpResponse('get_selection', {
    jsonrpc: '2.0', id: 1,
    result: {
      content: [{ type: 'text', text: '{"selectedIds":["n1"],"activePageId":"p1","nodes":[]}' }],
    },
  })
  assert.deepEqual(result.value, { selectedIds: ['n1'], activePageId: 'p1', nodes: [] })
  assert.deepEqual(result.images, [])
})

test('parses bounded MCP ImageContent and strips duplicate embedded base64 metadata', () => {
  const png = Buffer.from('89504e470d0a1a0a00000000', 'hex')
  const base64 = png.toString('base64')
  const result = mcp.parseOpenPencilMcpResponse('get_screenshot', {
    jsonrpc: '2.0', id: 1,
    result: {
      content: [
        { type: 'image', data: base64, mimeType: 'image/png' },
        { type: 'text', text: JSON.stringify({ nodeId: 'root', image_base64: base64 }) },
      ],
    },
  })
  assert.equal(result.images.length, 1)
  assert.equal(result.images[0].mimeType, 'image/png')
  assert.deepEqual(result.images[0].bytes, png)
  assert.deepEqual(result.value, { nodeId: 'root' })
  assert.equal(result.text.includes(base64), false)
})

test('call client redacts daemon credentials from successful values and errors', async () => {
  const token = 'sensitive-daemon-token-123456789'
  let successRequest
  const success = await mcp.callOpenPencilMcp({
    baseUrl: 'http://127.0.0.1:43123',
    token,
    tool: 'get_design_prompt',
    fetcher: async (_url, init) => {
      successRequest = init
      return Response.json({
        jsonrpc: '2.0', id: 1,
        result: { content: [{ type: 'text', text: JSON.stringify({ token, message: `echo ${token}` }) }] },
      })
    },
  })
  assert.equal(successRequest.headers.authorization, undefined)
  assert.equal(successRequest.headers['x-openpencil-token'], undefined)
  assert.deepEqual(success.value, { token: '[redacted]', message: 'echo [redacted]' })
  assert.equal(JSON.stringify(success).includes(token), false)

  await assert.rejects(mcp.callOpenPencilMcp({
    baseUrl: 'http://127.0.0.1:43123',
    token,
    tool: 'get_design_prompt',
    fetcher: async () => Response.json({
      jsonrpc: '2.0', id: 1,
      result: { isError: true, content: [{ type: 'text', text: `denied ${token}` }] },
    }),
  }), error => error instanceof Error && !error.message.includes(token) && error.message.includes('[redacted]'))
})

test('surfaces JSON-RPC, MCP isError, and transactional applied=false failures', () => {
  assert.throws(() => mcp.parseOpenPencilMcpResponse('x', {
    jsonrpc: '2.0', id: 1, error: { message: 'denied' },
  }), /denied/)
  assert.throws(() => mcp.parseOpenPencilMcpResponse('x', {
    jsonrpc: '2.0', id: 1, result: { isError: true, content: [{ type: 'text', text: 'bad patch' }] },
  }), /bad patch/)
  assert.throws(() => mcp.parseOpenPencilMcpResponse('batch_design', {
    jsonrpc: '2.0', id: 1,
    result: { content: [{ type: 'text', text: '{"applied":false,"errors":["line 2 failed"]}' }] },
  }), /line 2 failed/)
})

test('projects a bounded selection snapshot with id-only fallbacks', () => {
  const selection = mcp.selectionSnapshotFromMcp('/tmp/demo.op', {
    activePageId: 'p1', selectedIds: ['n1', '', 7],
    nodes: [{ id: 'n1', type: 'frame', name: 'Home', x: 1, y: 2, width: 375, height: 812 }, { nope: true }],
  }, 123)
  assert.deepEqual(selection, {
    sourcePath: '/tmp/demo.op', activePageId: 'p1', selectedIds: ['n1'], updatedAt: 123,
    nodes: [{ id: 'n1', type: 'frame', name: 'Home', x: 1, y: 2, width: 375, height: 812 }],
  })
})

test('version probes time out even when the daemon never responds', async () => {
  const fetcher = (_url, init) => new Promise((_resolve, reject) => {
    init.signal.addEventListener('abort', () => reject(init.signal.reason), { once: true })
  })
  await assert.rejects(mcp.getOpenPencilMcpVersion({
    baseUrl: 'http://127.0.0.1:43123',
    token: 'test-token',
    fetcher,
    timeoutMs: 10,
  }), error => error?.name === 'TimeoutError')
})

test('version probes do not send deployment credentials to the bundled daemon', async () => {
  let request
  const version = await mcp.getOpenPencilMcpVersion({
    baseUrl: 'http://127.0.0.1:43123',
    token: 'unused-single-tenant-token',
    fetcher: async (_url, init) => {
      request = init
      return Response.json({ version: 7 })
    },
  })
  assert.equal(version, 7)
  assert.equal(request.headers.authorization, undefined)
  assert.equal(request.headers['x-openpencil-token'], undefined)
})
