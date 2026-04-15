export * from './gen/types.gen';

import { createClient } from './gen/client/client.gen';
import { type Config } from './gen/client/types.gen';
import { OpencodeClient } from './gen/sdk.gen';
export { type Config as OpencodeClientConfig, OpencodeClient };

export function createOpencodeClient(
  config?: Config & { directory?: string; experimental_workspaceID?: string },
) {
  if (!config?.fetch) {
    const customFetch: any = (req: any) => {
      // @ts-ignore
      req.timeout = false;
      return fetch(req);
    };
    config = {
      ...config,
      fetch: customFetch,
    };
  }

  if (config?.directory) {
    const isNonASCII = /\P{ASCII}/u.test(config.directory);
    const encodedDirectory = isNonASCII ? encodeURIComponent(config.directory) : config.directory;
    config.headers = {
      ...config.headers,
      'x-opencode-directory': encodedDirectory,
    };
  }

  if (config?.experimental_workspaceID) {
    config.headers = {
      ...config.headers,
      'x-opencode-workspace': config.experimental_workspaceID,
    };
  }

  const client = createClient(config);
  return new OpencodeClient({ client });
}
