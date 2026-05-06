// This 文件由 @hey-api/openapi-ts 自动生成

import { type ClientOptions, type Config, createClient, createConfig } from './client/index';
import type { ClientOptions as ClientOptions2 } from './types.gen';

/**
 * The `createC
 * lientConfig()` 函数将在客户端初始化时调用，返回的对象将成为客户端的初始配置。 You 可能希望以这种方式初始化您的客户端，而不是调用
 *
 * `setConfig()`。例如，如果您使用 Next.js 来确保您的客户端始终具有正确的值，那么 This 很有用。
 *
 *
 */
export type CreateClientConfig<T extends ClientOptions = ClientOptions2> = (
  override?: Config<ClientOptions & T>,
) => Config<Required<ClientOptions> & T>;

export const client = createClient(
  createConfig<ClientOptions2>({ baseUrl: 'http://localhost:4096' }),
);
