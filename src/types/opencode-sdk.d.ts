/**
 * Minimal type declarations for @opencode-ai/sdk.
 * The package's export map points to non-existent paths (dist/ vs dist/src/).
 * A postinstall script patches this, but we keep declarations as fallback.
 */

interface OpencodeClient {
  config: {
    providers(options?: unknown): Promise<{
      data: {
        providers: OpencodeProvider[]
        default: Record<string, string>
      }
      error: unknown
    }>
  }
  session: {
    create(options?: {
      body?: { parentID?: string; title?: string }
    }): Promise<{
      data: OpencodeSession | undefined
      error: unknown
    }>
    prompt(options: {
      path: { id: string }
      body: {
        model?: { providerID: string; modelID: string }
        noReply?: boolean
        parts: Array<{ type: string; text: string }>
      }
    }): Promise<{
      data:
        | {
            info: Record<string, unknown>
            parts: Array<{ type: string; text?: string } & Record<string, unknown>>
          }
        | undefined
      error: unknown
    }>
  }
}

interface OpencodeProvider {
  id: string
  name: string
  models: Record<string, OpencodeModel>
}

interface OpencodeModel {
  id: string
  name: string
  providerID: string
}

interface OpencodeSession {
  id: string
  title: string
}

declare module '@opencode-ai/sdk' {
  export function createOpencode(options?: {
    hostname?: string
    port?: number
    signal?: AbortSignal
    timeout?: number
  }): Promise<{
    client: OpencodeClient
    server: { url: string; close(): void }
  }>

  export function createOpencodeClient(config?: {
    baseUrl?: string
    directory?: string
  }): OpencodeClient
}

declare module '@opencode-ai/sdk/client' {
  export function createOpencodeClient(config?: {
    baseUrl?: string
    directory?: string
  }): OpencodeClient
}
