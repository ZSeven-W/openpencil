export * from './client';
export * from './server';

import { createOpencodeClient } from './client';
import { createOpencodeServer } from './server';
import type { ServerOptions } from './server';

export async function createOpencode(options?: ServerOptions) {
  const server = await createOpencodeServer({
    ...options,
  });

  const client = createOpencodeClient({
    baseUrl: server.url,
  });

  return {
    client,
    server,
  };
}
