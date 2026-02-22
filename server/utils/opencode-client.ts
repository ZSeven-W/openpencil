/**
 * Shared OpenCode client manager.
 * Reuses an existing server on port 4096; starts one on a random port as fallback.
 * Tracks spawned servers so they can be cleaned up on process exit.
 */

const activeServers = new Set<{ close(): void }>()

// Clean up spawned OpenCode servers on process exit
function cleanup() {
  for (const server of activeServers) {
    try { server.close() } catch { /* ignore */ }
  }
  activeServers.clear()
}

process.on('beforeExit', cleanup)
process.on('SIGTERM', cleanup)
process.on('SIGINT', cleanup)

export async function getOpencodeClient() {
  const { createOpencodeClient, createOpencode } = await import('@opencode-ai/sdk/v2')

  // Try connecting to an existing server first
  try {
    const client = createOpencodeClient()
    await client.config.providers() // probe
    return { client, server: undefined }
  } catch {
    // No running server — start a temporary one on a random port
    const oc = await createOpencode({ port: 0 })
    activeServers.add(oc.server)
    return { client: oc.client, server: oc.server }
  }
}

export function releaseOpencodeServer(server: { close(): void } | undefined) {
  if (!server) return
  try { server.close() } catch { /* ignore */ }
  activeServers.delete(server)
}
