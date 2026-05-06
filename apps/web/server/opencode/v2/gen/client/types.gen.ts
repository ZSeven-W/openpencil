// This 文件由 @hey-api/openapi-ts 自动生成

import type { Auth } from '../core/auth.gen.js';
import type {
  ServerSentEventsOptions,
  ServerSentEventsResult,
} from '../core/serverSentEvents.gen.js';
import type { Client as CoreClient, Config as CoreConfig } from '../core/types.gen.js';
import type { Middleware } from './utils.gen.js';

export type ResponseStyle = 'data' | 'fields';

export interface Config<T extends ClientOptions = ClientOptions>
  extends Omit<RequestInit, 'body' | 'headers' | 'method'>, CoreConfig {
  /**
   * Base URL 用于此客户端发出的所有请求。
   */
  baseUrl?: T['baseUrl'];
  /**
   * Fetch API 实现。 You 可以使用此选项来提供自定义
   * 获取实例。
   *
   * @default globalThis.fetch
   */
  fetch?: typeof fetch;
  /**
   * Please 不将
   * Fetch 客户端用于 Next.js 应用程序。 The `next` 选项不会有任何效果。 Install {@link
   *
   * https://www.npmjs.com/package/
   @hey-api/cli
   ent-next `@hey-api/client-next`} 代替。
   */
  next?: never;
  /**
   * Return 以指定格式解析的响应数据。默认 By，`auto`
   * 将从 `Content-Type` 响应标头推断出适当的方法。
   * You 可以使用任何 {@link Body} 方法覆盖此行为。
   * Select `stream` 如果您根本不想解析响应数据。
   *
   * @default “自动”
   */
  parseAs?: 'arrayBuffer' | 'auto' | 'blob' | 'formData' | 'json' | 'stream' | 'text';
  /**
   * Should 我们只返回数据还是多个字段（数据、错误、响应等）？
   *
   * @default “田野”
   */
  responseStyle?: ResponseStyle;
  /**
   * Throw 一个错误而不是在响应中返回它？
   *
   * @default 假的
   */
  throwOnError?: T['throwOnError'];
}

export interface RequestOptions<
  TData = unknown,
  TResponseStyle extends ResponseStyle = 'fields',
  ThrowOnError extends boolean = boolean,
  Url extends string = string,
>
  extends
    Config<{
      responseStyle: TResponseStyle;
      throwOnError: ThrowOnError;
    }>,
    Pick<
      ServerSentEventsOptions<TData>,
      | 'onSseError'
      | 'onSseEvent'
      | 'sseDefaultRetryDelay'
      | 'sseMaxRetryAttempts'
      | 'sseMaxRetryDelay'
    > {
  /**
   * 您要添加到请求中的 Any 正文。
   *
   * {@link https://developer.mozilla.org/docs/Web/API/fetch#body}
   */
  body?: unknown;
  path?: Record<string, unknown>;
  query?: Record<string, unknown>;
  /**
   * Security mechanism(s) 用于请求。
   */
  security?: ReadonlyArray<Auth>;
  url: Url;
}

export interface ResolvedRequestOptions<
  TResponseStyle extends ResponseStyle = 'fields',
  ThrowOnError extends boolean = boolean,
  Url extends string = string,
> extends RequestOptions<unknown, TResponseStyle, ThrowOnError, Url> {
  serializedBody?: string;
}

export type RequestResult<
  TData = unknown,
  TError = unknown,
  ThrowOnError extends boolean = boolean,
  TResponseStyle extends ResponseStyle = 'fields',
> = ThrowOnError extends true
  ? Promise<
      TResponseStyle extends 'data'
        ? TData extends Record<string, unknown>
          ? TData[keyof TData]
          : TData
        : {
            data: TData extends Record<string, unknown> ? TData[keyof TData] : TData;
            request: Request;
            response: Response;
          }
    >
  : Promise<
      TResponseStyle extends 'data'
        ? (TData extends Record<string, unknown> ? TData[keyof TData] : TData) | undefined
        : (
            | {
                data: TData extends Record<string, unknown> ? TData[keyof TData] : TData;
                error: undefined;
              }
            | {
                data: undefined;
                error: TError extends Record<string, unknown> ? TError[keyof TError] : TError;
              }
          ) & {
            request: Request;
            response: Response;
          }
    >;

export interface ClientOptions {
  baseUrl?: string;
  responseStyle?: ResponseStyle;
  throwOnError?: boolean;
}

type MethodFn = <
  TData = unknown,
  TError = unknown,
  ThrowOnError extends boolean = false,
  TResponseStyle extends ResponseStyle = 'fields',
>(
  options: Omit<RequestOptions<TData, TResponseStyle, ThrowOnError>, 'method'>,
) => RequestResult<TData, TError, ThrowOnError, TResponseStyle>;

type SseFn = <
  TData = unknown,
  TError = unknown,
  ThrowOnError extends boolean = false,
  TResponseStyle extends ResponseStyle = 'fields',
>(
  options: Omit<RequestOptions<TData, TResponseStyle, ThrowOnError>, 'method'>,
) => Promise<ServerSentEventsResult<TData, TError>>;

type RequestFn = <
  TData = unknown,
  TError = unknown,
  ThrowOnError extends boolean = false,
  TResponseStyle extends ResponseStyle = 'fields',
>(
  options: Omit<RequestOptions<TData, TResponseStyle, ThrowOnError>, 'method'> &
    Pick<Required<RequestOptions<TData, TResponseStyle, ThrowOnError>>, 'method'>,
) => RequestResult<TData, TError, ThrowOnError, TResponseStyle>;

type BuildUrlFn = <
  TData extends {
    body?: unknown;
    path?: Record<string, unknown>;
    query?: Record<string, unknown>;
    url: string;
  },
>(
  options: TData & Options<TData>,
) => string;

export type Client = CoreClient<RequestFn, Config, MethodFn, BuildUrlFn, SseFn> & {
  interceptors: Middleware<Request, Response, unknown, ResolvedRequestOptions>;
};

/**
 * The `createC
 * lientConfig()` 函数将在客户端初始化时调用，返回的对象将成为客户端的初始配置。 You 可能希望以这种方式初始化您的客户端，而不是调用
 *
 * `setConfig()`。例如，如果您使用 Next.js 来确保您的客户端始终具有正确的值，那么 This 很有用。
 *
 *
 */
export type CreateClientConfig<T extends ClientOptions = ClientOptions> = (
  override?: Config<ClientOptions & T>,
) => Config<Required<ClientOptions> & T>;

export interface TDataShape {
  body?: unknown;
  headers?: unknown;
  path?: unknown;
  query?: unknown;
  url: string;
}

type OmitKeys<T, K> = Pick<T, Exclude<keyof T, K>>;

export type Options<
  TData extends TDataShape = TDataShape,
  ThrowOnError extends boolean = boolean,
  TResponse = unknown,
  TResponseStyle extends ResponseStyle = 'fields',
> = OmitKeys<
  RequestOptions<TResponse, TResponseStyle, ThrowOnError>,
  'body' | 'path' | 'query' | 'url'
> &
  ([TData] extends [never] ? unknown : Omit<TData, 'url'>);
