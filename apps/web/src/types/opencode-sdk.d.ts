/**
 * @opencode-ai
 * /sdk 的 Minimal 类型声明。 The 包的导出映射指向不存在的路径（dist/ 与
 * dist/src/）。安装后脚本对此进行了修补，但我们保留声明作为后备。
 */

interface OpencodeClient {
  config: {
    providers(options?: unknown): Promise<{
      data: {
        providers: OpencodeProvider[];
        default: Record<string, string>;
      };
      error: unknown;
    }>;
  };
  session: {
    create(options?: { body?: { parentID?: string; title?: string } }): Promise<{
      data: OpencodeSession | undefined;
      error: unknown;
    }>;
    prompt(options: {
      path: { id: string };
      body: {
        model?: { providerID: string; modelID: string };
        noReply?: boolean;
        parts: Array<{ type: string; text: string }>;
      };
    }): Promise<{
      data:
        | {
            info: Record<string, unknown>;
            parts: Array<{ type: string; text?: string } & Record<string, unknown>>;
          }
        | undefined;
      error: unknown;
    }>;
  };
}

interface OpencodeProvider {
  id: string;
  name: string;
  models: Record<string, OpencodeModel>;
}

interface OpencodeModel {
  id: string;
  name: string;
  providerID: string;
}

interface OpencodeSession {
  id: string;
  title: string;
}

declare module '@opencode-ai/sdk' {
  export function createOpencode(options?: {
    hostname?: string;
    port?: number;
    signal?: AbortSignal;
    timeout?: number;
  }): Promise<{
    client: OpencodeClient;
    server: { url: string; close(): void };
  }>;

  export function createOpencodeClient(config?: {
    baseUrl?: string;
    directory?: string;
  }): OpencodeClient;
}

declare module '@opencode-ai/sdk/client' {
  export function createOpencodeClient(config?: {
    baseUrl?: string;
    directory?: string;
  }): OpencodeClient;
}
